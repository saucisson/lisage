//! D3-1 tests — command bus of the debugger: navigation over the trace
//! (step/back/goto/step-over/step-out/next-op/next-round/until/
//! continue), where/frame output, visit stack, granularity parity with
//! the span tree. DoD: the standard scenario n=4 runs end-to-end.

use lisage_core::NodeId;
use lisage_core::observer::{SpanId, SpanKind, TraceEvent};
use lisage_liseuse::debugger::{
    Command, Debugger, EventKind, parse_command, round_numbers, standard_scenario,
};

/// Hand-built trace with nested spans: an Op span [0..10] holding a
/// Visite span [2..5] and a second Visite span [6..8].
fn nested_spans_trace() -> Vec<TraceEvent> {
    let visite = |node: u32| TraceEvent::VisiteEntree {
        node: NodeId(node),
        span: None,
    };
    vec![
        TraceEvent::SpanOpen {
            span: SpanId(1),
            kind: SpanKind::Op,
            lane: 0,
        }, // 0
        TraceEvent::OpStart {
            fingerprint: 1,
            span: None,
        }, // 1
        TraceEvent::SpanOpen {
            span: SpanId(2),
            kind: SpanKind::Visite,
            lane: 0,
        }, // 2
        visite(10),                                // 3
        visite(11),                                // 4
        TraceEvent::SpanClose { span: SpanId(2) }, // 5
        TraceEvent::SpanOpen {
            span: SpanId(3),
            kind: SpanKind::Visite,
            lane: 0,
        }, // 6
        visite(12),                                // 7
        TraceEvent::SpanClose { span: SpanId(3) }, // 8
        TraceEvent::OpEnd { span: None },          // 9
        TraceEvent::SpanClose { span: SpanId(1) }, // 10
    ]
}

/// The standard scenario n=4: 4 rounds, 4 Round-span roots.
#[test]
fn test_standard_scenario_n4() {
    let events = standard_scenario(4);
    assert_eq!(round_numbers(&events).len(), 4, "n=4 -> 4 rounds");
    let debugger = Debugger::new(events);
    assert!(debugger.length() > 50, "a real trace, not a toy");
    assert_eq!(debugger.position(), 0);
}

/// step / back / goto move the cursor, clamped at both ends.
#[test]
fn test_step_back_goto() {
    let mut debugger = Debugger::new(nested_spans_trace());
    assert_eq!(debugger.run(Command::Step), debugger.run(Command::Where));
    assert_eq!(debugger.position(), 1);
    debugger.run(Command::Goto(7));
    assert_eq!(debugger.position(), 7);
    debugger.run(Command::Back);
    assert_eq!(debugger.position(), 6);
    debugger.run(Command::Goto(999));
    assert_eq!(debugger.position(), 11, "goto clamps at the trace end");
    debugger.run(Command::Back);
    assert_eq!(debugger.position(), 10);
    debugger.run(Command::Goto(0));
    debugger.run(Command::Back);
    assert_eq!(debugger.position(), 0, "back clamps at 0");
}

/// step-over stops at the end of the innermost enclosing span;
/// step-out at the end of its parent — parity with the span tree.
#[test]
fn test_step_over_step_out_span_boundaries() {
    let mut debugger = Debugger::new(nested_spans_trace());
    debugger.run(Command::Goto(3)); // inside Visite span 2 [2..6)
    assert_eq!(debugger.visit_stack(), vec![SpanId(2)]);
    debugger.run(Command::StepOver);
    assert_eq!(debugger.position(), 6, "step-over: end of the Visite span");
    debugger.run(Command::Goto(3));
    debugger.run(Command::StepOut);
    assert_eq!(
        debugger.position(),
        11,
        "step-out: end of the Op span (parent)"
    );
    // Outside any span, step-over/step-out degrade to a single step.
    let mut debugger = Debugger::new(nested_spans_trace());
    debugger.run(Command::Goto(11));
    debugger.run(Command::StepOver);
    assert_eq!(debugger.position(), 11, "at the end: clamped");
}

/// next-op / next-round / until jump to the next matching event.
#[test]
fn test_next_op_next_round_until() {
    let events = standard_scenario(4);
    let mut debugger = Debugger::new(events);
    debugger.run(Command::NextRound);
    assert!(matches!(
        debugger.current_event(),
        Some(TraceEvent::Round { numero: 1, .. })
    ));
    debugger.run(Command::NextRound);
    assert!(matches!(
        debugger.current_event(),
        Some(TraceEvent::Round { numero: 2, .. })
    ));
    debugger.run(Command::NextOp);
    assert!(matches!(
        debugger.current_event(),
        Some(TraceEvent::OpStart { .. })
    ));
    debugger.run(Command::Until(EventKind::MemoHit));
    assert!(matches!(
        debugger.current_event(),
        Some(TraceEvent::MemoHit { .. })
    ));
    // until with no further match runs to the end.
    debugger.run(Command::Continue);
    assert_eq!(debugger.position(), debugger.length());
    debugger.run(Command::Until(EventKind::Round));
    assert_eq!(
        debugger.position(),
        debugger.length(),
        "no round after the end"
    );
}

/// `where` shows the position, the event kind, the enclosing spans and
/// the visit stack; `frame` shows the innermost span with its balance.
#[test]
fn test_where_frame_visit_stack() {
    let mut debugger = Debugger::new(nested_spans_trace());
    debugger.run(Command::Goto(3));
    let where_out = debugger.run(Command::Where);
    assert!(where_out.contains("pos 3/11"), "{where_out}");
    assert!(where_out.contains("VisiteEntree"), "{where_out}");
    assert!(where_out.contains("Op[0..11]"), "{where_out}");
    assert!(where_out.contains("Visite[2..6]"), "{where_out}");
    assert!(where_out.contains("SpanId(2)"), "visit stack: {where_out}");
    let frame = debugger.run(Command::Frame);
    assert!(frame.contains("Visite"), "{frame}");
    assert!(frame.contains("range 2..6"), "{frame}");
    assert!(frame.contains("balance"), "{frame}");
    // Outside any span, frame reports it.
    debugger.run(Command::Goto(11));
    assert!(debugger.run(Command::Frame).contains("no enclosing span"));
}

/// Command parsing: aliases and arguments.
#[test]
fn test_parse_command() {
    assert_eq!(parse_command("step"), Some(Command::Step));
    assert_eq!(parse_command("s"), Some(Command::Step));
    assert_eq!(parse_command("back"), Some(Command::Back));
    assert_eq!(parse_command("goto 42"), Some(Command::Goto(42)));
    assert_eq!(parse_command("step-over"), Some(Command::StepOver));
    assert_eq!(parse_command("so"), Some(Command::StepOver));
    assert_eq!(parse_command("step-out"), Some(Command::StepOut));
    assert_eq!(parse_command("next-op"), Some(Command::NextOp));
    assert_eq!(parse_command("next-round"), Some(Command::NextRound));
    assert_eq!(
        parse_command("until memo-hit"),
        Some(Command::Until(EventKind::MemoHit))
    );
    assert_eq!(parse_command("continue"), Some(Command::Continue));
    assert_eq!(parse_command("where"), Some(Command::Where));
    assert_eq!(parse_command("frame"), Some(Command::Frame));
    assert_eq!(parse_command("frobnicate"), None);
    assert_eq!(parse_command("goto"), None);
    assert_eq!(parse_command("until"), None);
}

/// DoD: the standard scenario n=4 runs end-to-end through a scripted
/// debugging session (well under 5 minutes — it runs in milliseconds).
#[test]
fn test_dod_scenario_n4_end_to_end() {
    let mut debugger = Debugger::new(standard_scenario(4));
    let script = [
        "where",
        "next-round",
        "next-round",
        "frame",
        "step",
        "step-over",
        "step-out",
        "back",
        "goto 0",
        "until round",
        "next-op",
        "until memo-hit",
        "continue",
    ];
    for line in script {
        let command = parse_command(line).expect("script command parses");
        let output = debugger.run(command);
        assert!(!output.is_empty(), "command {line} answers");
    }
    assert_eq!(
        debugger.position(),
        debugger.length(),
        "continue reaches the end"
    );
    // The session visited every round: 4 rounds in the scenario.
    assert_eq!(round_numbers(&standard_scenario(4)).len(), 4);
}
