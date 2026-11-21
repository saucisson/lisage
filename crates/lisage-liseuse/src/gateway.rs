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

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::debugger::event_kind_name;
use lisage_core::observer::TraceEvent;

use crate::trace::deserialiser_trace;

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
        let new_pos = current + 1;
        self.position.store(new_pos, Ordering::SeqCst);
        let hit_break = self.break_round.load(Ordering::SeqCst) > 0
            && matches!(&self.events[current], TraceEvent::Round { numero, .. } if *numero as usize == self.break_round.load(Ordering::SeqCst));
        Some((new_pos, hit_break))
    }
}

/// Generate a random hex token (16 bytes → 32 hex chars).
pub fn generate_token() -> String {
    let mut bytes = [0u8; 16];
    // Simple xorshift PRNG seeded from system time — sufficient for a
    // loopback token (not cryptographic, the threat model is local).
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e3779b97f4a7c15);
    let mut state = seed | 1;
    for byte in &mut bytes {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state >> 32) as u8;
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Run the gateway: serve HTTP page + WebSocket on 127.0.0.1.
///
/// `trace_path` is the `.jsonl` file to replay. Returns the bound port.
pub async fn run_gateway(trace_path: &str, title: &str) -> std::io::Result<u16> {
    let events = load_trace(trace_path)?;
    let token = generate_token();
    let state = Arc::new(GatewayState::new(events));
    let page = render_live_page(title, state.events.len());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    eprintln!("lisage-gateway: http://127.0.0.1:{port}/?token={token}");
    eprintln!("lisage-gateway: ws://127.0.0.1:{port}/ws?token={token}");
    eprintln!(
        "lisage-gateway: {} events loaded from {trace_path}",
        state.events.len()
    );

    loop {
        let (stream, _addr) = listener.accept().await?;
        let token = token.clone();
        let page = page.clone();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, token, page, state).await {
                eprintln!("connection error: {e}");
            }
        });
    }
}

/// Load a trace from a `.jsonl` file.
fn load_trace(path: &str) -> std::io::Result<Vec<TraceEvent>> {
    let content = std::fs::read_to_string(path)?;
    deserialiser_trace(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}")))
}

/// Handle one TCP connection: HTTP GET → page, WebSocket upgrade → stream.
async fn handle_connection(
    mut stream: TcpStream,
    token: String,
    page: String,
    state: Arc<GatewayState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);
    // Check token in query string.
    if !request.contains(&format!("token={token}")) {
        let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
    if request.contains("GET /ws") {
        // WebSocket upgrade — handled by tungstenite below.
        let ws = tokio_tungstenite::accept_async(stream).await?;
        handle_websocket(ws, state).await?;
    } else {
        // Serve the HTML page.
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
    ws: tokio_tungstenite::WebSocketStream<TcpStream>,
    state: Arc<GatewayState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, mut rx) = mpsc::channel::<String>(64);
    // Spawn a task forwarding messages to the websocket.
    let (mut ws_tx, mut ws_rx) = ws.split();
    let forward = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg) = ws_rx.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg
            && let Some(response) = handle_command(&text, &state)
        {
            let _ = tx.send(response).await;
        }
    }
    forward.abort();
    Ok(())
}

/// Parse a JSON command and produce a JSON response.
pub fn handle_command(text: &str, state: &Arc<GatewayState>) -> Option<String> {
    let parsed = crate::json::parse(text).ok()?;
    let cmd = parsed.get("cmd")?.as_str()?;
    let response = match cmd {
        "step" | "next" => {
            state.paused.store(0, Ordering::SeqCst);
            match state.step() {
                Some((pos, hit_break)) => {
                    let event_json = state
                        .events
                        .get(pos.saturating_sub(1))
                        .map(|e| format!(r#","kind":"{}""#, event_kind_name(e)))
                        .unwrap_or_default();
                    if hit_break {
                        format!(
                            r#"{{"event":"break","position":{pos},"length":{L}}}{{"event":"position","position":{pos},"length":{L}}}"#,
                            L = state.events.len()
                        )
                    } else {
                        format!(
                            r#"{{"event":"event","position":{pos}{event_json}}}{{"event":"position","position":{pos},"length":{L}}}"#,
                            L = state.events.len()
                        )
                    }
                }
                None => r#"{"event":"done"}"#.to_string(),
            }
        }
        "back" => {
            let pos = state.position().saturating_sub(1);
            state.position.store(pos, Ordering::SeqCst);
            format!(
                r#"{{"event":"position","position":{pos},"length":{}}}"#,
                state.events.len()
            )
        }
        "goto" => {
            let target = parsed.get("position").and_then(|p| p.as_num()).unwrap_or(0);
            let pos = (target as usize).min(state.events.len());
            state.position.store(pos, Ordering::SeqCst);
            format!(
                r#"{{"event":"position","position":{pos},"length":{}}}"#,
                state.events.len()
            )
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
            // Replay until break or end.
            let mut last_pos = state.position();
            let mut hit = false;
            while let Some((pos, h)) = state.step() {
                last_pos = pos;
                if h {
                    hit = true;
                    break;
                }
            }
            if hit {
                format!(
                    r#"{{"event":"break","position":{last_pos},"length":{L}}}{{"event":"position","position":{last_pos},"length":{L}}}"#,
                    L = state.events.len()
                )
            } else {
                format!(
                    r#"{{"event":"done","position":{last_pos},"length":{L}}}"#,
                    L = state.events.len()
                )
            }
        }
        "status" => {
            let pos = state.position();
            format!(
                r#"{{"event":"status","position":{pos},"length":{},"paused":{},"break_round":{}}}"#,
                state.events.len(),
                state.paused.load(Ordering::SeqCst),
                state.break_round.load(Ordering::SeqCst)
            )
        }
        _ => return None,
    };
    Some(response)
}

/// Minimal inline HTML page for the live gateway (replaced by the full
/// `page.rs` renderer once D6-1 lands on daily).
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
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_token_unique() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b, "two tokens should differ");
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
        let (pos, hit) = state.step().unwrap();
        assert_eq!(pos, 1);
        assert!(!hit);
        // Set break at round 2, step → should hit.
        state.break_round.store(2, Ordering::SeqCst);
        let (pos, hit) = state.step().unwrap();
        assert_eq!(pos, 2);
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
        let resp = handle_command(r#"{"cmd":"step"}"#, &state).unwrap();
        assert!(resp.contains(r#""event":"position""#));
        assert!(resp.contains(r#""position":1"#));
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
        let resp = handle_command(r#"{"cmd":"break","round":12}"#, &state).unwrap();
        assert!(resp.contains(r#""event":"break_set""#));
        assert!(resp.contains(r#""round":12"#));
        // continue should hit the break.
        let resp = handle_command(r#"{"cmd":"continue"}"#, &state).unwrap();
        assert!(
            resp.contains(r#""event":"break""#),
            "continue hits round 12 break: {resp}"
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
        let resp = handle_command(r#"{"cmd":"back"}"#, &state).unwrap();
        assert_eq!(state.position(), 1);
        assert!(resp.contains(r#""position":1"#));
    }

    #[test]
    fn test_handle_command_unknown() {
        let state = Arc::new(GatewayState::new(vec![]));
        assert!(handle_command(r#"{"cmd":"fly"}"#, &state).is_none());
    }
}
