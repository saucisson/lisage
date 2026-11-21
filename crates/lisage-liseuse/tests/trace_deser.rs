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
use lisage_liseuse::trace::{
    TRACE_VERSION, deserialiser_evenement, deserialiser_trace, serialiser_evenement,
};

/// One event of every variant (span set on half of them), to cover the
/// whole deserializer.
fn tous_les_evenements() -> Vec<TraceEvent> {
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
            kind: SpanKind::Visite,
            lane: 0,
        },
        TraceEvent::SpanClose { span: SpanId(1) },
        TraceEvent::NodeCreated {
            node: NodeId(7),
            level: Some(Level(2)),
            forme: TraceVal::Texte("Node { level: 2 }".to_string()),
            span,
        },
        TraceEvent::NodeCreated {
            node: NodeId(8),
            level: None,
            forme: TraceVal::Texte("Terminal(true)".to_string()),
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
            value: TraceVal::Texte("5".to_string()),
            span,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node: NodeId(8),
            value: TraceVal::Texte("{1, 2}".to_string()),
            span: None,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Memory,
            node: NodeId(9),
            value: TraceVal::Texte("x\"y\\z".to_string()),
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
            liste_collectes: vec![NodeId(3), NodeId(4)],
            span,
        },
        TraceEvent::Round {
            numero: 2,
            niveaux_actifs: vec![0, 1],
            niveaux_modifies: vec![1],
            avant: NodeId(7),
            image: NodeId(8),
            apres: NodeId(9),
            croissance: -3,
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
        TraceEvent::VisiteEntree {
            node: NodeId(7),
            span,
        },
        TraceEvent::VisiteSortie {
            node: NodeId(7),
            resultat: NodeId(8),
            span,
        },
        TraceEvent::MemoConsult {
            fingerprint: 4164,
            node: NodeId(7),
            h: TraceVal::Texte("3".to_string()),
            version: 1,
            hit: true,
            span,
        },
        TraceEvent::MemoInsert {
            fingerprint: 4164,
            node: NodeId(7),
            h: TraceVal::Texte("3".to_string()),
            version: 1,
            resultat: NodeId(8),
            span,
        },
        TraceEvent::EcritureMemoire {
            version_avant: 0,
            version_apres: 1,
            span,
        },
        TraceEvent::StructureChange {
            node: NodeId(7),
            classification: StructureChangeKind::Collapse,
            arcs_avant: vec![(0, NodeId(3)), (1, NodeId(4))],
            arcs_apres: vec![(1, NodeId(4))],
            span,
        },
        TraceEvent::StructureChange {
            node: NodeId(8),
            classification: StructureChangeKind::Expansion,
            arcs_avant: vec![],
            arcs_apres: vec![(0, NodeId(3))],
            span: None,
        },
    ]
}

/// Line-level round-trip: serialize → deserialize → serialize gives the
/// identical line, for every event of the trace.
fn assert_round_trip_ligne(events: &[TraceEvent]) {
    for event in events {
        let ligne = serialiser_evenement(event);
        let back = deserialiser_evenement(&ligne).expect("deserializes");
        let ligne2 = serialiser_evenement(&back);
        assert_eq!(ligne, ligne2, "round-trip line identical for {event:?}");
    }
}

/// DoD: round-trip identical on the test traces (real scenario + every
/// event variant).
#[test]
fn test_round_trip_traces_de_test() {
    assert_round_trip_ligne(&standard_scenario(4));
    assert_round_trip_ligne(&standard_scenario(7));
    assert_round_trip_ligne(&tous_les_evenements());
}

/// Whole-document round-trip: JSONL in → events → JSONL out, identical.
#[test]
fn test_round_trip_document() {
    let events = standard_scenario(4);
    let document: String = events
        .iter()
        .map(serialiser_evenement)
        .map(|l| format!("{l}\n"))
        .collect();
    let back = deserialiser_trace(&document).expect("document deserializes");
    let document2: String = back
        .iter()
        .map(serialiser_evenement)
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(document, document2);
    assert_eq!(back.len(), events.len());
}

/// Event-level equality for the events without lazy values (the lazy
/// `TraceVal` fragments become `Texte` on deserialization — covered by
/// the line-level round-trip above).
#[test]
fn test_egalite_evenementielle_sans_valeurs_lazy() {
    for event in tous_les_evenements() {
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
        let ligne = serialiser_evenement(&event);
        let back = deserialiser_evenement(&ligne).expect("deserializes");
        assert_eq!(&back, &event, "event-level equality for {event:?}");
    }
    // A Texte-valued event round-trips at the event level too.
    let event = TraceEvent::NodeCreated {
        node: NodeId(7),
        level: Some(Level(2)),
        forme: TraceVal::Texte("abc".to_string()),
        span: None,
    };
    let back = deserialiser_evenement(&serialiser_evenement(&event)).unwrap();
    assert_eq!(back, event);
}

/// Unknown versions are rejected explicitly — including v2 (documented
/// decision: no silent upgrade, re-run to produce a v5 trace).
#[test]
fn test_rejet_versions_inconnues() {
    for version in [0, 2, 3, 4, 6, 99] {
        let ligne = format!("{{\"v\":{version},\"type\":\"OpEnd\",\"span\":null}}");
        let erreur = deserialiser_evenement(&ligne).unwrap_err();
        assert!(
            erreur.contains(&format!("unsupported trace version {version}")),
            "{erreur}"
        );
        assert!(
            erreur.contains(&format!("expected {TRACE_VERSION}")),
            "{erreur}"
        );
    }
    // Missing version.
    let erreur = deserialiser_evenement("{\"type\":\"OpEnd\",\"span\":null}").unwrap_err();
    assert!(erreur.contains("'v'"), "{erreur}");
}

/// Malformed input: bad JSON, unknown type, missing field — with the
/// 1-based line number in whole-document mode.
#[test]
fn test_rejet_entrees_malformees() {
    assert!(deserialiser_evenement("not json").is_err());
    assert!(deserialiser_evenement("{\"v\":5,\"type\":\"Nope\",\"span\":null}").is_err());
    assert!(deserialiser_evenement("{\"v\":5,\"type\":\"OpStart\",\"span\":null}").is_err());
    assert!(deserialiser_evenement("{\"v\":5,\"type\":\"MemoHit\",\"span\":null}").is_err());
    assert!(
        deserialiser_evenement(
            "{\"v\":5,\"type\":\"SpanOpen\",\"span\":1,\"kind\":\"x\",\"lane\":0}"
        )
        .is_err()
    );
    let doc = "{\"v\":5,\"type\":\"OpEnd\",\"span\":null}\n\
               {\"v\":5,\"type\":\"MemoHit\",\"span\":null}\n";
    let erreur = deserialiser_trace(doc).unwrap_err();
    assert_eq!(erreur.ligne, 2, "the failing line is reported");
    assert!(erreur.message.contains("noeud"), "{}", erreur.message);
    // Empty lines are skipped.
    let doc = "\n{\"v\":5,\"type\":\"OpEnd\",\"span\":null}\n\n";
    assert_eq!(deserialiser_trace(doc).unwrap().len(), 1);
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
        .map(serialiser_evenement)
        .map(|l| format!("{l}\n"))
        .collect();
    let back = deserialiser_trace(&document).unwrap();
    assert_eq!(round_numbers(&back).len(), 4);
    let tree = SpanTree::build(&back);
    assert_eq!(tree.roots().len(), 4, "4 Round-span roots");
    let debugger = Debugger::new(back);
    assert!(debugger.length() > 50);
}
