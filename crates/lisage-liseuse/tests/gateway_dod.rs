//! D6-2 DoD: the live page drives pause/step/break of an attached program
//! (here a `.jsonl` trace file replayed by the gateway).

use std::sync::Arc;
use std::sync::atomic::Ordering;

use lisage_core::NodeId;
use lisage_core::observer::TraceEvent;
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
    let resp = handle_command(r#"{"cmd":"break","round":12}"#, &state).unwrap();
    assert!(resp.contains(r#""event":"break_set""#));
    // Continue from position 0 → should stop at round 12 (position 11, 0-based).
    let resp = handle_command(r#"{"cmd":"continue"}"#, &state).unwrap();
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
    let resp = handle_command(r#"{"cmd":"step"}"#, &state).unwrap();
    assert!(resp.contains(r#""position":1"#));
    // pause
    let resp = handle_command(r#"{"cmd":"pause"}"#, &state).unwrap();
    assert!(resp.contains(r#""event":"paused""#));
    assert_eq!(state.paused.load(Ordering::SeqCst), 1);
    // back
    let resp = handle_command(r#"{"cmd":"back"}"#, &state).unwrap();
    assert!(resp.contains(r#""position":0"#));
    // status
    let resp = handle_command(r#"{"cmd":"status"}"#, &state).unwrap();
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
    assert_eq!(t1.len(), 32);
    assert_ne!(t1, t2);
    assert!(t1.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Goto clamps to trace length.
#[test]
fn test_d6_2_goto_clamps() {
    let state = Arc::new(GatewayState::new(sample_trace()));
    handle_command(r#"{"cmd":"goto","position":999}"#, &state).unwrap();
    assert_eq!(state.position(), 15, "clamped to length");
}
