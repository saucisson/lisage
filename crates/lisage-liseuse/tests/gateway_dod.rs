//! D6-2 DoD: the live page drives pause/step/break of an attached program
//! (here a `.jsonl` trace file replayed by the gateway).

use std::sync::Arc;
use std::sync::atomic::Ordering;

use lisage_core::NodeId;
use lisage_core::observer::{NoOpObserver, TraceEvent};
use lisage_liseuse::gateway::{
    ConnectionToken, GatewayState, generate_token, handle_command, render_live_page,
};

fn sample_trace() -> Vec<TraceEvent> {
    (1..=15)
        .map(|i| TraceEvent::Round {
            number: i,
            active_levels: vec![0],
            modified_levels: if i < 15 { vec![0] } else { vec![] },
            before: NodeId(i),
            image: NodeId(i),
            after: NodeId(i),
            growth: if i < 15 { 1 } else { 0 },
            span: None,
        })
        .collect()
}

/// DoD: break at round 12 is hit by `continue`, not at the end.
#[test]
#[rustfmt::skip]
fn test_d6_2_break_round_12_caught() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    let response = handle_command(r#"{"cmd":"break","round":12}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""event":"break_set""#));
    let response = handle_command(r#"{"cmd":"continue"}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""event":"break""#), "continue must hit the round-12 break: {response}");
    assert!(response.contains(r#""position":12"#), "break after round 12 (position 12)");
    assert_eq!(state.position(), 12);
}

/// DoD: step/pause/back/status commands work over the JSON protocol.
#[test]
#[rustfmt::skip]
fn test_d6_2_step_pause_back_status() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    let response = handle_command(r#"{"cmd":"step"}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""position":1"#));
    let response = handle_command(r#"{"cmd":"pause"}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""event":"paused""#));
    assert_eq!(state.paused.load(Ordering::SeqCst), 1);
    let response = handle_command(r#"{"cmd":"back"}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""position":0"#));
    let response = handle_command(r#"{"cmd":"status"}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""event":"status""#));
    assert!(response.contains(r#""length":15"#));
    assert!(response.contains(r#""paused":1"#));
}

/// The live page is self-contained and connects with the token.
#[test]
#[rustfmt::skip]
fn test_d6_2_live_page_self_contained() {
    let token = ConnectionToken::from_hex_for_test("abc123");
    let html = render_live_page("test", 42, &token);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("42 events"));
    assert!(html.contains("auth"), "page authenticates the WebSocket as first frame");
    assert!(!html.contains("?token="), "no secret in the WebSocket URL");
    assert!(html.contains("WebSocket"), "page opens a WebSocket");
    assert!(html.contains("step"), "step button");
    assert!(html.contains("pause"), "pause button");
    assert!(html.contains("break round 12"), "break button");
    assert!(!html.contains(r#"src="http"#));
    assert!(!html.contains(r#"href="http"#));
}

/// Token generation: 32 hex chars, unique across calls.
#[test]
#[rustfmt::skip]
fn test_d6_2_token() {
    let t1 = generate_token().expect("entropy source available");
    let t2 = generate_token().expect("entropy source available");
    let t1 = t1.as_secret_str().to_string();
    let t2 = t2.as_secret_str().to_string();
    assert_eq!(t1.len(), 32);
    assert_ne!(t1, t2);
    assert!(t1.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Goto clamps to trace length.
#[test]
#[rustfmt::skip]
fn test_d6_2_goto_clamps() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    handle_command(r#"{"cmd":"goto","position":999}"#, &state, &NoOpObserver).unwrap();
    assert_eq!(state.position(), 15, "clamped to length");
}

/// Start a gateway on a temp trace; returns (port, token, path).
async fn start_gateway(name: &str, scenario_size: u64) -> (u16, String, std::path::PathBuf) {
    let name = name.to_string();
    use lisage_liseuse::debugger::standard_scenario;
    use lisage_liseuse::gateway::{GatewayEvents, run_gateway};
    use lisage_liseuse::trace::serialize_event;
    let events = standard_scenario(scenario_size);
    let content = events
        .iter()
        .map(serialize_event)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let path = std::env::temp_dir().join(format!(
        "lisage-gw-{name}-{}-{}.jsonl",
        std::process::id(),
        generate_token().unwrap().as_secret_str()
    ));
    std::fs::write(&path, content).expect("writing temp trace");
    let (tx, rx) = std::sync::mpsc::channel();
    let path_string = path.to_str().unwrap().to_string();
    tokio::spawn(async move {
        let events = GatewayEvents {
            announce: Box::new(move |port, token, _| {
                let _ = tx.send((port, token.as_secret_str().to_string()));
            }),
            connection_error: Box::new(|_| {}),
        };
        let _ = run_gateway(
            &path_string,
            &name,
            std::sync::Arc::new(NoOpObserver),
            events,
        )
        .await;
    });
    let (port, token) = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("announce");
    (port, token, path)
}

/// One raw HTTP GET; returns the response head+body.
async fn http_get(port: u16, target: &str, authorization: Option<&str>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    let header = authorization
        .map(|v| format!("Authorization: {v}\r\n"))
        .unwrap_or_default();
    stream
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n{header}\r\n").as_bytes())
        .await
        .expect("write");
    let mut buffer = vec![0u8; 4096];
    let count = stream.read(&mut buffer).await.expect("read");
    String::from_utf8_lossy(&buffer[..count]).into_owned()
}

/// DoD: the bearer token is enforced on every HTTP connection — none/wrong →
/// 403, `?token=` in the URL → 403 (no secret in URLs), valid bearer → 200.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rustfmt::skip]
async fn test_d6_2_token_enforced_403() {
    let (port, token, path) = start_gateway("dod403", 2u64).await;
    let response = http_get(port, "/", None).await;
    assert!(response.starts_with("HTTP/1.1 403"), "missing bearer → 403, got: {response}");
    let response = http_get(port, "/", Some("Bearer deadbeef")).await;
    assert!(response.starts_with("HTTP/1.1 403"), "wrong bearer → 403, got: {response}");
    let response = http_get(port, &format!("/?token={token}"), None).await;
    assert!(response.starts_with("HTTP/1.1 403"), "token in URL without header → 403, got: {response}");
    let response = http_get(port, "/", Some(&format!("Bearer {token}"))).await;
    assert!(response.starts_with("HTTP/1.1 200") && response.contains("<!DOCTYPE html>"), "valid bearer → 200 page, got: {}", &response[..40]);
    let _ = std::fs::remove_file(&path);
}

/// One WS round trip: handshake, send one frame, return the reply text.
#[rustfmt::skip]
fn ws_roundtrip(port: u16, first_frame: &str) -> String {
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request = format!("ws://127.0.0.1:{port}/ws").into_client_request().expect("url");
    let (mut socket, _) = tokio_tungstenite::tungstenite::client(request, stream).expect("ws handshake");
    socket.send(Message::Text(first_frame.to_string())).expect("send");
    match socket.read().expect("read") { Message::Text(text) => text, other => panic!("expected text, got {other:?}") }
}

/// DoD: the live page drives step/pause/status over a REAL WebSocket — auth
/// as first frame (no secret in the URL), wrong token refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rustfmt::skip]
async fn test_d6_2_websocket_drives_replay() {
    let (port, token, path) = start_gateway("dodws", 3u64).await;
    tokio::task::spawn_blocking(move || {
        let auth = format!(r#"{{"cmd":"auth","token":"{token}"}}"#);
        let reply = ws_roundtrip(port, &auth);
        assert!(reply.contains(r#""event":"auth_ok""#), "auth_ok expected: {reply}");
        // step → position, then pause → paused, then status reports paused=1.
        use tokio_tungstenite::tungstenite::Message;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let request = format!("ws://127.0.0.1:{port}/ws").into_client_request().expect("url");
        let (mut socket, _) = tokio_tungstenite::tungstenite::client(request, stream).expect("hs");
        socket.send(Message::Text(auth)).expect("send");
        assert!(socket.read().expect("read").to_string().contains("auth_ok"));
        socket.send(Message::Text(r#"{"cmd":"step"}"#.to_string())).expect("send");
        assert!(socket.read().expect("read").to_string().contains(r#""event":"position""#), "step → position");
        socket.send(Message::Text(r#"{"cmd":"pause"}"#.to_string())).expect("send");
        assert!(socket.read().expect("read").to_string().contains(r#""event":"paused""#), "pause → paused");
        socket.send(Message::Text(r#"{"cmd":"status"}"#.to_string())).expect("send");
        assert!(socket.read().expect("read").to_string().contains(r#""paused":1"#), "status paused=1");
        // Wrong token on a fresh connection is refused.
        let reply = ws_roundtrip(port, r#"{"cmd":"auth","token":"deadbeef"}"#);
        assert!(reply.contains(r#""event":"auth_failed""#), "wrong token refused: {reply}");
    }).await.expect("client task");
    let _ = std::fs::remove_file(&path);
}
