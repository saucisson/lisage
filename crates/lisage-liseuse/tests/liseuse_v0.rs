//! Liseuse v0 tests (M1-9): versioned JSONL serializer and static SVG
//! rendering of the diagram (levels = keys).

use std::collections::BTreeSet;

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::svg::rendre_svg;
use lisage_liseuse::trace::{JsonlObserver, TRACE_VERSION, serialiser_evenement};

type PoolRef = Arena<ReferenceController>;

fn schema_ensemble() -> Schema {
    Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]))
}

fn ensemble(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    atomes: &[u64],
) -> lisage_core::NodeId {
    let valeur = Value::Set(
        atomes
            .iter()
            .map(|a| Value::Atom(Atom(*a)))
            .collect::<BTreeSet<_>>(),
    );
    compiler(pool, anchor, schema, &valeur).unwrap()
}

/// The JSONL is versioned, one line = one event, and the trace is
/// deterministic (two identical constructions ⇒ same trace).
#[test]
fn test_trace_jsonl_versionnee_et_deterministe() {
    let schema = schema_ensemble();
    let construire = |observateur: &JsonlObserver| {
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(observateur.clone()),
        );
        ensemble(&mut pool, Anchor(0), &schema, &[0])
    };
    let observateur_a = JsonlObserver::new();
    let observateur_b = JsonlObserver::new();
    let racine_a = construire(&observateur_a);
    let racine_b = construire(&observateur_b);
    assert_eq!(racine_a, racine_b, "C1: same construction ⇒ same root");
    assert_eq!(
        observateur_a.lignes(),
        observateur_b.lignes(),
        "deterministic trace (replayable identically)"
    );

    let lignes = observateur_a.lignes();
    assert!(!lignes.is_empty());
    for ligne in &lignes {
        assert!(
            ligne.starts_with(&format!("{{\"v\":{TRACE_VERSION},")),
            "each line carries the schema version: {ligne}"
        );
        assert!(ligne.ends_with('}'));
    }
    // The acceptance terminal is created first.
    assert!(
        lignes[0].contains("\"type\":\"NodeCreated\"") && lignes[0].contains("\"level\":null"),
        "terminal = null level: {}",
        lignes[0]
    );
    // The level-0 node exists in the trace.
    assert!(
        lignes.iter().any(|l| l.contains("\"level\":0")),
        "a level-0 node was created"
    );
}

/// Exact (golden) serialization of one event of each type.
#[test]
fn test_serialiser_evenement_golden() {
    use lisage_core::observer::{ReductionRule, TraceEvent};
    use lisage_core::{ControllerId, Handle, Level, NodeId};

    assert_eq!(
        serialiser_evenement(&TraceEvent::NodeCreated {
            node: NodeId(3),
            level: Some(Level(2))
        }),
        "{\"v\":2,\"type\":\"NodeCreated\",\"node\":3,\"level\":2}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::NodeShared { node: NodeId(7) }),
        "{\"v\":2,\"type\":\"NodeShared\",\"node\":7}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level: Level(1)
        }),
        "{\"v\":2,\"type\":\"Reduction\",\"rule\":\"R-collapse\",\"level\":1}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::DiagramBuilt {
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
fn test_svg_statique_deterministe() {
    let schema = schema_ensemble();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let racine = ensemble(&mut pool, anchor, &schema, &[0, 1]);

    let svg = rendre_svg(&pool, racine);
    let svg2 = rendre_svg(&pool, racine);
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
fn test_svg_structure_equilibree() {
    let schema = schema_ensemble();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let racine = ensemble(&mut pool, Anchor(0), &schema, &[1]);
    let svg = rendre_svg(&pool, racine);
    let ouvertures = svg.matches("<rect ").count();
    let fermetures_rect = svg.matches("/>").count();
    assert!(ouvertures >= 2, "node + terminal rendered");
    assert!(fermetures_rect >= ouvertures, "self-closing tags present");
}
