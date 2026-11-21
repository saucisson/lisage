//! D6-2 — Loopback WebSocket gateway: serves the offline page over HTTP
//! and streams a `.jsonl` trace over a WebSocket, both bound to `127.0.0.1`
//! only, with a random token required on every connection.
//!
//! Protocol (JSON text frames):
//! - client → server: `{"cmd":"step"}`, `{"cmd":"back"}`, `{"cmd":"goto","position":N}`,
//!   `{"cmd":"break","round":N}`, `{"cmd":"continue"}`, `{"cmd":"pause"}`
//! - server → client: `{"event":"position","position":N,"length":L}`,
//!   `{"event":"event","position":N,"kind":"OpEnd",...}`,
//!   `{"event":"break","round":N,"position":P}`, `{"event":"done"}`
//!
//! The library never writes to stdout/stderr: the entry point receives the
//! bound address and the token through [`GatewayEvents`] and does the
//! printing itself.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::debugger::event_kind_name;
use crate::trace::{ErreurDeserialisation, deserialiser_trace};
use lisage_core::observer::TraceEvent;

/// Loopback bind address — the gateway is never exposed on 0.0.0.0.
const LOOPBACK_ADDRESS: &str = "127.0.0.1:0";
/// Token size in bytes (16 bytes → 32 hex chars).
const TOKEN_BYTES: usize = 16;
/// Read buffer size for the initial HTTP request.
const READ_BUFFER_SIZE: usize = 4096;
/// Capacity of the channel forwarding responses to the WebSocket.
const CHANNEL_CAPACITY: usize = 64;

/// Typed gateway errors; the cause chain is preserved.
#[derive(Debug)]
pub enum GatewayError {
    /// Underlying I/O failure (bind, accept, read, write).
    Io(std::io::Error),
    /// The trace file could not be deserialized (typed cause preserved).
    Trace(ErreurDeserialisation),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayError::Io(error) => write!(formatter, "I/O error: {error}"),
            GatewayError::Trace(error) => write!(formatter, "trace error: {error}"),
        }
    }
}

impl std::error::Error for GatewayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GatewayError::Io(error) => Some(error),
            GatewayError::Trace(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for GatewayError {
    fn from(error: std::io::Error) -> Self {
        GatewayError::Io(error)
    }
}

/// Announcement callback: called once with (port, token, event count) when
/// the gateway is bound.
pub type Announce = Box<dyn FnOnce(u16, &str, usize) + Send>;
/// Connection-error callback: called with the error message of each failed
/// connection.
pub type ConnectionError = Box<dyn Fn(String) + Send + Sync>;

/// Side-channel events of the gateway, consumed by the entry point (the only
/// place allowed to write to stdout/stderr).
pub struct GatewayEvents {
    /// Called once with (port, token, event count) when the gateway is bound.
    pub announce: Announce,
    /// Called with the error message of each failed connection.
    pub connection_error: ConnectionError,
}

/// Shared gateway state: the trace, the current position, the break round.
pub struct GatewayState {
    /// All trace events.
    pub events: Vec<TraceEvent>,
    /// Current replay position.
    pub position: AtomicUsize,
    /// Break round (0 = none).
    pub break_round: AtomicUsize,
    /// Paused flag.
    pub paused: AtomicUsize,
}

impl GatewayState {
    /// New state from a trace.
    pub fn new(events: Vec<TraceEvent>) -> Self {
        GatewayState {
            events,
            position: AtomicUsize::new(0),
            break_round: AtomicUsize::new(0),
            paused: AtomicUsize::new(0),
        }
    }

    /// Current position.
    pub fn position(&self) -> usize {
        self.position.load(Ordering::SeqCst)
    }

    /// Advance one step; returns the new position and whether a break was hit.
    pub fn step(&self) -> Option<(usize, bool)> {
        let current = self.position.load(Ordering::SeqCst);
        if current >= self.events.len() {
            return None;
        }
        let new_position = current + 1;
        self.position.store(new_position, Ordering::SeqCst);
        let break_round = self.break_round.load(Ordering::SeqCst);
        let hit_break = break_round > 0
            && matches!(&self.events[current], TraceEvent::Round { numero, .. } if *numero as usize == break_round);
        Some((new_position, hit_break))
    }
}

/// Generate a random hex token (16 bytes → 32 hex chars).
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    // Simple xorshift PRNG seeded from system time — sufficient for a
    // loopback token (not cryptographic, the threat model is local).
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9e3779b97f4a7c15);
    let mut state = seed | 1;
    for byte in &mut bytes {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state >> 32) as u8;
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Run the gateway: serve HTTP page + WebSocket on 127.0.0.1.
///
/// `trace_path` is the `.jsonl` file to replay. The bound port, token and
/// event count are reported through `events.announce`. This future never
/// resolves while the gateway is up.
pub async fn run_gateway(
    trace_path: &str,
    title: &str,
    events: GatewayEvents,
) -> Result<(), GatewayError> {
    let trace_events = load_trace(trace_path)?;
    let token = generate_token();
    let state = Arc::new(GatewayState::new(trace_events));
    let page = render_live_page(title, state.events.len());

    let listener = TcpListener::bind(LOOPBACK_ADDRESS).await?;
    let port = listener.local_addr()?.port();
    let GatewayEvents {
        announce,
        connection_error,
    } = events;
    announce(port, &token, state.events.len());
    let connection_error: Arc<dyn Fn(String) + Send + Sync> = connection_error.into();

    loop {
        let (stream, _addr) = listener.accept().await?;
        let token = token.clone();
        let page = page.clone();
        let state = Arc::clone(&state);
        let connection_error = Arc::clone(&connection_error);
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, token, page, state).await {
                connection_error(error.to_string());
            }
        });
    }
}

/// Load a trace from a `.jsonl` file, preserving the typed error.
fn load_trace(path: &str) -> Result<Vec<TraceEvent>, GatewayError> {
    let content = std::fs::read_to_string(path)?;
    deserialiser_trace(&content).map_err(GatewayError::Trace)
}

/// Handle one TCP connection: HTTP GET → page, WebSocket upgrade → stream.
/// Requests without the valid `token` query parameter get a 403.
async fn handle_connection(
    mut stream: TcpStream,
    token: String,
    page: String,
    state: Arc<GatewayState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buffer = vec![0u8; READ_BUFFER_SIZE];
    let count = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..count]);
    if !request.contains(&format!("token={token}")) {
        let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
    if request.contains("GET /ws") {
        let socket = tokio_tungstenite::accept_async(stream).await?;
        handle_websocket(socket, state).await?;
    } else {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
            page.len(),
            page
        );
        stream.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

/// WebSocket session: replay the trace on demand, handle commands.
async fn handle_websocket(
    socket: tokio_tungstenite::WebSocketStream<TcpStream>,
    state: Arc<GatewayState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (sender, mut receiver) = mpsc::channel::<String>(CHANNEL_CAPACITY);
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let forward = tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if socket_sender.send(Message::Text(message)).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = socket_receiver.next().await {
        let message = message?;
        if let Message::Text(text) = message
            && let Some(response) = handle_command(&text, &state)
        {
            let _ = sender.send(response).await;
        }
    }
    forward.abort();
    Ok(())
}

/// Parse a JSON command and produce a JSON response.
pub fn handle_command(text: &str, state: &Arc<GatewayState>) -> Option<String> {
    let parsed = crate::json::parse(text).ok()?;
    let cmd = parsed.get("cmd")?.as_str()?;
    let length = state.events.len();
    let response = match cmd {
        "step" | "next" => {
            state.paused.store(0, Ordering::SeqCst);
            match state.step() {
                Some((position, hit_break)) => {
                    let event_json = state
                        .events
                        .get(position.saturating_sub(1))
                        .map(|event| format!(r#",\"kind\":\"{}\""#, event_kind_name(event)))
                        .unwrap_or_default();
                    if hit_break {
                        format!(
                            r#"{{"event":"break","position":{position},"length":{length}}}{{"event":"position","position":{position},"length":{length}}}"#
                        )
                    } else {
                        format!(
                            r#"{{"event":"event","position":{position}{event_json}}}{{"event":"position","position":{position},"length":{length}}}"#
                        )
                    }
                }
                None => r#"{"event":"done"}"#.to_string(),
            }
        }
        "back" => {
            let position = state.position().saturating_sub(1);
            state.position.store(position, Ordering::SeqCst);
            format!(r#"{{"event":"position","position":{position},"length":{length}}}"#)
        }
        "goto" => {
            let target = parsed.get("position").and_then(|p| p.as_num()).unwrap_or(0);
            let position = (target as usize).min(length);
            state.position.store(position, Ordering::SeqCst);
            format!(r#"{{"event":"position","position":{position},"length":{length}}}"#)
        }
        "break" => {
            let round = parsed.get("round").and_then(|r| r.as_num()).unwrap_or(0);
            state.break_round.store(round as usize, Ordering::SeqCst);
            format!(r#"{{"event":"break_set","round":{round}}}"#)
        }
        "pause" => {
            state.paused.store(1, Ordering::SeqCst);
            r#"{"event":"paused"}"#.to_string()
        }
        "continue" | "resume" => {
            state.paused.store(0, Ordering::SeqCst);
            let mut last_position = state.position();
            let mut hit = false;
            while let Some((position, hit_break)) = state.step() {
                last_position = position;
                if hit_break {
                    hit = true;
                    break;
                }
            }
            if hit {
                format!(
                    r#"{{"event":"break","position":{last_position},"length":{length}}}{{"event":"position","position":{last_position},"length":{length}}}"#
                )
            } else {
                format!(r#"{{"event":"done","position":{last_position},"length":{length}}}"#)
            }
        }
        "status" => {
            let position = state.position();
            format!(
                r#"{{"event":"status","position":{position},"length":{length},"paused":{},"break_round":{}}}"#,
                state.paused.load(Ordering::SeqCst),
                state.break_round.load(Ordering::SeqCst)
            )
        }
        _ => return None,
    };
    Some(response)
}

/// Minimal inline HTML page for the live gateway (static navigation is
/// provided by the D6-1 `page.rs` renderer; this live page adds the
/// WebSocket hookup carrying the token).
pub fn render_live_page(title: &str, event_count: usize) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>{title}</title>
<style>
body {{ font-family: monospace; margin: 1em; background: #1e1e1e; color: #d4d4d4; }}
button {{ background: #0e639c; color: white; border: none; padding: 4px 12px; margin: 2px; cursor: pointer; }}
#status {{ color: #9cdcfe; }}
#log {{ background: #252526; padding: 0.5em; max-height: 20em; overflow-y: auto; white-space: pre-wrap; }}
</style></head>
<body>
<h1>{title}</h1>
<div>{event_count} events</div>
<div>
  <button onclick="send('step')">step</button>
  <button onclick="send('back')">back</button>
  <button onclick="send('pause')">pause</button>
  <button onclick="send('continue')">continue</button>
  <button onclick="sendBreak()">break round 12</button>
  <button onclick="send('status')">status</button>
</div>
<div id="status">connecting…</div>
<div id="log"></div>
<script>
const TOKEN = new URLSearchParams(location.search).get('token');
const ws = new WebSocket('ws://' + location.host + '/ws?token=' + TOKEN);
ws.onopen = () => document.getElementById('status').textContent = 'connected';
ws.onmessage = (e) => {{
  const log = document.getElementById('log');
  log.textContent += e.data + '\n';
  log.scrollTop = log.scrollHeight;
}};
ws.onclose = () => document.getElementById('status').textContent = 'disconnected';
function send(cmd) {{ ws.send(JSON.stringify({{cmd: cmd}})); }}
function sendBreak() {{ ws.send(JSON.stringify({{cmd: 'break', round: 12}})); }}
</script>
</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 32, "16 bytes → 32 hex chars");
        assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_token_unique() {
        let first = generate_token();
        let second = generate_token();
        assert_ne!(first, second, "two tokens should differ");
    }

    #[test]
    fn test_gateway_state_step() {
        use lisage_core::NodeId;
        let events = vec![
            TraceEvent::Round {
                numero: 1,
                niveaux_actifs: vec![0],
                niveaux_modifies: vec![0],
                avant: NodeId(0),
                image: NodeId(1),
                apres: NodeId(1),
                croissance: 1,
                span: None,
            },
            TraceEvent::Round {
                numero: 2,
                niveaux_actifs: vec![0],
                niveaux_modifies: vec![],
                avant: NodeId(1),
                image: NodeId(1),
                apres: NodeId(1),
                croissance: 0,
                span: None,
            },
        ];
        let state = GatewayState::new(events);
        assert_eq!(state.position(), 0);
        let (position, hit) = state.step().unwrap();
        assert_eq!(position, 1);
        assert!(!hit);
        state.break_round.store(2, Ordering::SeqCst);
        let (position, hit) = state.step().unwrap();
        assert_eq!(position, 2);
        assert!(hit, "break at round 2 should trigger");
        assert!(state.step().is_none(), "end of trace");
    }

    #[test]
    fn test_handle_command_step() {
        let events = vec![
            TraceEvent::OpEnd { span: None },
            TraceEvent::OpEnd { span: None },
        ];
        let state = Arc::new(GatewayState::new(events));
        let response = handle_command(r#"{"cmd":"step"}"#, &state).unwrap();
        assert!(response.contains(r#""event":"position""#));
        assert!(response.contains(r#""position":1"#));
    }

    #[test]
    fn test_handle_command_break() {
        use lisage_core::NodeId;
        let events = vec![TraceEvent::Round {
            numero: 12,
            niveaux_actifs: vec![],
            niveaux_modifies: vec![],
            avant: NodeId(0),
            image: NodeId(0),
            apres: NodeId(0),
            croissance: 0,
            span: None,
        }];
        let state = Arc::new(GatewayState::new(events));
        let response = handle_command(r#"{"cmd":"break","round":12}"#, &state).unwrap();
        assert!(response.contains(r#""event":"break_set""#));
        assert!(response.contains(r#""round":12"#));
        let response = handle_command(r#"{"cmd":"continue"}"#, &state).unwrap();
        assert!(
            response.contains(r#""event":"break""#),
            "continue hits round 12 break: {response}"
        );
    }

    #[test]
    fn test_handle_command_goto_and_back() {
        let events = vec![
            TraceEvent::OpEnd { span: None },
            TraceEvent::OpEnd { span: None },
            TraceEvent::OpEnd { span: None },
        ];
        let state = Arc::new(GatewayState::new(events));
        handle_command(r#"{"cmd":"goto","position":2}"#, &state).unwrap();
        assert_eq!(state.position(), 2);
        let response = handle_command(r#"{"cmd":"back"}"#, &state).unwrap();
        assert_eq!(state.position(), 1);
        assert!(response.contains(r#""position":1"#));
    }

    #[test]
    fn test_handle_command_unknown() {
        let state = Arc::new(GatewayState::new(vec![]));
        assert!(handle_command(r#"{"cmd":"fly"}"#, &state).is_none());
    }
}
