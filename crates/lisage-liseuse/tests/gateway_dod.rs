//! D6-2 DoD: the live page drives pause/step/break of an attached program
//! (here a `.jsonl` trace file replayed by the gateway).

use std::sync::Arc;
use std::sync::atomic::Ordering;

use lisage_core::NodeId;
use lisage_core::observer::{NoOpObserver, TraceEvent};
use lisage_liseuse::gateway::{GatewayState, generate_token, handle_command, render_live_page};

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
    // Set break at round 12.
    let resp = handle_command(r#"{"cmd":"break","round":12}"#, &state, &NoOpObserver).unwrap();
    assert!(resp.contains(r#""event":"break_set""#));
    // Continue from position 0 → should stop at round 12 (position 11, 0-based).
    let resp = handle_command(r#"{"cmd":"continue"}"#, &state, &NoOpObserver).unwrap();
    assert!(
        resp.contains(r#""event":"break""#),
        "continue must hit the round-12 break: {resp}"
    );
    assert!(
        resp.contains(r#""position":12"#),
        "break after round 12 (position 12)"
    );
    assert_eq!(state.position(), 12);
}

/// DoD: step/pause/back/status commands work over the JSON protocol.
#[test]
fn test_d6_2_step_pause_back_status() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    // step
    let resp = handle_command(r#"{"cmd":"step"}"#, &state, &NoOpObserver).unwrap();
    assert!(resp.contains(r#""position":1"#));
    // pause
    let resp = handle_command(r#"{"cmd":"pause"}"#, &state, &NoOpObserver).unwrap();
    assert!(resp.contains(r#""event":"paused""#));
    assert_eq!(state.paused.load(Ordering::SeqCst), 1);
    // back
    let resp = handle_command(r#"{"cmd":"back"}"#, &state, &NoOpObserver).unwrap();
    assert!(resp.contains(r#""position":0"#));
    // status
    let resp = handle_command(r#"{"cmd":"status"}"#, &state, &NoOpObserver).unwrap();
    assert!(resp.contains(r#""event":"status""#));
    assert!(resp.contains(r#""length":15"#));
    assert!(resp.contains(r#""paused":1"#));
}

/// The live page is self-contained and connects with the token.
#[test]
fn test_d6_2_live_page_self_contained() {
    let html = render_live_page("test", 42);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("42 events"));
    assert!(html.contains("token"), "page reads token from query string");
    assert!(html.contains("WebSocket"), "page opens a WebSocket");
    assert!(html.contains("step"), "step button");
    assert!(html.contains("pause"), "pause button");
    assert!(html.contains("break round 12"), "break button");
    // No external resources.
    assert!(!html.contains(r#"src="http"#));
    assert!(!html.contains(r#"href="http"#));
}

/// Token generation: 32 hex chars, unique across calls.
#[test]
fn test_d6_2_token() {
    let t1 = generate_token();
    let t2 = generate_token();
    let t1 = t1.to_string();
    let t2 = t2.to_string();
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

    // Write a real .jsonl trace to a temp file.
    let events = standard_scenario(2);
    let mut content = String::new();
    for event in &events {
        content.push_str(&serialiser_evenement(event));
        content.push('\n');
    }
    let path = std::env::temp_dir().join(format!(
        "lisage-gateway-dod-{}-{}.jsonl",
        std::process::id(),
        generate_token()
    ));
    std::fs::write(&path, &content).expect("writing temp trace");

    let (announce_sender, announce_receiver) = std::sync::mpsc::channel();
    let path_string = path.to_str().expect("temp path is UTF-8").to_string();
    tokio::spawn(async move {
        let events = GatewayEvents {
            announce: Box::new(move |port, token, _event_count| {
                let _ = announce_sender.send((port, token.to_string()));
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

    async fn request(port: u16, target: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connecting to the loopback gateway");
        stream
            .write_all(format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
            .await
            .expect("sending request");
        let mut buffer = vec![0u8; 4096];
        let count = stream.read(&mut buffer).await.expect("reading response");
        String::from_utf8_lossy(&buffer[..count]).into_owned()
    }

    // No token → 403.
    let response = request(port, "/").await;
    assert!(
        response.starts_with("HTTP/1.1 403"),
        "missing token must be rejected with 403, got: {response}"
    );
    // Wrong token → 403.
    let response = request(port, "/?token=deadbeef").await;
    assert!(
        response.starts_with("HTTP/1.1 403"),
        "wrong token must be rejected with 403, got: {response}"
    );
    // Valid token → 200 with the HTML page.
    let response = request(port, &format!("/?token={token}")).await;
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "valid token must get 200, got: {response}"
    );
    assert!(response.contains("<!DOCTYPE html>"), "the page is served");

    let _ = std::fs::remove_file(&path);
}
