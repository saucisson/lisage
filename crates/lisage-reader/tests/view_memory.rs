//! Tests of the memory and quotient view (M4-5): pools + GC, multiplicity
//! badges, quotient classes (forms × anchors, C1: same form
//! under two anchors = two classes), naive/DD/Lisage comparison, static
//! SVG rendering.

use lisage_core::arena::Arena;
use lisage_core::biplan::Plan;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiled};
use lisage_core::{Anchor, Form, NodeId};
use lisage_lang::value::{Atom, Value};
use lisage_reader::view_memory::{
    ClasseQuotient, ComparisonView, ViewPool, render_svg_view_memory,
};

/// Test anchor.
const ANCRE_TEST: Anchor = Anchor(0);
/// Second anchor (C1: same form, other anchor = other handle).
const ANCRE_TEST_2: Anchor = Anchor(1);

type PoolRef = Arena<ReferenceController>;

fn schema_set() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]))
}

fn pool_nine(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

fn set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> NodeId {
    compiled(
        pool,
        anchor,
        schema,
        &Value::Set(atoms.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M4-5: the pool view reflects the GC metrics (live, dead, gc,
/// compactions) and the live counts per plane after a GC.
#[test]
fn test_view_pool_since_metrics_gc() {
    let schema = schema_set();
    let mut pool = pool_nine(&schema);
    let root = set(&mut pool, ANCRE_TEST, &schema, &[0, 1]);
    // Orphan collected by the GC.
    let _orphelin = set(&mut pool, ANCRE_TEST, &schema, &[2]);
    let stats_gc = pool.gc(&[root]);
    let metrics = pool.metrics();
    let view = ViewPool::since_metrics("reference", metrics, stats_gc.live, 0);
    assert_eq!(view.name, "reference");
    assert_eq!(view.metrics.gc, 1);
    assert_eq!(view.metrics.live, stats_gc.live);
    assert_eq!(view.metrics.dead, stats_gc.collected);
    assert_eq!(view.live_main, stats_gc.live);
    assert_eq!(view.live_shadow, 0);
}

/// M4-5: quotient classes (forms × anchors) — C1 in action: the same
/// form interned under two different anchors forms two distinct classes.
#[test]
fn test_classes_quotient_forms_by_anchors() {
    let schema = schema_set();
    let mut pool = pool_nine(&schema);
    let handle_anchor_0 = set(&mut pool, ANCRE_TEST, &schema, &[0]);
    let handle_anchor_1 = set(&mut pool, ANCRE_TEST_2, &schema, &[0]);
    assert_ne!(
        handle_anchor_0, handle_anchor_1,
        "C1: same form, different anchors = distinct handles"
    );
    // Two classes (form {0}, anchor 0) and (form {0}, anchor 1).
    let classes = [
        ClasseQuotient {
            form: "{0}".to_string(),
            anchor: "anchor 0".to_string(),
            multiplicite: 1,
        },
        ClasseQuotient {
            form: "{0}".to_string(),
            anchor: "anchor 1".to_string(),
            multiplicite: 1,
        },
    ];
    // The two classes are distinct (C1): forms × anchors.
    assert_ne!(classes[0], classes[1]);
    // Multiplicity badge: a handle shared 3 times under the same anchor.
    let shared = set(&mut pool, ANCRE_TEST, &schema, &[0]);
    assert_eq!(
        shared, handle_anchor_0,
        "R-share: same handle under the same anchor"
    );
    let classe = ClasseQuotient {
        form: "{0}".to_string(),
        anchor: "anchor 0".to_string(),
        multiplicite: 3,
    };
    let badge = classe.badge();
    assert_eq!(badge.multiplicite, classe.multiplicite);
    assert_eq!(badge.handle, "{0} @ anchor 0");
}

/// M4-5: SVG rendering of the memory and quotient view — panels present,
/// ×k badge, comparison bars with proportional width (real coordinate parsed,
/// not a byte offset), determinism.
#[test]
fn test_render_svg_view_memory() {
    let schema = schema_set();
    let mut pool = pool_nine(&schema);
    let root = set(&mut pool, ANCRE_TEST, &schema, &[0, 1]);
    let _orphelin = set(&mut pool, ANCRE_TEST, &schema, &[2]);
    let stats_gc = pool.gc(&[root]);
    let view_pool = ViewPool::since_metrics("reference", pool.metrics(), stats_gc.live, 0);
    let classes = vec![
        ClasseQuotient {
            form: "{0,1}".to_string(),
            anchor: "anchor 0".to_string(),
            multiplicite: 1,
        },
        ClasseQuotient {
            form: "{2}".to_string(),
            anchor: "anchor 0".to_string(),
            multiplicite: 6,
        },
    ];
    let comparison = ComparisonView {
        naive: 152,
        dd: 15,
        lisage: 7,
    };
    let svg = render_svg_view_memory(&[view_pool], &classes, &comparison);
    // Panels and contents.
    assert!(svg.contains("memory and quotient view"));
    assert!(svg.contains("pools (main / shadow plane, GC)"));
    assert!(svg.contains("quotient classes (forms × anchors)"));
    assert!(svg.contains("naive / DD / Lisage comparison"));
    assert!(svg.contains("reference : live="));
    assert!(svg.contains("{2} @ anchor 0"));
    // ×6 badge present (k > 1).
    assert!(svg.contains("×6"), "multiplicity badge ×6");
    // No badge for the multiplicity-1 class.
    assert!(!svg.contains("×1"));
    // Bars: width of the Lisage bar parsed from the SVG (real
    // coordinate) — proportional to 7/152 of the max width.
    let width_lisage = svg
        .lines()
        .find_map(|line| {
            if line.contains("fill=\"#2a7\"") {
                let start = line.find("width=\"").unwrap() + 7;
                let end = line[start..].find('"').unwrap() + start;
                line[start..end].parse::<u32>().ok()
            } else {
                None
            }
        })
        .expect("Lisage bar present");
    let expected = 7 * 400 / 152;
    assert_eq!(width_lisage, expected, "Lisage bar proportional");
    // Determinism.
    let svg2 = render_svg_view_memory(
        &[ViewPool::since_metrics(
            "reference",
            pool.metrics(),
            stats_gc.live,
            0,
        )],
        &classes,
        &comparison,
    );
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// M4-5: naive/DD/Lisage comparison — the Lisage mode shows the lowest
/// count (shared forms), the naive bar is the widest.
#[test]
fn test_comparison_naive_dd_lisage() {
    let comparison = ComparisonView {
        naive: 536_754_176,
        dd: 15,
        lisage: 7,
    };
    let svg = render_svg_view_memory(&[], &[], &comparison);
    assert!(svg.contains("536754176"));
    assert!(svg.contains("Lisage"));
    // The naive bar reaches the max width (it is the max).
    let width_naive = svg
        .lines()
        .find_map(|line| {
            if line.contains("fill=\"#999\"") {
                let start = line.find("width=\"").unwrap() + 7;
                let end = line[start..].find('"').unwrap() + start;
                line[start..end].parse::<u32>().ok()
            } else {
                None
            }
        })
        .expect("naive bar present");
    assert_eq!(width_naive, 400, "the max reaches the max width");
    // Shadow plane: a node interned in the shadow plane is counted apart.
    let schema = schema_set();
    let mut pool = pool_nine(&schema);
    let root = set(&mut pool, ANCRE_TEST, &schema, &[0]);
    let shadow = pool.intern_plan(
        ANCRE_TEST,
        Form::Node {
            level: lisage_core::Level(1),
            edges: Default::default(),
        },
        Plan::Shadow,
    );
    assert_eq!(pool.plan_of(shadow), Some(Plan::Shadow));
    assert_eq!(pool.plan_of(root), Some(Plan::Main));
}
