//! D6-2 — Loopback WebSocket gateway: serves the live page over HTTP and
//! replays a `.jsonl` trace over a WebSocket, bound to `127.0.0.1` only,
//! with a random token required on every connection. Protocol contract:
//! `docs/gateway-protocol.md` (ADR-0016). The library never writes to
//! stdout/stderr (the entry point does, via [`GatewayEvents`]); narration
//! goes through the `Observer` channel (ADR-0100 decision 6); the token is
//! a secret and is never narrated.

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
/// Cap on the HTTP request head read from a connection.
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
    /// Exact equality check against a candidate token value (header or first WS frame).
    fn matches(&self, candidate: Option<&str>) -> bool {
        candidate == Some(self.0.as_str())
    }
}

impl fmt::Display for ConnectionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl ConnectionToken {
    /// Raw token value — for the entry point printing and the authenticated
    /// page rendering ONLY. Never log it (ADR-0005); `Display`/`Debug` are
    /// redacted.
    pub fn as_secret_str(&self) -> &str {
        &self.0
    }

    /// Build a token from a hex string — tests only.
    #[doc(hidden)]
    pub fn from_hex_for_test(hex: &str) -> Self {
        ConnectionToken(hex.to_string())
    }
}

/// Called once with (port, token, event count) when the gateway is bound.
pub type Announce = Box<dyn FnOnce(u16, &ConnectionToken, usize) + Send>;
/// Called with the error message of each failed connection.
pub type ConnectionError = Box<dyn Fn(String) + Send + Sync>;

/// Side-channel events consumed by the entry point (the only place allowed
/// to write to stdout/stderr).
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

fn bearer_matches(token: &ConnectionToken, authorization: Option<&str>) -> bool {
    authorization == Some(format!("Bearer {}", token.as_secret_str()).as_str())
}

/// Run the gateway on 127.0.0.1: port/token/event count via `events.announce`,
/// narration via `observer`; never resolves while the gateway is up.
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
    let page = render_live_page(title, state.events.len(), &token);

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

fn load_trace(path: &str) -> Result<Vec<TraceEvent>, GatewayError> {
    let content = std::fs::read_to_string(path)?;
    deserialiser_trace(&content).map_err(GatewayError::Trace)
}

/// Handle one TCP connection on a blocking thread (sync tungstenite, no
/// extra dependency): plain GET → page (bearer required, else 403),
/// `GET /ws` → WebSocket upgrade (credential in the first frame).
async fn handle_connection(
    stream: TcpStream,
    token: ConnectionToken,
    page: String,
    state: Arc<GatewayState>,
    observer: Arc<dyn Observer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let std_stream = stream.into_std()?;
    tokio::task::spawn_blocking(move || serve_stream(std_stream, token, page, state, observer))
        .await??;
    Ok(())
}

/// Read the HTTP request head (up to the blank line) from a blocking stream.
fn read_request_head(stream: &mut std::net::TcpStream) -> std::io::Result<String> {
    use std::io::Read;
    let mut bytes = Vec::with_capacity(READ_BUFFER_SIZE);
    let mut byte = [0u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => bytes.push(byte[0]),
            Err(error) => return Err(error),
        }
        if bytes.len() > READ_BUFFER_SIZE {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Extract one header value from the request head (case-insensitive name).
fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    for line in request.lines().skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case(name)
        {
            return Some(value.trim());
        }
    }
    None
}

/// Serve one connection on a blocking thread.
fn serve_stream(
    mut stream: std::net::TcpStream,
    token: ConnectionToken,
    page: String,
    state: Arc<GatewayState>,
    observer: Arc<dyn Observer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Write;
    stream.set_nonblocking(false)?;
    let request = read_request_head(&mut stream)?;
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    if !target.starts_with("/ws") {
        // Plain HTTP: the page requires `Authorization: Bearer <token>`
        // (ADR-0016: no secret in URLs), otherwise 403.
        if !bearer_matches(&token, header_value(&request, "authorization")) {
            narrate(
                observer.as_ref(),
                "gateway: connection rejected status=403".to_string(),
            );
            stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")?;
            return Ok(());
        }
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
    // WebSocket upgrade at /ws: no credential in the URL. Complete the
    // 101 handshake, then authenticate on the first text frame.
    let Some(key) = header_value(&request, "sec-websocket-key") else {
        stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")?;
        return Ok(());
    };
    let accept = tokio_tungstenite::tungstenite::handshake::derive_accept_key(key.as_bytes());
    let handshake = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream.write_all(handshake.as_bytes())?;
    let socket = tokio_tungstenite::tungstenite::WebSocket::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    );
    websocket_session(socket, token, state, observer)
}

fn websocket_session(
    mut socket: tokio_tungstenite::tungstenite::WebSocket<std::net::TcpStream>,
    token: ConnectionToken,
    state: Arc<GatewayState>,
    observer: Arc<dyn Observer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    narrate(
        observer.as_ref(),
        "gateway: websocket session open".to_string(),
    );
    if !authenticate_frame(&mut socket, &token, observer.as_ref())? {
        return Ok(());
    }
    while let Ok(message) = socket.read() {
        let Message::Text(text) = message else {
            if matches!(message, Message::Close(_)) {
                break;
            }
            continue;
        };
        if let Some(response) = handle_command(&text, &state, observer.as_ref())
            && socket.send(Message::Text(response)).is_err()
        {
            break;
        }
    }
    narrate(
        observer.as_ref(),
        "gateway: websocket session closed".to_string(),
    );
    Ok(())
}

fn authenticate_frame(
    socket: &mut tokio_tungstenite::tungstenite::WebSocket<std::net::TcpStream>,
    token: &ConnectionToken,
    observer: &dyn Observer,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let Ok(Message::Text(text)) = socket.read() else {
        return Ok(false);
    };
    let parsed = crate::json::parse(&text).ok();
    let is_auth_frame = parsed
        .as_ref()
        .and_then(|value| value.get("cmd"))
        .and_then(|command| command.as_str())
        == Some("auth");
    let candidate = parsed
        .as_ref()
        .and_then(|value| value.get("token"))
        .and_then(|value| value.as_str());
    let authenticated = is_auth_frame && token.matches(candidate);
    let reply = if authenticated {
        r#"{"event":"auth_ok"}"#.to_string()
    } else {
        narrate(observer, "gateway: websocket auth failed".to_string());
        r#"{"event":"auth_failed"}"#.to_string()
    };
    socket.send(Message::Text(reply))?;
    Ok(authenticated)
}

/// Parse a JSON command and produce a JSON response: an ARRAY of event
/// objects (one JSON value per replay event, so every frame is strict JSON).
/// The command name is narrated (never the token, which commands do not
/// carry).
pub fn handle_command(
    text: &str,
    state: &Arc<GatewayState>,
    observer: &dyn Observer,
) -> Option<String> {
    let parsed = crate::json::parse(text).ok()?;
    let command = parsed.get("cmd")?.as_str()?;
    narrate(observer, format!("gateway: command received cmd={command}"));
    let events = match command {
        "step" | "next" => command_step(state),
        "back" => vec![command_back(state)],
        "goto" => vec![command_goto(&parsed, state)],
        "break" => vec![command_break(&parsed, state)],
        "pause" => command_pause(state),
        "continue" | "resume" => command_continue(state),
        "status" => vec![command_status(state)],
        _ => return None,
    };
    Some(format!("[{}]", events.join(",")))
}

/// `step`: advance one event; emits `event`/`break` + `position`, or `done`.
fn command_step(state: &Arc<GatewayState>) -> Vec<String> {
    state.paused.store(0, Ordering::SeqCst);
    let length = state.events.len();
    let Some((position, hit_break)) = state.step() else {
        return vec![r#"{"event":"done"}"#.to_string()];
    };
    let kind = state
        .events
        .get(position.saturating_sub(1))
        .map(|event| format!(r#""kind":"{}""#, event_kind_name(event)))
        .unwrap_or_default();
    let position_event =
        format!(r#"{{"event":"position","position":{position},"length":{length}}}"#);
    if hit_break {
        vec![
            format!(r#"{{"event":"break","position":{position},"length":{length}}}"#),
            position_event,
        ]
    } else {
        vec![
            format!(r#"{{"event":"event","position":{position},{kind}}}"#),
            position_event,
        ]
    }
}

/// `back`: one step backward (clamped at 0).
fn command_back(state: &Arc<GatewayState>) -> String {
    let position = state.position().saturating_sub(1);
    state.position.store(position, Ordering::SeqCst);
    format!(
        r#"{{"event":"position","position":{position},"length":{}}}"#,
        state.events.len()
    )
}

/// `goto`: jump, clamped to `[0, length]`.
fn command_goto(parsed: &crate::json::Json, state: &Arc<GatewayState>) -> String {
    let target = parsed.get("position").and_then(|p| p.as_num()).unwrap_or(0);
    let position = (target.max(0) as usize).min(state.events.len());
    state.position.store(position, Ordering::SeqCst);
    format!(
        r#"{{"event":"position","position":{position},"length":{}}}"#,
        state.events.len()
    )
}

/// `break`: set the break round.
fn command_break(parsed: &crate::json::Json, state: &Arc<GatewayState>) -> String {
    let round = parsed.get("round").and_then(|r| r.as_num()).unwrap_or(0);
    state
        .break_round
        .store(round.max(0) as usize, Ordering::SeqCst);
    format!(r#"{{"event":"break_set","round":{round}}}"#)
}

/// `pause`.
fn command_pause(state: &Arc<GatewayState>) -> Vec<String> {
    state.paused.store(1, Ordering::SeqCst);
    vec![r#"{"event":"paused"}"#.to_string()]
}

/// `continue`: run until the break round or the end of the trace.
fn command_continue(state: &Arc<GatewayState>) -> Vec<String> {
    state.paused.store(0, Ordering::SeqCst);
    let length = state.events.len();
    let mut last_position = state.position();
    let mut hit = false;
    while let Some((position, hit_break)) = state.step() {
        last_position = position;
        if hit_break {
            hit = true;
            break;
        }
    }
    let position_event =
        format!(r#"{{"event":"position","position":{last_position},"length":{length}}}"#);
    if hit {
        vec![
            format!(r#"{{"event":"break","position":{last_position},"length":{length}}}"#),
            position_event,
        ]
    } else {
        vec![format!(
            r#"{{"event":"done","position":{last_position},"length":{length}}}"#
        )]
    }
}

/// `status`: current position, length, paused flag, break round.
fn command_status(state: &Arc<GatewayState>) -> String {
    format!(
        r#"{{"event":"status","position":{},"length":{},"paused":{},"break_round":{}}}"#,
        state.position(),
        state.events.len(),
        state.paused.load(Ordering::SeqCst),
        state.break_round.load(Ordering::SeqCst)
    )
}

/// Minimal inline HTML page for the live gateway (static navigation is
/// provided by the D6-1 `page.rs` renderer; this live page adds the
/// WebSocket hookup carrying the token).
pub fn render_live_page(title: &str, event_count: usize, token: &ConnectionToken) -> String {
    // The token is 32 hex chars: JSON-safe without escaping.
    let token_json = format!(r#""{}""#, token.as_secret_str());
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>{title}</title>
<style>
body {{ font-family: monospace; margin: 1em; background: #1e1e1e; color: #d4d4d4; }}
button {{ background: #0e639c; color: white; border: none; padding: 4px 12px; margin: 2px; cursor: pointer; }}
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
// The page is served only after `Authorization: Bearer` authentication, so
// embedding the token here exposes it to the authenticated holder only. The
// WebSocket URL itself carries no secret: auth is the first frame.
const TOKEN = {token_json};
const ws = new WebSocket('ws://' + location.host + '/ws');
ws.onopen = () => ws.send(JSON.stringify({{cmd: 'auth', token: TOKEN}}));
ws.onmessage = (e) => {{
  const log = document.getElementById('log');
  for (const event of JSON.parse(e.data)) log.textContent += JSON.stringify(event) + '\n';
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
#[rustfmt::skip]
mod tests {
    use super::*;
    use lisage_core::observer::NoOpObserver;
    use std::sync::Mutex;

    fn round(number: u32) -> TraceEvent {
        TraceEvent::Round { numero: number, niveaux_actifs: vec![0], niveaux_modifies: vec![], avant: NodeId(0), image: NodeId(0), apres: NodeId(0), croissance: 0, span: None }
    }

    #[test]
    fn test_generate_token_length() {
        let token = generate_token().expect("entropy source available");
        assert_eq!(token.as_secret_str().len(), 32, "16 bytes → 32 hex chars");
        assert!(token.as_secret_str().chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(format!("{token}"), "<redacted>");
        assert_eq!(format!("{token:?}"), "ConnectionToken(<redacted>)");
    }

    #[test]
    fn test_generate_token_unique() {
        assert_ne!(generate_token().unwrap(), generate_token().unwrap());
    }

    #[test]
    fn test_bearer_check_exact() {
        let token = ConnectionToken("abc123".to_string());
        assert!(bearer_matches(&token, Some("Bearer abc123")));
        assert!(!bearer_matches(&token, Some("Bearer abc123AAAA")));
        assert!(!bearer_matches(&token, None));
        assert!(!bearer_matches(&token, Some("Basic abc123")));
        assert!(!bearer_matches(&token, Some("bearer abc123")));
    }

    #[test]
    fn test_gateway_state_step() {
        let state = GatewayState::new(vec![round(1), round(2)]);
        assert_eq!(state.position(), 0);
        let (position, hit) = state.step().unwrap();
        assert_eq!((position, hit), (1, false));
        state.break_round.store(2, Ordering::SeqCst);
        let (position, hit) = state.step().unwrap();
        assert_eq!((position, hit), (2, true), "break at round 2 should trigger");
        assert!(state.step().is_none(), "end of trace");
    }





    /// Narration contract pinned (ADR-0005, checklist §4); never the token.
    #[test]
    fn test_handle_command_narration_pinned() {
        struct Collector { events: Mutex<Vec<TraceEvent>> }
        impl Observer for Collector {
            fn on_event(&self, event: &TraceEvent) { self.events.lock().expect("mutex").push(event.clone()); }
        }
        let collector = Arc::new(Collector { events: Mutex::new(Vec::new()) });
        let state = Arc::new(GatewayState::new(vec![TraceEvent::OpEnd { span: None }]));
        let _ = handle_command(r#"{"cmd":"step"}"#, &state, collector.as_ref()).unwrap();
        let events = collector.events.lock().expect("mutex").clone();
        assert!(events.iter().any(|e| matches!(e, TraceEvent::Attr { value, .. } if value.rendre() == "gateway: command received cmd=step")), "narration event must be emitted: {events:?}");
    }

    /// Position invariants over the whole range: `goto` clamps into
    /// `[0, length]`, stepping to the end lands exactly on `length`.
    #[test]
    fn test_position_invariants_sweep() {
        let state = Arc::new(GatewayState::new(vec![TraceEvent::OpEnd { span: None }; 10]));
        for target in 0..=12 {
            handle_command(&format!(r#"{{"cmd":"goto","position":{target}}}"#), &state, &NoOpObserver).unwrap();
            assert!(state.position() <= 10, "goto clamps to trace length");
        }
        handle_command(r#"{"cmd":"goto","position":0}"#, &state, &NoOpObserver).unwrap();
        while state.step().is_some() {}
        assert_eq!(state.position(), 10, "stepping to the end lands on length");
        assert!(state.step().is_none(), "no step past the end");
    }
}
