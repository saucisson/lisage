//! D6-2 property tier (proptest, dev-dependency approved on #175):
//! invariants of the GatewayState replay/break state machine over arbitrary
//! command sequences.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use lisage_core::NodeId;
use lisage_core::observer::{NoOpObserver, TraceEvent};
use lisage_liseuse::gateway::{GatewayState, handle_command};
use proptest::prelude::*;

fn trace_with_rounds(count: u32) -> Vec<TraceEvent> {
    (1..=count)
        .map(|numero| TraceEvent::Round {
            numero,
            niveaux_actifs: vec![0],
            niveaux_modifies: vec![0],
            avant: NodeId(numero),
            image: NodeId(numero),
            apres: NodeId(numero),
            croissance: 1,
            span: None,
        })
        .collect()
}

/// One random command: (json, description).
fn command_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r#"{"cmd":"step"}"#.to_string()),
        Just(r#"{"cmd":"back"}"#.to_string()),
        Just(r#"{"cmd":"pause"}"#.to_string()),
        Just(r#"{"cmd":"continue"}"#.to_string()),
        (0u32..100u32).prop_map(|position| format!(r#"{{"cmd":"goto","position":{position}}}"#)),
        (0u32..40u32).prop_map(|round| format!(r#"{{"cmd":"break","round":{round}}}"#)),
    ]
}

proptest! {
    /// Whatever the command sequence, the position always stays within
    /// `[0, length]` and a `step` past the end reports `None`.
    #[test]
    fn prop_position_stays_in_bounds(commands in prop::collection::vec(command_strategy(), 0..64), length in 1u32..40) {
        let state = Arc::new(GatewayState::new(trace_with_rounds(length)));
        for command in &commands {
            let _ = handle_command(command, &state, &NoOpObserver);
            prop_assert!(state.position() <= length as usize, "position {} exceeds length {length} after {command}", state.position());
        }
        while state.step().is_some() {}
        prop_assert_eq!(state.position(), length as usize, "stepping to the end lands on length");
        prop_assert!(state.step().is_none(), "no step past the end");
    }

    /// A `continue` with a break round inside the trace stops exactly at that
    /// round's position; without a matching break it runs to the end.
    #[test]
    fn prop_continue_stops_at_break_round(break_round in 1u32..20, length in 1u32..20) {
        let state = Arc::new(GatewayState::new(trace_with_rounds(length)));
        let _ = handle_command(&format!(r#"{{"cmd":"break","round":{break_round}}}"#), &state, &NoOpObserver);
        let response = handle_command(r#"{"cmd":"continue"}"#, &state, &NoOpObserver).unwrap();
        if break_round <= length {
            prop_assert!(response.contains(r#""event":"break""#), "break round {break_round} within {length} must be hit: {response}");
            prop_assert_eq!(state.position(), break_round as usize, "stop exactly at the break round");
        } else {
            prop_assert!(response.contains(r#""event":"done""#), "no break round within trace: {response}");
            prop_assert_eq!(state.position(), length as usize);
        }
        prop_assert_eq!(state.break_round.load(Ordering::SeqCst), break_round as usize);
    }
}
