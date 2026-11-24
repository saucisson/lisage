//! D1-4 tests — trace deserializer: line-level round-trip on real and
//! hand-built traces (every event variant), explicit rejection of
//! unknown versions (incl. v2), and malformed-input rejection with line
//! numbers.

use lisage_core::observer::{
    ChannelAttr, ClassHistogram, ReductionRule, SpanId, SpanKind, StructureChangeKind, TraceEvent,
    TraceVal,
};
use lisage_core::{Anchor, ControllerId, Handle, Level, NodeId};
use lisage_liseuse::debugger::standard_scenario;
use lisage_liseuse::trace::{TRACE_VERSION, deserialize_event, deserialize_trace, serialize_event};

/// One event of every variant (span set on half of them), to cover the
/// whole deserializer.
fn all_the_events() -> Vec<TraceEvent> {
    let span = Some(SpanId(3));
    vec![
        TraceEvent::SpanOpen {
            span: SpanId(1),
            kind: SpanKind::Op,
            lane: 0,
        },
        TraceEvent::SpanOpen {
            span: SpanId(2),
            kind: SpanKind::Round,
            lane: 1,
        },
        TraceEvent::SpanOpen {
            span: SpanId(3),
            kind: SpanKind::Visit,
            lane: 0,
        },
        TraceEvent::SpanOpen {
            span: SpanId(4),
            kind: SpanKind::Fire,
            lane: 0,
        },
        TraceEvent::SpanClose { span: SpanId(1) },
        TraceEvent::NodeCreated {
            node: NodeId(7),
            level: Some(Level(2)),
            form: TraceVal::Text("Node { level: 2 }".to_string()),
            span,
        },
        TraceEvent::NodeCreated {
            node: NodeId(8),
            level: None,
            form: TraceVal::Text("Terminal(true)".to_string()),
            span: None,
        },
        TraceEvent::NodeShared {
            node: NodeId(7),
            span,
        },
        TraceEvent::ReductionApplied {
            rule: ReductionRule::Share,
            level: Level(1),
            span,
        },
        TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level: Level(0),
            span: None,
        },
        TraceEvent::ReductionApplied {
            rule: ReductionRule::Default,
            level: Level(3),
            span,
        },
        TraceEvent::DiagramBuilt {
            handle: Handle {
                controller: ControllerId(2),
                anchor: Anchor(0),
                node: NodeId(7),
            },
            nodes: 42,
            span,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Inherited,
            node: NodeId(7),
            value: TraceVal::Text("5".to_string()),
            span,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node: NodeId(8),
            value: TraceVal::Text("{1, 2}".to_string()),
            span: None,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Memory,
            node: NodeId(9),
            value: TraceVal::Text("x\"y\\z".to_string()),
            span,
        },
        TraceEvent::MemoHit {
            node: NodeId(7),
            span,
        },
        TraceEvent::MemoMiss {
            node: NodeId(8),
            span: None,
        },
        TraceEvent::Gc {
            live: 10,
            collected: 2,
            compaction: true,
            collected_list: vec![NodeId(3), NodeId(4)],
            span,
        },
        TraceEvent::Round {
            number: 2,
            active_levels: vec![0, 1],
            modified_levels: vec![1],
            before: NodeId(7),
            image: NodeId(8),
            after: NodeId(9),
            growth: -3,
            span,
        },
        TraceEvent::LocalFire {
            number: 4,
            level: 2,
            before: NodeId(7),
            image: NodeId(10),
            merged: NodeId(11),
            span,
        },
        TraceEvent::Histogram {
            classes: vec![ClassHistogram {
                class: 1,
                live: 10,
                peak: 12,
                slack: 2,
            }],
            span,
        },
        TraceEvent::OpStart {
            fingerprint: 4164,
            span,
        },
        TraceEvent::OpEnd { span },
        TraceEvent::VisitEntry {
            node: NodeId(7),
            span,
        },
        TraceEvent::VisitExit {
            node: NodeId(7),
            result: NodeId(8),
            span,
        },
        TraceEvent::MemoConsult {
            fingerprint: 4164,
            node: NodeId(7),
            h: TraceVal::Text("3".to_string()),
            version: 1,
            hit: true,
            span,
        },
        TraceEvent::MemoInsert {
            fingerprint: 4164,
            node: NodeId(7),
            h: TraceVal::Text("3".to_string()),
            version: 1,
            result: NodeId(8),
            span,
        },
        TraceEvent::MemoryWrite {
            version_before: 0,
            version_after: 1,
            span,
        },
        TraceEvent::StructureChange {
            node: NodeId(7),
            classification: StructureChangeKind::Collapse,
            arcs_before: vec![(0, NodeId(3)), (1, NodeId(4))],
            arcs_after: vec![(1, NodeId(4))],
            span,
        },
        TraceEvent::StructureChange {
            node: NodeId(8),
            classification: StructureChangeKind::Expansion,
            arcs_before: vec![],
            arcs_after: vec![(0, NodeId(3))],
            span: None,
        },
    ]
}

/// Line-level round-trip: serialize → deserialize → serialize gives the
/// identical line, for every event of the trace.
fn assert_round_trip_line(events: &[TraceEvent]) {
    for event in events {
        let line = serialize_event(event);
        let back = deserialize_event(&line).expect("deserializes");
        let line2 = serialize_event(&back);
        assert_eq!(line, line2, "round-trip line identical for {event:?}");
    }
}

/// DoD: round-trip identical on the test traces (real scenario + every
/// event variant).
#[test]
fn test_round_trip_over_test_traces() {
    assert_round_trip_line(&standard_scenario(4));
    assert_round_trip_line(&standard_scenario(7));
    assert_round_trip_line(&all_the_events());
}

/// Whole-document round-trip: JSONL in → events → JSONL out, identical.
#[test]
fn test_round_trip_document() {
    let events = standard_scenario(4);
    let document: String = events
        .iter()
        .map(serialize_event)
        .map(|l| format!("{l}\n"))
        .collect();
    let back = deserialize_trace(&document).expect("document deserializes");
    let document2: String = back
        .iter()
        .map(serialize_event)
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(document, document2);
    assert_eq!(back.len(), events.len());
}

/// Event-level equality for the events without lazy values (the lazy
/// `TraceVal` fragments become `Text` on deserialization — covered by
/// the line-level round-trip above).
#[test]
fn test_equality_event_without_values_lazy() {
    for event in all_the_events() {
        let a_lazy = matches!(
            event,
            TraceEvent::NodeCreated { .. }
                | TraceEvent::Attr { .. }
                | TraceEvent::MemoConsult { .. }
                | TraceEvent::MemoInsert { .. }
        );
        if a_lazy {
            continue;
        }
        let line = serialize_event(&event);
        let back = deserialize_event(&line).expect("deserializes");
        assert_eq!(&back, &event, "event-level equality for {event:?}");
    }
    // A Text-valued event round-trips at the event level too.
    let event = TraceEvent::NodeCreated {
        node: NodeId(7),
        level: Some(Level(2)),
        form: TraceVal::Text("abc".to_string()),
        span: None,
    };
    let back = deserialize_event(&serialize_event(&event)).unwrap();
    assert_eq!(back, event);
}

/// Unknown versions are rejected explicitly — including v2 (documented
/// decision: no silent upgrade, re-run to produce a v6 trace).
#[test]
fn test_rejection_versions_inconnues() {
    for version in [0, 2, 3, 4, 5, 99] {
        let line = format!("{{\"v\":{version},\"type\":\"OpEnd\",\"span\":null}}");
        let error = deserialize_event(&line).unwrap_err();
        assert!(
            error.contains(&format!("unsupported trace version {version}")),
            "{error}"
        );
        assert!(
            error.contains(&format!("expected {TRACE_VERSION}")),
            "{error}"
        );
    }
    // Missing version.
    let error = deserialize_event("{\"type\":\"OpEnd\",\"span\":null}").unwrap_err();
    assert!(error.contains("'v'"), "{error}");
}

/// Malformed input: bad JSON, unknown type, missing field — with the
/// 1-based line number in whole-document mode.
#[test]
fn test_rejection_entries_malformed() {
    assert!(deserialize_event("not json").is_err());
    assert!(deserialize_event("{\"v\":6,\"type\":\"Nope\",\"span\":null}").is_err());
    assert!(deserialize_event("{\"v\":6,\"type\":\"OpStart\",\"span\":null}").is_err());
    assert!(deserialize_event("{\"v\":6,\"type\":\"MemoHit\",\"span\":null}").is_err());
    assert!(
        deserialize_event("{\"v\":6,\"type\":\"SpanOpen\",\"span\":1,\"kind\":\"x\",\"lane\":0}")
            .is_err()
    );
    let doc = "{\"v\":6,\"type\":\"OpEnd\",\"span\":null}\n\
               {\"v\":6,\"type\":\"MemoHit\",\"span\":null}\n";
    let error = deserialize_trace(doc).unwrap_err();
    assert_eq!(error.line, 2, "the failing line is reported");
    assert!(error.message.contains("noeud"), "{}", error.message);
    // Empty lines are skipped.
    let doc = "\n{\"v\":6,\"type\":\"OpEnd\",\"span\":null}\n\n";
    assert_eq!(deserialize_trace(doc).unwrap().len(), 1);
}

/// The deserialized trace is usable by the D2/D3 machinery: the span
/// tree builds on it and finds the same rounds.
#[test]
fn test_trace_deserialisee_utilisable() {
    use lisage_liseuse::debugger::{Debugger, round_numbers};
    use lisage_liseuse::spans::SpanTree;

    let events = standard_scenario(4);
    let document: String = events
        .iter()
        .map(serialize_event)
        .map(|l| format!("{l}\n"))
        .collect();
    let back = deserialize_trace(&document).unwrap();
    assert_eq!(round_numbers(&back).len(), 4);
    let tree = SpanTree::build(&back);
    assert_eq!(tree.roots().len(), 4, "4 Round-span roots");
    let debugger = Debugger::new(back);
    assert!(debugger.length() > 50);
}
