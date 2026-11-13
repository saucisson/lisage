//! Tests of the Reader v0 (M1-9): versioned JSONL serializer and static SVG
//! rendering of the diagram (levels = keys).

use std::collections::BTreeSet;

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiled};
use lisage_lang::value::{Atom, Value};
use lisage_reader::svg::render_svg;
use lisage_reader::trace::{JsonlObserver, TRACE_VERSION, serialize_event};

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
    compiled(pool, anchor, schema, &value).unwrap()
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
    // The accepting terminal is created first.
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
    use lisage_core::observer::{ReductionRule, TraceEvent};
    use lisage_core::{ControllerId, Handle, Level, NodeId};

    assert_eq!(
        serialize_event(&TraceEvent::NodeCreated {
            node: NodeId(3),
            level: Some(Level(2))
        }),
        "{\"v\":2,\"type\":\"NodeCreated\",\"node\":3,\"level\":2}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::NodeShared { node: NodeId(7) }),
        "{\"v\":2,\"type\":\"NodeShared\",\"node\":7}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level: Level(1)
        }),
        "{\"v\":2,\"type\":\"Reduction\",\"rule\":\"R-collapse\",\"level\":1}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::DiagramBuilt {
            handle: Handle {
                controller: ControllerId(1),
                anchor: Anchor(0),
                node: NodeId(4)
            },
            nodes: 5
        }),
        "{\"v\":2,\"type\":\"DiagramBuilt\",\"controller\":1,\"anchor\":0,\"node\":4,\"nodes\":5}"
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
    // Accepting terminal ⊤ rendered (⊥ is eliminated by R-default on this
    // set: the « absent atom » edge to ⊥ is implicit).
    assert!(svg.contains('⊤'), "accepting terminal rendered");
    // At least one edge.
    assert!(svg.contains("<line "), "at least one edge rendered");
}

/// The SVG escapes XML (no raw `<` outside tags) — tested via a standard
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
    let closures_rect = svg.matches("/>").count();
    assert!(ouvertures >= 2, "node + terminal rendered");
    assert!(closures_rect >= ouvertures, "self-closing tags present");
}
