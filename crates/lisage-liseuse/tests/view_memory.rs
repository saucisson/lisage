//! Memory and quotient view tests (M4-5): pools + GC, multiplicity
//! badges, quotient classes (forms × anchors, C1: same form
//! under two anchors = two classes), naive/DD/Lisage comparison, static
//! SVG rendering.

use lisage_core::arena::Arena;
use lisage_core::biplan::Plan;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiler};
use lisage_core::{Anchor, Form, NodeId};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::view_memory::{
    ClassQuotient, ComparaisonView, ViewPool, rendre_svg_view_memory,
};

/// Test anchor.
const ANCHOR_TEST: Anchor = Anchor(0);
/// Second anchor (C1: same form, other anchor = other handle).
const ANCHOR_TEST_2: Anchor = Anchor(1);

type PoolRef = Arena<ReferenceController>;

fn schema_set() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]))
}

fn pool_neuf(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

fn set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atomes: &[u64]) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atomes.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M4-5: the pool view reflects the GC metrics (live, dead, gc,
/// compactions) and the live nodes per plane after a GC.
#[test]
fn test_view_pool_depuis_metriques_gc() {
    let schema = schema_set();
    let mut pool = pool_neuf(&schema);
    let root = set(&mut pool, ANCHOR_TEST, &schema, &[0, 1]);
    // Orphan collected by the GC.
    let _orphelin = set(&mut pool, ANCHOR_TEST, &schema, &[2]);
    let stats_gc = pool.gc(&[root]);
    let metriques = pool.metriques();
    let view = ViewPool::depuis_metriques("reference", metriques, stats_gc.live, 0);
    assert_eq!(view.nom, "reference");
    assert_eq!(view.metriques.gc, 1);
    assert_eq!(view.metriques.live, stats_gc.live);
    assert_eq!(view.metriques.morts, stats_gc.collectes);
    assert_eq!(view.live_principal, stats_gc.live);
    assert_eq!(view.live_ombre, 0);
}

/// M4-5: quotient classes (forms × anchors) — C1 in action: the same
/// form interned under two different anchors forms two distinct classes.
#[test]
fn test_classes_quotient_forms_par_anchors() {
    let schema = schema_set();
    let mut pool = pool_neuf(&schema);
    let handle_anchor_0 = set(&mut pool, ANCHOR_TEST, &schema, &[0]);
    let handle_anchor_1 = set(&mut pool, ANCHOR_TEST_2, &schema, &[0]);
    assert_ne!(
        handle_anchor_0, handle_anchor_1,
        "C1: same form, different anchors = distinct handles"
    );
    // Two classes (form {0}, anchor 0) and (form {0}, anchor 1).
    let classes = [
        ClassQuotient {
            form: "{0}".to_string(),
            anchor: "anchor 0".to_string(),
            multiplicite: 1,
        },
        ClassQuotient {
            form: "{0}".to_string(),
            anchor: "anchor 1".to_string(),
            multiplicite: 1,
        },
    ];
    // The two classes are distinct (C1): forms × anchors.
    assert_ne!(classes[0], classes[1]);
    // Multiplicity badge: a handle shared 3 times under the same anchor.
    let partage = set(&mut pool, ANCHOR_TEST, &schema, &[0]);
    assert_eq!(
        partage, handle_anchor_0,
        "R-share: same handle under the same anchor"
    );
    let class = ClassQuotient {
        form: "{0}".to_string(),
        anchor: "anchor 0".to_string(),
        multiplicite: 3,
    };
    let badge = class.badge();
    assert_eq!(badge.multiplicite, class.multiplicite);
    assert_eq!(badge.handle, "{0} @ anchor 0");
}

/// M4-5: SVG rendering of the memory and quotient view — panels present,
/// ×k badge, comparison bars with proportional width (real parsed coordinate,
/// not a byte offset), determinism.
#[test]
fn test_rendre_svg_view_memory() {
    let schema = schema_set();
    let mut pool = pool_neuf(&schema);
    let root = set(&mut pool, ANCHOR_TEST, &schema, &[0, 1]);
    let _orphelin = set(&mut pool, ANCHOR_TEST, &schema, &[2]);
    let stats_gc = pool.gc(&[root]);
    let view_pool = ViewPool::depuis_metriques("reference", pool.metriques(), stats_gc.live, 0);
    let classes = vec![
        ClassQuotient {
            form: "{0,1}".to_string(),
            anchor: "anchor 0".to_string(),
            multiplicite: 1,
        },
        ClassQuotient {
            form: "{2}".to_string(),
            anchor: "anchor 0".to_string(),
            multiplicite: 6,
        },
    ];
    let comparaison = ComparaisonView {
        naif: 152,
        dd: 15,
        lisage: 7,
    };
    let svg = rendre_svg_view_memory(&[view_pool], &classes, &comparaison);
    // Panels and contents.
    assert!(svg.contains("memory and quotient view"));
    assert!(svg.contains("pools (main / shadow plane, GC)"));
    assert!(svg.contains("quotient classes (forms × anchors)"));
    assert!(svg.contains("comparison naive / DD / Lisage"));
    assert!(svg.contains("reference : alive="));
    assert!(svg.contains("{2} @ anchor 0"));
    // Badge ×6 present (k > 1).
    assert!(svg.contains("×6"), "multiplicity badge ×6");
    // No badge for the class of multiplicity 1.
    assert!(!svg.contains("×1"));
    // Bars: width of the Lisage bar parsed from the SVG (real
    // coordinate) — proportional to 7/152 of the max width.
    let width_lisage = svg
        .lines()
        .find_map(|ligne| {
            if ligne.contains("fill=\"#2a7\"") {
                let debut = ligne.find("width=\"").unwrap() + 7;
                let fin = ligne[debut..].find('"').unwrap() + debut;
                ligne[debut..fin].parse::<u32>().ok()
            } else {
                None
            }
        })
        .expect("Lisage bar present");
    let expected = 7 * 400 / 152;
    assert_eq!(width_lisage, expected, "proportional Lisage bar");
    // Determinism.
    let svg2 = rendre_svg_view_memory(
        &[ViewPool::depuis_metriques(
            "reference",
            pool.metriques(),
            stats_gc.live,
            0,
        )],
        &classes,
        &comparaison,
    );
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// M4-5: naive/DD/Lisage comparison — the Lisage mode shows the lowest
/// count (shared forms), the naive bar is the widest.
#[test]
fn test_comparaison_naif_dd_lisage() {
    let comparaison = ComparaisonView {
        naif: 536_754_176,
        dd: 15,
        lisage: 7,
    };
    let svg = rendre_svg_view_memory(&[], &[], &comparaison);
    assert!(svg.contains("536754176"));
    assert!(svg.contains("Lisage"));
    // The naive bar reaches the max width (it is the max).
    let width_naif = svg
        .lines()
        .find_map(|ligne| {
            if ligne.contains("fill=\"#999\"") {
                let debut = ligne.find("width=\"").unwrap() + 7;
                let fin = ligne[debut..].find('"').unwrap() + debut;
                ligne[debut..fin].parse::<u32>().ok()
            } else {
                None
            }
        })
        .expect("naive bar present");
    assert_eq!(width_naif, 400, "the max reaches the max width");
    // Shadow plane: a node interned in the shadow plane is counted apart.
    let schema = schema_set();
    let mut pool = pool_neuf(&schema);
    let root = set(&mut pool, ANCHOR_TEST, &schema, &[0]);
    let ombre = pool.intern_plan(
        ANCHOR_TEST,
        Form::Node {
            level: lisage_core::Level(1),
            arcs: Default::default(),
        },
        Plan::Ombre,
    );
    assert_eq!(pool.plan_de(ombre), Some(Plan::Ombre));
    assert_eq!(pool.plan_de(root), Some(Plan::Principal));
}
