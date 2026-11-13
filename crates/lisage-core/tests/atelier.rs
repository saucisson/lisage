//! Tests of the reduction dashboard (didactic workshop, backlog § 9)
//! : commutation of R-collapse and R-default via `WorkshopController`, and
//! didactic metrics — shared nodes (R-share) against paths (unfolded
//! DAG size). Pool errors are propagated, never swallowed.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compile_bdd};
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{EMPTY, Interval, RegionController, RegionKeys};
use lisage_core::workshop::{
    Reductions, WorkshopController, count_nodes, count_paths, count_size_unfolded, measure,
};

type PoolBdd = Arena<WorkshopController<BddController>>;
type PoolRegion = Arena<WorkshopController<RegionController>>;

const ANCHOR: Anchor = Anchor(0);

fn pool_bdd(reductions: Reductions) -> PoolBdd {
    Arena::new(
        WorkshopController::new(BddController::new(2), reductions),
        Box::new(NoOpObserver),
    )
}

fn pool_region(reductions: Reductions) -> PoolRegion {
    Arena::new(
        WorkshopController::new(RegionController::new(RegionKeys::new(vec![3])), reductions),
        Box::new(NoOpObserver),
    )
}

/// R-collapse commuted: a node whose edges all lead to the same target
/// collapses to that target; without R-collapse, the node is kept.
#[test]
fn test_collapse_commutes() {
    // BDD 1 variable : {0 → ⊥, 1 → ⊥}.
    let mut with = pool_bdd(Reductions::ALL);
    let false_node = with.intern(ANCHOR, Form::Terminal(false));
    let root_with = with.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, false_node), (1, false_node)]),
        },
    );
    assert_eq!(root_with, false_node, "R-collapse: the node reduces to ⊥");
    let report = measure(&with, root_with, None).expect("interned node");
    assert_eq!(report.nodes_shared, 1);
    assert_eq!(report.paths, 1);

    let mut without = pool_bdd(Reductions::without_collapse());
    let false_node = without.intern(ANCHOR, Form::Terminal(false));
    let root_without = without.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, false_node), (1, false_node)]),
        },
    );
    assert_ne!(
        root_without, false_node,
        "without R-collapse: the node is kept"
    );
    let report = measure(&without, root_without, None).expect("interned node");
    assert_eq!(report.nodes_shared, 2, "node + terminal ⊥");
    assert_eq!(report.paths, 2, "two paths to ⊥");
}

/// R-default commuted: the explicit δ(ℓ) edge to D_δ is absorbed or not.
#[test]
fn test_default_commutes() {
    // Region over domain [0, 3) : {point 0 → ⊤, EMPTY → ⊥}.
    let mut with = pool_region(Reductions::ALL);
    let true_node = with.intern(ANCHOR, Form::Terminal(true));
    let false_node = with.intern(ANCHOR, Form::Terminal(false));
    let root_with = with.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(Interval::point(0), true_node), (EMPTY, false_node)]),
        },
    );
    let Form::Node { edges, .. } = with.resolve(root_with).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(edges.len(), 1, "R-default: the EMPTY edge to ⊥ is absorbed");
    let report = measure(&with, root_with, None).expect("interned node");
    assert_eq!(report.nodes_shared, 2, "node + ⊤");
    assert_eq!(report.paths, 1);

    let mut without = pool_region(Reductions::without_default());
    let true_node = without.intern(ANCHOR, Form::Terminal(true));
    let false_node = without.intern(ANCHOR, Form::Terminal(false));
    let root_without = without.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(Interval::point(0), true_node), (EMPTY, false_node)]),
        },
    );
    let Form::Node { edges, .. } = without.resolve(root_without).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(edges.len(), 2, "without R-default: the EMPTY edge is kept");
    let report = measure(&without, root_without, None).expect("interned node");
    assert_eq!(report.nodes_shared, 3, "node + ⊤ + ⊥");
    assert_eq!(report.paths, 2);
}

/// Metrics on a diamond: 4 shared nodes (root, n1, ⊤, ⊥) and 4
/// paths — each root × n1 edge combination is a path of the unfolded
/// DAG, even when the sub-paths are shared.
#[test]
fn test_paths_against_nodes() {
    let mut pool = pool_bdd(Reductions::without_collapse());
    let true_node = pool.intern(ANCHOR, Form::Terminal(true));
    let false_node = pool.intern(ANCHOR, Form::Terminal(false));
    let n1 = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(1),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    let root = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, n1), (1, n1)]),
        },
    );
    assert_eq!(count_nodes(&pool, root).expect("interned node"), 4);
    assert_eq!(count_paths(&pool, root).expect("interned node"), 4);
    // Unfolded size: root (1) + 2 × n1 (1 + ⊤ + ⊥ = 3) = 7.
    assert_eq!(count_size_unfolded(&pool, root).expect("interned node"), 7);
    // A not-interned NodeId is a propagated error.
    assert!(count_nodes(&pool, NodeId(u32::MAX)).is_err());
    assert!(count_paths(&pool, NodeId(u32::MAX)).is_err());
    assert!(count_size_unfolded(&pool, NodeId(u32::MAX)).is_err());
    assert!(measure(&pool, NodeId(u32::MAX), None).is_err());
}

/// Property (LCG): on random BDDs, disabling the reductions can only
/// increase (or leave equal) shared nodes and paths — the same
/// form is rebuilt in both pools (same seed).
#[test]
fn test_property_commutation_lcg() {
    const GRAINE_LCG: u64 = 0x0A7E_11E5_2026_1110;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    let mut next = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    // Builds a random binary tree of depth ≤ 3 in the pool,
    // with its own LCG re-seeded to `amorce` (same form guaranteed).
    fn tree(pool: &mut PoolBdd, depth: u32, amorce: u64) -> NodeId {
        let mut graine = amorce;
        fn next(graine: &mut u64) -> u64 {
            *graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            *graine
        }
        fn construit(pool: &mut PoolBdd, depth: u32, graine: &mut u64) -> NodeId {
            if depth == 0 || !next(graine).is_multiple_of(3) {
                return pool.intern(ANCHOR, Form::Terminal(next(graine) % 2 == 1));
            }
            let left = construit(pool, depth - 1, graine);
            let right = construit(pool, depth - 1, graine);
            pool.intern(
                ANCHOR,
                Form::Node {
                    level: Level(depth),
                    edges: BTreeMap::from([(0, left), (1, right)]),
                },
            )
        }
        construit(pool, depth, &mut graine)
    }
    for _ in 0..NUMBER_CAS {
        let amorce = next();
        let mut with = pool_bdd(Reductions::ALL);
        let root_with = tree(&mut with, 3, amorce);
        let mut without = pool_bdd(Reductions::NO);
        let root_without = tree(&mut without, 3, amorce);
        let report_with = measure(&with, root_with, None).expect("interned node");
        let report_without = measure(&without, root_without, None).expect("interned node");
        assert!(report_with.paths >= 1, "at least one path");
        assert!(
            report_without.nodes_shared >= report_with.nodes_shared,
            "without reduction: nodes ≥ (same form)"
        );
        assert!(
            report_without.paths >= report_with.paths,
            "without reduction: paths ≥ (same form)"
        );
    }
}

/// Narration (ADR-0100 decision 6): `measure` emits an
/// `Attribute`/`Synthesize` event carrying the full report — pinned by a
/// collector observer.
#[test]
fn test_narration_measure() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttribute, TraceEvent};

    #[derive(Clone)]
    struct Collecteur {
        events: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.events.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::without_collapse());
    let true_node = pool.intern(ANCHOR, Form::Terminal(true));
    let false_node = pool.intern(ANCHOR, Form::Terminal(false));
    let n1 = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(1),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    let root = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, n1), (1, n1)]),
        },
    );
    let collecteur = Collecteur {
        events: Arc::new(Mutex::new(Vec::new())),
    };
    let report = measure(&pool, root, Some(&collecteur)).expect("interned node");
    assert_eq!(report.nodes_shared, 4);
    let events = collecteur.events.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                value,
                ..
            } if value == "nodes_shared=4 paths=4 size_unfolded=7"
        )),
        "the narration event carries the full report"
    );
}

/// Sliding window, absolute against relative encoding (controller
/// parameter, backlog § 9): same paths (same logical object), but
/// the absolute encoding creates one node per base while the relative one shares a single
/// form — the "4 against infinity" of the window, measured.
#[test]
fn test_window_absolute_grows_relative_constant() {
    use lisage_core::workshop::compare_encodings_window;

    const SIZE: u64 = 4;
    // 10 bases: absolute = root + 10 windows + ⊤ = 12 nodes ; relative = 3.
    // (R-collapse commuted: an all-true window would otherwise collapse.)
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report =
        compare_encodings_window(&mut pool, ANCHOR, SIZE, 10, None).expect("interned nodes");
    assert_eq!(report.absolute.nodes_shared, 12);
    assert_eq!(report.relative.nodes_shared, 3);
    // Same logical object: same paths (10 bases × 4 keys) and same
    // unfolded size (root + 10 × (window + ⊤)).
    assert_eq!(report.absolute.paths, 40);
    assert_eq!(report.relative.paths, 40);
    assert_eq!(report.absolute.size_unfolded, 51);
    assert_eq!(report.relative.size_unfolded, 51);

    // 100 bases: the relative stays at 3 nodes (constant), the absolute grows to 102.
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report =
        compare_encodings_window(&mut pool, ANCHOR, SIZE, 100, None).expect("interned nodes");
    assert_eq!(report.relative.nodes_shared, 3, "the relative is O(1)");
    assert_eq!(
        report.absolute.nodes_shared, 102,
        "the absolute grows with the bases"
    );
    assert_eq!(report.relative.paths, 400);
}

/// Narration (ADR-0100): the encoding comparison emits an event
/// `Attribute`/`Synthesize` carrying the two node counts.
#[test]
fn test_narration_comparison_encodings() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttribute, TraceEvent};
    use lisage_core::workshop::compare_encodings_window;

    #[derive(Clone)]
    struct Collecteur {
        events: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.events.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::without_collapse());
    let collecteur = Collecteur {
        events: Arc::new(Mutex::new(Vec::new())),
    };
    let report = compare_encodings_window(&mut pool, ANCHOR, 4, 10, Some(&collecteur))
        .expect("interned nodes");
    assert_eq!(report.relative.nodes_shared, 3);
    let events = collecteur.events.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                value,
                ..
            } if value == "encoding=relative_vs_absolute nodes=3 vs 12"
        )),
        "the narration event carries the encoding comparison"
    );
}

/// Truth table of f = (x₀ ∧ x₁) ∨ (x₂ ∧ x₃) over 4 variables:
/// `table[a]` = bit₀(a) ∧ bit₁(a) ∨ bit₂(a) ∧ bit₃(a).
fn table_or_two_pairs() -> Vec<bool> {
    (0..16)
        .map(|a| {
            let bit = |v: u32| (a >> v) & 1 == 1;
            (bit(0) && bit(1)) || (bit(2) && bit(3))
        })
        .collect()
}

/// Order parameter (backlog § 9): the same function compiled under two
/// orders has the same paths (2^n, one per assignment — same logical
/// object) and the same unfolded size (the full tree 2^{n+1}-1), but
/// different shared nodes — the order changes the diagram, not the
/// function. (R-collapse commuted: otherwise the paths would merge.)
/// Natural order [0,1,2,3]: 10 nodes ; interleaved [0,2,1,3]: 12 nodes.
#[test]
fn test_order_changed_nodes_not_paths() {
    use lisage_core::workshop::{Order, compare_orders};

    let table = table_or_two_pairs();
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report = compare_orders(
        &mut pool,
        ANCHOR,
        &table,
        &Order::naturel(4),
        &Order::personnalise(vec![0, 2, 1, 3]),
        None,
    )
    .expect("interned nodes");
    assert_eq!(report.first.nodes_shared, 10, "natural order");
    assert_eq!(report.second.nodes_shared, 12, "interleaved order");
    // Same logical object: 2^4 = 16 paths and unfolded tree 2^5-1 = 31.
    assert_eq!(report.first.paths, 16);
    assert_eq!(report.second.paths, 16);
    assert_eq!(report.first.size_unfolded, 31);
    assert_eq!(report.second.size_unfolded, 31);
}

/// The workshop's natural order is exactly the reference `compile_bdd`
/// (same interned forms, same `NodeId` in the same pool).
#[test]
fn test_order_naturel_equals_a_compile_bdd() {
    use lisage_core::workshop::{Order, compile_bdd_order};

    let table = table_or_two_pairs();
    let mut pool = pool_bdd(Reductions::ALL);
    let via_workshop = compile_bdd_order(&mut pool, ANCHOR, &Order::naturel(4), &table);
    let via_bdd = compile_bdd(&mut pool, ANCHOR, &BddKeys::new(4), &table);
    assert_eq!(via_workshop, via_bdd);
}

/// Didactic growth: over 6 variables, f = (x₀∧x₁)∨(x₂∧x₃)∨(x₄∧x₅),
/// the natural order groups the pairs (15 nodes), the interleaved order
/// [0,2,4,1,3,5] separates them (25 nodes) — same 64 paths on both sides.
#[test]
fn test_order_interleaved_growth_six_variables() {
    use lisage_core::workshop::{Order, compare_orders};

    let table: Vec<bool> = (0..64)
        .map(|a| {
            let bit = |v: u32| (a >> v) & 1 == 1;
            (bit(0) && bit(1)) || (bit(2) && bit(3)) || (bit(4) && bit(5))
        })
        .collect();
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report = compare_orders(
        &mut pool,
        ANCHOR,
        &table,
        &Order::naturel(6),
        &Order::personnalise(vec![0, 2, 4, 1, 3, 5]),
        None,
    )
    .expect("interned nodes");
    assert_eq!(report.first.nodes_shared, 15, "natural order");
    assert_eq!(report.second.nodes_shared, 25, "interleaved order");
    assert_eq!(report.first.paths, 64);
    assert_eq!(report.second.paths, 64);
}

/// Narration (ADR-0100): the order comparison emits an event
/// `Attribute`/`Synthesize` carrying the two order labels and the two
/// node counts.
#[test]
fn test_narration_comparison_orders() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttribute, TraceEvent};
    use lisage_core::workshop::{Order, compare_orders};

    #[derive(Clone)]
    struct Collecteur {
        events: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.events.lock().expect("mutex").push(event.clone());
        }
    }

    let table = table_or_two_pairs();
    let mut pool = pool_bdd(Reductions::without_collapse());
    let collecteur = Collecteur {
        events: Arc::new(Mutex::new(Vec::new())),
    };
    let report = compare_orders(
        &mut pool,
        ANCHOR,
        &table,
        &Order::naturel(4),
        &Order::personnalise(vec![0, 2, 1, 3]),
        Some(&collecteur),
    )
    .expect("interned nodes");
    assert_eq!(report.first.nodes_shared, 10);
    let events = collecteur.events.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                value,
                ..
            } if value == "order=[0,1,2,3]_vs_[0,2,1,3] nodes=10 vs 12"
        )),
        "the narration event carries the order comparison"
    );
}

/// Composed anchor parameter (backlog § 9, SWP golden thread): the slide is
/// the displacement of the base in the anchor `{base, horizon}` ; the horizon does
/// not move and the absolute read of the window advances.
#[test]
fn test_anchor_window_slides() {
    use lisage_core::workshop::AnchorWindow;

    let anchor = AnchorWindow::new(0, 4);
    assert_eq!(anchor.base(), 0);
    assert_eq!(anchor.horizon(), 4);
    assert_eq!(anchor.window_absolute(), (0, 4));
    assert_eq!(
        anchor.key_input(),
        0,
        "the base is the argument of the input edge"
    );
    assert_eq!(anchor.label(), "{base: 0, horizon: 4}");
    let slid = anchor.glessee(2);
    assert_eq!(slid.base(), 2);
    assert_eq!(slid.horizon(), 4, "the slide only touches the base");
    assert_eq!(slid.window_absolute(), (2, 6));
    assert_eq!(slid.key_input(), 2);
    assert_eq!(slid.label(), "{base: 2, horizon: 4}");
    // The original anchor is unchanged (the slide produces a new anchor).
    assert_eq!(anchor.base(), 0);
}

/// The window read under two composed anchors: each anchor has its root
/// (the input edge carries its base), but both reach the same window
/// form (`same_form`) — same nodes, same paths, different
/// absolute reads: "the anchor moves, the pool never". During 50
/// slides, only the root is created at each step, never the form.
#[test]
fn test_slides_pool_constant_reads_different() {
    use lisage_core::workshop::{AnchorWindow, build_window_anchored, compare_anchors_window};

    const SIZE: u64 = 4;
    // (R-collapse commuted: an all-true window would otherwise collapse.)
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report = compare_anchors_window(
        &mut pool,
        ANCHOR,
        SIZE,
        &AnchorWindow::new(0, 4),
        &AnchorWindow::new(7, 4),
        None,
    )
    .expect("interned nodes");
    assert!(report.same_form, "both anchors reach the same window form");
    assert_eq!(report.before, report.after, "same metrics on both sides");
    assert_eq!(report.before.nodes_shared, 3, "root + window + ⊤");
    assert_eq!(report.before.paths, 4, "one path per window key");
    assert_eq!(report.before.size_unfolded, 6);
    assert_eq!(
        report.nodes_pool, 4,
        "⊤ + shared window + one root per anchor"
    );
    assert_eq!(report.read_before, (0, 4));
    assert_eq!(report.read_after, (7, 11));

    // 50 slides (bases 100..150, all new): at each step the base
    // enters through the input edge, the window form reached is always
    // the same and the pool creates only one root per slide — never a
    // copy of the window.
    let nodes_before = pool.node_count();
    let (_, window) = build_window_anchored(&mut pool, ANCHOR, SIZE, &AnchorWindow::new(0, 4));
    let anchor = AnchorWindow::new(100, SIZE as i64);
    for delta in 0..50 {
        let slid = anchor.glessee(delta);
        assert_eq!(
            slid.window_absolute(),
            (100 + delta, 100 + delta + SIZE as i64)
        );
        let (root, window_slid) = build_window_anchored(&mut pool, ANCHOR, SIZE, &slid);
        assert_eq!(window_slid, window, "window form shared at each slide");
        assert_ne!(root, window, "each base has its own root");
    }
    assert_eq!(
        pool.node_count(),
        nodes_before + 50,
        "a single root per slide: the window form is never duplicated"
    );
}

/// Property (LCG): the slide composes — sliding by `a` then by `b` equals
/// sliding by `a + b` — and the horizon is invariant (30 cases).
#[test]
fn test_property_slides_composition_lcg() {
    use lisage_core::workshop::AnchorWindow;

    const GRAINE_LCG: u64 = 0x5EED_2026_1112_4E11;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    let mut next = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    for _ in 0..NUMBER_CAS {
        let a = next() as i64 % 1000;
        let b = next() as i64 % 1000;
        let horizon = next() as i64 % 16 + 1;
        let anchor = AnchorWindow::new(0, horizon);
        assert_eq!(
            anchor.glessee(a).glessee(b),
            anchor.glessee(a + b),
            "the slide composes: f(a)∘f(b) = f(a+b)"
        );
        assert_eq!(
            anchor.glessee(a).horizon(),
            horizon,
            "the horizon is invariant"
        );
    }
}

/// Narration (ADR-0100): the composed-anchor comparison emits an
/// `Attribute`/`Synthesize` event carrying the two labels, the two
/// node counts and the two absolute reads.
#[test]
fn test_narration_comparison_anchors() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttribute, TraceEvent};
    use lisage_core::workshop::{AnchorWindow, compare_anchors_window};

    #[derive(Clone)]
    struct Collecteur {
        events: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.events.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::without_collapse());
    let collecteur = Collecteur {
        events: Arc::new(Mutex::new(Vec::new())),
    };
    let report = compare_anchors_window(
        &mut pool,
        ANCHOR,
        4,
        &AnchorWindow::new(0, 4),
        &AnchorWindow::new(2, 4),
        Some(&collecteur),
    )
    .expect("interned nodes");
    assert_eq!(report.before.nodes_shared, 3);
    let events = collecteur.events.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                value,
                ..
            } if value == "anchor={base: 0, horizon: 4}_vs_{base: 2, horizon: 4} nodes=3 vs 3 read=[0..4) vs [2..6) pool=4"
        )),
        "the narration event carries the anchor comparison"
    );
}
