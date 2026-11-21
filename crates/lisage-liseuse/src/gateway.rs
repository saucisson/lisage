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
//! printing itself. Narration goes through the `Observer` channel
//! (ADR-0100 decision 6); the token is a secret and is never narrated.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use crate::debugger::event_kind_name;
use crate::trace::{ErreurDeserialisation, deserialiser_trace};
use lisage_core::NodeId;
use lisage_core::observer::{ChannelAttr, Observer, TraceEvent, TraceVal};

/// Loopback bind address — the gateway is never exposed on 0.0.0.0.
const LOOPBACK_ADDRESS: &str = "127.0.0.1:0";
/// Token size in bytes (16 bytes → 32 hex chars).
const TOKEN_BYTES: usize = 16;
/// Read buffer size for the initial HTTP request.
const READ_BUFFER_SIZE: usize = 4096;

/// Typed gateway errors; the cause chain is preserved.
#[derive(Debug)]
pub enum GatewayError {
    /// Underlying I/O failure (bind, accept, read, write).
    Io(std::io::Error),
    /// The trace file could not be deserialized (typed cause preserved).
    Trace(ErreurDeserialisation),
}

impl fmt::Display for GatewayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Connection token — the sole credential authenticating every HTTP and
/// WebSocket connection. A distinct type at the boundary (ADR-0006): the
/// token never crosses the public API as a bare `String`/`&str`. Its
/// `Debug` is redacted: a secret is never logged (ADR-0005).
#[derive(Clone, PartialEq, Eq)]
pub struct ConnectionToken(String);

impl fmt::Debug for ConnectionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConnectionToken(<redacted>)")
    }
}

impl ConnectionToken {
    /// Exact comparison against a candidate token from the query string.
    fn matches(&self, candidate: Option<&str>) -> bool {
        candidate == Some(self.0.as_str())
    }
}

impl fmt::Display for ConnectionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Announcement callback: called once with (port, token, event count) when
/// the gateway is bound.
pub type Announce = Box<dyn FnOnce(u16, &ConnectionToken, usize) + Send>;
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

/// Narrate one DEBUG event through the observer channel (ADR-0100 decision
/// 6). Named fields only; secrets (the token) are never narrated.
fn narrate(observer: &dyn Observer, message: String) {
    observer.on_event(&TraceEvent::Attr {
        channel: ChannelAttr::Synthesized,
        node: NodeId(0),
        value: TraceVal::texte(message),
        span: None,
    });
}

/// Generate a random token (16 bytes → 32 hex chars) from the OS
/// cryptographic source (`/dev/urandom`, no extra dependency). A failure to
/// read the entropy source is a typed `GatewayError::Io`, never a panic.
pub fn generate_token() -> Result<ConnectionToken, GatewayError> {
    use std::io::Read;
    let mut bytes = [0u8; TOKEN_BYTES];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(GatewayError::Io)?;
    Ok(ConnectionToken(
        bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
    ))
}

/// Extract one query parameter from the HTTP request line. Exact key match;
/// the returned value is compared for equality by the caller.
fn query_param(request: &str, name: &str) -> Option<String> {
    let request_line = request.lines().next()?;
    let target = request_line.split_whitespace().nth(1)?;
    let query = target.split_once('?')?.1;
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=')
            && key == name
        {
            return Some(value.to_string());
        }
    }
    None
}

/// Run the gateway: serve HTTP page + WebSocket on 127.0.0.1.
///
/// `trace_path` is the `.jsonl` file to replay. The bound port, token and
/// event count are reported through `events.announce`; narration goes
/// through `observer`. This future never resolves while the gateway is up.
pub async fn run_gateway(
    trace_path: &str,
    title: &str,
    observer: Arc<dyn Observer>,
    events: GatewayEvents,
) -> Result<(), GatewayError> {
    let trace_events = load_trace(trace_path)?;
    narrate(
        observer.as_ref(),
        format!(
            "gateway: trace loaded path={trace_path} events={}",
            trace_events.len()
        ),
    );
    let token = generate_token()?;
    let state = Arc::new(GatewayState::new(trace_events));
    let page = render_live_page(title, state.events.len());

    let listener = TcpListener::bind(LOOPBACK_ADDRESS).await?;
    let port = listener.local_addr()?.port();
    narrate(
        observer.as_ref(),
        format!("gateway: bound address=127.0.0.1 port={port}"),
    );
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
        let observer = Arc::clone(&observer);
        let connection_error = Arc::clone(&connection_error);
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, token, page, state, observer).await {
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
/// Requests whose `token` query parameter is not exactly the valid token
/// get a 403. The request line is read asynchronously; the rest of the
/// exchange runs on a blocking thread (tungstenite's sync API).
async fn handle_connection(
    mut stream: TcpStream,
    token: ConnectionToken,
    page: String,
    state: Arc<GatewayState>,
    observer: Arc<dyn Observer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buffer = vec![0u8; READ_BUFFER_SIZE];
    let count = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..count]).into_owned();
    if !token.matches(query_param(&request, "token").as_deref()) {
        narrate(
            observer.as_ref(),
            "gateway: connection rejected status=403".to_string(),
        );
        let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
    let is_websocket = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .map(|target| target.starts_with("/ws"))
        .unwrap_or(false);
    let std_stream = stream.into_std()?;
    let part = buffer[..count].to_vec();
    tokio::task::spawn_blocking(move || {
        serve_stream(std_stream, part, is_websocket, page, state, observer)
    })
    .await??;
    Ok(())
}

/// Serve one connection on a blocking thread: page for plain GET, WebSocket
/// session (sync tungstenite framing, no extra dependency) for `GET /ws`.
fn serve_stream(
    mut stream: std::net::TcpStream,
    part: Vec<u8>,
    is_websocket: bool,
    page: String,
    state: Arc<GatewayState>,
    observer: Arc<dyn Observer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Write;
    stream.set_nonblocking(false)?;
    if !is_websocket {
        narrate(
            observer.as_ref(),
            "gateway: page served status=200".to_string(),
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
            page.len(),
            page
        );
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }
    let mut socket = tokio_tungstenite::tungstenite::WebSocket::from_partially_read(
        stream,
        part,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    );
    narrate(
        observer.as_ref(),
        "gateway: websocket session open".to_string(),
    );
    loop {
        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Some(response) = handle_command(&text, &state, observer.as_ref())
                    && socket.write(Message::Text(response)).is_err()
                {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    narrate(
        observer.as_ref(),
        "gateway: websocket session closed".to_string(),
    );
    Ok(())
}

/// Parse a JSON command and produce a JSON response. The command name is
/// narrated (never the token, which commands do not carry).
pub fn handle_command(
    text: &str,
    state: &Arc<GatewayState>,
    observer: &dyn Observer,
) -> Option<String> {
    let parsed = crate::json::parse(text).ok()?;
    let command = parsed.get("cmd")?.as_str()?;
    narrate(observer, format!("gateway: command received cmd={command}"));
    let length = state.events.len();
    let response = match command {
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
    use lisage_core::observer::NoOpObserver;
    use std::sync::Mutex;

    #[test]
    fn test_generate_token_length() {
        let token = generate_token().expect("entropy source available");
        assert_eq!(token.to_string().len(), 32, "16 bytes → 32 hex chars");
        assert!(
            token
                .to_string()
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        );
    }

    #[test]
    fn test_generate_token_unique() {
        let first = generate_token().expect("entropy source available");
        let second = generate_token().expect("entropy source available");
        assert_ne!(first, second, "two tokens should differ");
    }

    #[test]
    fn test_query_param_exact() {
        let request = "GET /?token=abc123&x=1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        assert_eq!(query_param(request, "token").as_deref(), Some("abc123"));
        assert_eq!(query_param(request, "x").as_deref(), Some("1"));
        assert_eq!(query_param(request, "missing"), None);
        // A suffix extension must NOT match: exact value comparison only.
        let token = ConnectionToken("abc123".to_string());
        assert!(
            !token.matches(
                query_param(request, "token")
                    .map(|t| format!("{t}AAAA"))
                    .as_deref()
            )
        );
        assert!(token.matches(Some("abc123")));
        assert!(!token.matches(Some("abc123AAAA")));
        assert!(!token.matches(None));
    }

    #[test]
    fn test_gateway_state_step() {
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
        let response = handle_command(r#"{"cmd":"step"}"#, &state, &NoOpObserver).unwrap();
        assert!(response.contains(r#""event":"position""#));
        assert!(response.contains(r#""position":1"#));
    }

    #[test]
    fn test_handle_command_break() {
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
        let response =
            handle_command(r#"{"cmd":"break","round":12}"#, &state, &NoOpObserver).unwrap();
        assert!(response.contains(r#""event":"break_set""#));
        assert!(response.contains(r#""round":12"#));
        let response = handle_command(r#"{"cmd":"continue"}"#, &state, &NoOpObserver).unwrap();
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
        handle_command(r#"{"cmd":"goto","position":2}"#, &state, &NoOpObserver).unwrap();
        assert_eq!(state.position(), 2);
        let response = handle_command(r#"{"cmd":"back"}"#, &state, &NoOpObserver).unwrap();
        assert_eq!(state.position(), 1);
        assert!(response.contains(r#""position":1"#));
    }

    #[test]
    fn test_handle_command_unknown() {
        let state = Arc::new(GatewayState::new(vec![]));
        assert!(handle_command(r#"{"cmd":"fly"}"#, &state, &NoOpObserver).is_none());
    }

    /// Narration contract: the command narration event is pinned by a test
    /// (ADR-0005, checklist §4) — and never carries the token.
    #[test]
    fn test_handle_command_narration_pinned() {
        struct Collector {
            events: Mutex<Vec<TraceEvent>>,
        }
        impl Observer for Collector {
            fn on_event(&self, event: &TraceEvent) {
                self.events.lock().expect("mutex").push(event.clone());
            }
        }
        let collector = Arc::new(Collector {
            events: Mutex::new(Vec::new()),
        });
        let state = Arc::new(GatewayState::new(vec![TraceEvent::OpEnd { span: None }]));
        let _ = handle_command(r#"{"cmd":"step"}"#, &state, collector.as_ref()).unwrap();
        let events = collector.events.lock().expect("mutex").clone();
        assert!(
            events.iter().any(|event| matches!(
                event,
                TraceEvent::Attr { value, .. } if value.rendre() == "gateway: command received cmd=step"
            )),
            "the command narration event must be emitted: {events:?}"
        );
    }

    /// Invariants over the whole position range (deterministic sweep, no
    /// property crate): `goto` clamps into `[0, length]` and stepping to the
    /// end lands exactly on `length`.
    #[test]
    fn test_position_invariants_sweep() {
        let events: Vec<TraceEvent> = (0..10).map(|_| TraceEvent::OpEnd { span: None }).collect();
        let state = Arc::new(GatewayState::new(events));
        for target in 0..=12 {
            handle_command(
                &format!(r#"{{"cmd":"goto","position":{target}}}"#),
                &state,
                &NoOpObserver,
            )
            .unwrap();
            assert!(state.position() <= 10, "goto clamps to trace length");
        }
        handle_command(r#"{"cmd":"goto","position":0}"#, &state, &NoOpObserver).unwrap();
        while state.step().is_some() {}
        assert_eq!(state.position(), 10, "stepping to the end lands on length");
        assert!(state.step().is_none(), "no step past the end");
    }
}
