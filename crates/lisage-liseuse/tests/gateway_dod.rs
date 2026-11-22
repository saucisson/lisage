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
            numero: i,
            niveaux_actifs: vec![0],
            niveaux_modifies: if i < 15 { vec![0] } else { vec![] },
            avant: NodeId(i),
            image: NodeId(i),
            apres: NodeId(i),
            croissance: if i < 15 { 1 } else { 0 },
            span: None,
        })
        .collect()
}

/// DoD: break at round 12 is hit by `continue`, not at the end.
#[test]
fn test_d6_2_break_round_12_caught() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    let response = handle_command(r#"{"cmd":"break","round":12}"#, &state, &NoOpObserver).unwrap();
    assert!(response.contains(r#""event":"break_set""#));
    let response = handle_command(r#"{"cmd":"continue"}"#, &state, &NoOpObserver).unwrap();
    assert!(
        response.contains(r#""event":"break""#),
        "continue must hit the round-12 break: {response}"
    );
    assert!(
        response.contains(r#""position":12"#),
        "break after round 12 (position 12)"
    );
    assert_eq!(state.position(), 12);
}

/// DoD: step/pause/back/status commands work over the JSON protocol.
#[test]
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
fn test_d6_2_live_page_self_contained() {
    let token = ConnectionToken::from_hex_for_test("abc123");
    let html = render_live_page("test", 42, &token);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("42 events"));
    assert!(
        html.contains("auth"),
        "page authenticates the WebSocket as first frame"
    );
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
fn test_d6_2_goto_clamps() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    handle_command(r#"{"cmd":"goto","position":999}"#, &state, &NoOpObserver).unwrap();
    assert_eq!(state.position(), 15, "clamped to length");
}

/// DoD: the token is enforced on every connection — a request without the
/// token (or with a wrong one) is rejected with 403, the valid token gets
/// the page with 200. Exercises the real TCP path of `run_gateway`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_d6_2_token_enforced_403() {
    use lisage_liseuse::debugger::standard_scenario;
    use lisage_liseuse::gateway::{GatewayEvents, run_gateway};
    use lisage_liseuse::trace::serialiser_evenement;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let events = standard_scenario(2);
    let mut content = String::new();
    for event in &events {
        content.push_str(&serialiser_evenement(event));
        content.push('\n');
    }
    let path = std::env::temp_dir().join(format!(
        "lisage-gateway-dod-{}-{}.jsonl",
        std::process::id(),
        generate_token().expect("entropy source available")
    ));
    std::fs::write(&path, &content).expect("writing temp trace");

    let (announce_sender, announce_receiver) = std::sync::mpsc::channel();
    let path_string = path.to_str().expect("temp path is UTF-8").to_string();
    tokio::spawn(async move {
        let events = GatewayEvents {
            announce: Box::new(move |port, token, _event_count| {
                let _ = announce_sender.send((port, token.as_secret_str().to_string()));
            }),
            connection_error: Box::new(|_message| {}),
        };
        let _ = run_gateway(
            &path_string,
            "dod",
            std::sync::Arc::new(NoOpObserver),
            events,
        )
        .await;
    });
    let (port, token) = announce_receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("gateway announces its port");

    async fn request(port: u16, authorization: Option<&str>) -> String {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connecting to the loopback gateway");
        let header = authorization
            .map(|value| format!("Authorization: {value}\r\n"))
            .unwrap_or_default();
        stream
            .write_all(format!("GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n{header}\r\n").as_bytes())
            .await
            .expect("sending request");
        let mut buffer = vec![0u8; 4096];
        let count = stream.read(&mut buffer).await.expect("reading response");
        String::from_utf8_lossy(&buffer[..count]).into_owned()
    }

    let response = request(port, None).await;
    assert!(
        response.starts_with("HTTP/1.1 403"),
        "missing bearer must be rejected with 403, got: {response}"
    );
    let response = request(port, Some("Bearer deadbeef")).await;
    assert!(
        response.starts_with("HTTP/1.1 403"),
        "wrong token must be rejected with 403, got: {response}"
    );
    // Token in the URL is NOT accepted (no secret in URLs, ADR-0016) — the
    // header is required even when the query string carries it.
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connecting");
    stream
        .write_all(format!("GET /?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
        .await
        .expect("sending request");
    let mut buffer = vec![0u8; 4096];
    let count = stream.read(&mut buffer).await.expect("reading response");
    let response = String::from_utf8_lossy(&buffer[..count]).into_owned();
    assert!(
        response.starts_with("HTTP/1.1 403"),
        "token in URL without header must be rejected with 403, got: {response}"
    );
    let response = request(port, Some(&format!("Bearer {token}"))).await;
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "valid bearer must get 200, got: {response}"
    );
    assert!(response.contains("<!DOCTYPE html>"), "the page is served");

    let _ = std::fs::remove_file(&path);
}

/// DoD: the live page drives pause/step/break of the replayed trace over a
/// REAL WebSocket — auth as first frame (no secret in the URL), then
/// step/pause/break/continue frames, asserting the server events.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_d6_2_websocket_drives_replay() {
    use lisage_liseuse::debugger::standard_scenario;
    use lisage_liseuse::gateway::{GatewayEvents, run_gateway};
    use lisage_liseuse::trace::serialiser_evenement;

    let events = standard_scenario(3);
    let mut content = String::new();
    for event in &events {
        content.push_str(&serialiser_evenement(event));
        content.push('\n');
    }
    let path = std::env::temp_dir().join(format!(
        "lisage-gateway-ws-dod-{}-{}.jsonl",
        std::process::id(),
        generate_token()
            .expect("entropy source available")
            .as_secret_str()
    ));
    std::fs::write(&path, &content).expect("writing temp trace");

    let (announce_sender, announce_receiver) = std::sync::mpsc::channel();
    let path_string = path.to_str().expect("temp path is UTF-8").to_string();
    tokio::spawn(async move {
        let events = GatewayEvents {
            announce: Box::new(move |port, token, _event_count| {
                let _ = announce_sender.send((port, token.as_secret_str().to_string()));
            }),
            connection_error: Box::new(|_message| {}),
        };
        let _ = run_gateway(
            &path_string,
            "dod-ws",
            std::sync::Arc::new(NoOpObserver),
            events,
        )
        .await;
    });
    let (port, token) = announce_receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("gateway announces its port");

    tokio::task::spawn_blocking(move || {
        use tokio_tungstenite::tungstenite::Message;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connecting");
        let request = format!("ws://127.0.0.1:{port}/ws")
            .into_client_request()
            .expect("valid ws url");
        let (mut socket, _response) =
            tokio_tungstenite::tungstenite::client(request, stream).expect("ws handshake");

        // First frame: auth (no secret in the URL).
        socket
            .send(Message::Text(format!(
                r#"{{"cmd":"auth","token":"{token}"}}"#
            )))
            .expect("sending auth frame");
        let reply = socket.read().expect("reading auth reply");
        assert!(
            matches!(&reply, Message::Text(text) if text.contains(r#""event":"auth_ok""#)),
            "auth frame must be acknowledged: {reply:?}"
        );

        // step → position event.
        socket
            .send(Message::Text(r#"{"cmd":"step"}"#.to_string()))
            .expect("sending step");
        let reply = socket.read().expect("reading step reply");
        assert!(
            matches!(&reply, Message::Text(text) if text.contains(r#""event":"position""#)),
            "step must produce a position event: {reply:?}"
        );

        // pause → paused event.
        socket
            .send(Message::Text(r#"{"cmd":"pause"}"#.to_string()))
            .expect("sending pause");
        let reply = socket.read().expect("reading pause reply");
        assert!(
            matches!(&reply, Message::Text(text) if text.contains(r#""event":"paused""#)),
            "pause must be acknowledged: {reply:?}"
        );

        // status over the wire reports paused=1.
        socket
            .send(Message::Text(r#"{"cmd":"status"}"#.to_string()))
            .expect("sending status");
        let reply = socket.read().expect("reading status reply");
        assert!(
            matches!(&reply, Message::Text(text) if text.contains(r#""paused":1"#)),
            "status must report paused=1: {reply:?}"
        );

        // Wrong auth on a fresh connection is refused.
        let stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connecting");
        let request = format!("ws://127.0.0.1:{port}/ws")
            .into_client_request()
            .expect("valid ws url");
        let (mut bad_socket, _) =
            tokio_tungstenite::tungstenite::client(request, stream).expect("ws handshake");
        bad_socket
            .send(Message::Text(
                r#"{"cmd":"auth","token":"deadbeef"}"#.to_string(),
            ))
            .expect("sending bad auth");
        let reply = bad_socket.read().expect("reading auth reply");
        assert!(
            matches!(&reply, Message::Text(text) if text.contains(r#""event":"auth_failed""#)),
            "wrong token must be refused: {reply:?}"
        );
    })
    .await
    .expect("blocking client task");
    let _ = std::fs::remove_file(&path);
}
