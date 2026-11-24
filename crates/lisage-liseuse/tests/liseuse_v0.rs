//! Liseuse v0 tests (M1-9): versioned JSONL serializer and static SVG
//! rendering of the diagram (levels = keys).

use std::collections::BTreeSet;

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiler};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::svg::render_svg;
use lisage_liseuse::trace::{JsonlObserver, TRACE_VERSION, serialize_event};

type PoolRef = Arena<ReferenceController>;

fn schema_set() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1)]))
}

fn set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> lisage_core::NodeId {
    let value = Value::Set(
        atoms
            .iter()
            .map(|a| Value::Atom(Atom(*a)))
            .collect::<BTreeSet<_>>(),
    );
    compiler(pool, anchor, schema, &value).unwrap()
}

/// The JSONL is versioned, one line = one event, and the trace is
/// deterministic (two identical constructions ⇒ same trace).
#[test]
fn test_trace_jsonl_versioned_and_deterministic() {
    let schema = schema_set();
    let build = |observer: &JsonlObserver| {
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(observer.clone()),
        );
        set(&mut pool, Anchor(0), &schema, &[0])
    };
    let observer_a = JsonlObserver::new();
    let observer_b = JsonlObserver::new();
    let root_a = build(&observer_a);
    let root_b = build(&observer_b);
    assert_eq!(root_a, root_b, "C1: same construction ⇒ same root");
    assert_eq!(
        observer_a.lines(),
        observer_b.lines(),
        "deterministic trace (replayable identically)"
    );

    let lines = observer_a.lines();
    assert!(!lines.is_empty());
    for line in &lines {
        assert!(
            line.starts_with(&format!("{{\"v\":{TRACE_VERSION},")),
            "each line carries the schema version: {line}"
        );
        assert!(line.ends_with('}'));
    }
    // The acceptance terminal is created first.
    assert!(
        lines[0].contains("\"type\":\"NodeCreated\"") && lines[0].contains("\"level\":null"),
        "terminal = null level: {}",
        lines[0]
    );
    // The level-0 node exists in the trace.
    assert!(
        lines.iter().any(|l| l.contains("\"level\":0")),
        "a level-0 node was created"
    );
}

/// Exact (golden) serialization of one event of each type.
#[test]
fn test_serialize_event_golden() {
    use lisage_core::observer::{ReductionRule, TraceEvent, TraceVal};
    use lisage_core::{ControllerId, Handle, Level, NodeId};

    assert_eq!(
        serialize_event(&TraceEvent::NodeCreated {
            node: NodeId(3),
            level: Some(Level(2)),
            form: TraceVal::text("Terminal(true)"),
            span: None,
        }),
        "{\"v\":6,\"type\":\"NodeCreated\",\"span\":null,\"noeud\":3,\"level\":2,\"forme\":\"Terminal(true)\"}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::NodeShared {
            node: NodeId(7),
            span: None
        }),
        "{\"v\":6,\"type\":\"NodeShared\",\"span\":null,\"noeud\":7}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level: Level(1),
            span: None,
        }),
        "{\"v\":6,\"type\":\"Reduction\",\"span\":null,\"rule\":\"R-collapse\",\"level\":1}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::DiagramBuilt {
            handle: Handle {
                controller: ControllerId(1),
                anchor: Anchor(0),
                node: NodeId(4)
            },
            nodes: 5,
            span: None,
        }),
        "{\"v\":6,\"type\":\"DiagramBuilt\",\"span\":null,\"controller\":1,\"anchor\":0,\"noeud\":4,\"noeuds\":5}"
    );
    // MEM-17: the degree histogram per class serializes as a JSON array.
    use lisage_core::observer::ClassHistogram;
    assert_eq!(
        serialize_event(&TraceEvent::Histogram {
            classes: vec![
                ClassHistogram {
                    class: 0,
                    live: 2,
                    peak: 2,
                    slack: 0
                },
                ClassHistogram {
                    class: 2,
                    live: 5,
                    peak: 7,
                    slack: 3
                },
            ],
            span: None,
        }),
        "{\"v\":6,\"type\":\"Histogram\",\"span\":null,\"classes\":[{\"class\":0,\"live\":2,\"peak\":2,\"slack\":0},{\"class\":2,\"live\":5,\"peak\":7,\"slack\":3}]}"
    );
}

/// The v0 SVG rendering is deterministic and shows levels, nodes and terminals.
#[test]
fn test_svg_static_deterministic() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let svg = render_svg(&pool, root);
    let svg2 = render_svg(&pool, root);
    assert_eq!(
        svg, svg2,
        "deterministic rendering (replayable identically)"
    );
    assert!(svg.starts_with("<svg "), "SVG document");
    assert!(svg.trim_end().ends_with("</svg>"));
    // Levels = keys: label ℓ0 for level 0.
    assert!(svg.contains("ℓ0"), "level 0 is labeled");
    // Acceptance terminal ⊤ rendered (⊥ is eliminated by R-default on this
    // set: the « absent atom » arc to ⊥ is implicit).
    assert!(svg.contains('⊤'), "acceptance terminal rendered");
    // At least one arc.
    assert!(svg.contains("<line "), "at least one arc rendered");
}

/// The SVG escapes XML (no raw `<` outside tags) — test via a standard
/// boolean payload: we mostly check the balanced structure.
#[test]
fn test_svg_structure_balanced() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let root = set(&mut pool, Anchor(0), &schema, &[1]);
    let svg = render_svg(&pool, root);
    let ouvertures = svg.matches("<rect ").count();
    let fermetures_rect = svg.matches("/>").count();
    assert!(ouvertures >= 2, "node + terminal rendered");
    assert!(fermetures_rect >= ouvertures, "self-closing tags present");
}

/// D1-3: a filtered JSONL observer forces the lazy `TraceVal` values only
/// for the events that pass the filter (elision).
#[test]
fn test_jsonl_observer_filter_and_elision() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use lisage_core::NodeId;
    use lisage_core::observer::{ChannelAttr, Observer, TraceEvent, TraceVal};
    use lisage_liseuse::trace::JsonlObserver;

    let counter = Arc::new(AtomicU64::new(0));
    let counter_clone = counter.clone();
    let value_lazy = TraceVal::Fragment(Arc::new(move || {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        TraceVal::text("calcule")
    }));

    let observer = JsonlObserver::with_filter(Arc::new(|e: &TraceEvent| {
        matches!(e, TraceEvent::StructureChange { .. })
    }));
    // Rejected by the filter: the lazy value is never forced.
    observer.on_event(&TraceEvent::Attr {
        channel: ChannelAttr::Inherited,
        node: NodeId(1),
        value: value_lazy.clone(),
        span: None,
    });
    assert_eq!(counter.load(Ordering::Relaxed), 0);
    assert!(observer.lines().is_empty());

    // Accepted: the value is forced exactly once, at serialization.
    observer.on_event(&TraceEvent::StructureChange {
        node: NodeId(1),
        classification: lisage_core::observer::StructureChangeKind::Identity,
        arcs_before: vec![],
        arcs_after: vec![],
        span: None,
    });
    assert_eq!(observer.lines().len(), 1);
    assert!(observer.lines()[0].contains("\"classification\":\"identite\""));
    assert_eq!(
        counter.load(Ordering::Relaxed),
        0,
        "the accepted event carries no lazy value"
    );

    // An accepted event carrying a lazy value forces it once.
    observer.on_event(&TraceEvent::Attr {
        channel: ChannelAttr::Memory,
        node: NodeId(2),
        value: value_lazy,
        span: None,
    });
    // The filter rejects Attr: still not forced.
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    let all = JsonlObserver::new();
    all.on_event(&TraceEvent::Attr {
        channel: ChannelAttr::Memory,
        node: NodeId(2),
        value: TraceVal::Fragment(Arc::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            TraceVal::text("calcule")
        })),
        span: None,
    });
    assert_eq!(
        all.lines()[0],
        "{\"v\":6,\"type\":\"Attr\",\"span\":null,\"canal\":\"memoire\",\"noeud\":2,\"valeur\":\"calcule\"}"
    );
}
