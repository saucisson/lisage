//! Tests for the reduction dashboard (didactic workshop, backlog § 9):
//! commutation of R-collapse and R-default via `AtelierController`, and
//! didactic metrics — shared nodes (R-share) vs paths (size of the unfolded
//! DAG). Pool errors are propagated, never swallowed.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::atelier::{
    AtelierController, Reductions, count_nodes, count_paths, count_size_depliee, measure,
};
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd};
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{EMPTY, Interval, RegionController, RegionKeys};

type PoolBdd = Arena<AtelierController<BddController>>;
type PoolRegion = Arena<AtelierController<RegionController>>;

const ANCHOR: Anchor = Anchor(0);

fn pool_bdd(reductions: Reductions) -> PoolBdd {
    Arena::new(
        AtelierController::new(BddController::new(2), reductions),
        Box::new(NoOpObserver),
    )
}

fn pool_region(reductions: Reductions) -> PoolRegion {
    Arena::new(
        AtelierController::new(RegionController::new(RegionKeys::new(vec![3])), reductions),
        Box::new(NoOpObserver),
    )
}

/// Commuted R-collapse: a node whose arcs all lead to the same target
/// collapses into that target; without R-collapse, the node is kept.
#[test]
fn test_collapse_commute() {
    // 1-variable BDD: {0 → ⊥, 1 → ⊥}.
    let mut with = pool_bdd(Reductions::TOUTES);
    let faux = with.intern(ANCHOR, Form::Terminal(false));
    let root_with = with.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, faux), (1, faux)]),
        },
    );
    assert_eq!(root_with, faux, "R-collapse: the node reduces to ⊥");
    let report = measure(&with, root_with, None).expect("interned node");
    assert_eq!(report.nodes_partages, 1);
    assert_eq!(report.paths, 1);

    let mut without = pool_bdd(Reductions::without_collapse());
    let faux = without.intern(ANCHOR, Form::Terminal(false));
    let root_without = without.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, faux), (1, faux)]),
        },
    );
    assert_ne!(root_without, faux, "without R-collapse: the node is kept");
    let report = measure(&without, root_without, None).expect("interned node");
    assert_eq!(report.nodes_partages, 2, "node + terminal ⊥");
    assert_eq!(report.paths, 2, "two paths to ⊥");
}

/// Commuted R-default: the explicit δ(ℓ) arc toward D_δ is absorbed or not.
#[test]
fn test_defaut_commute() {
    // Region over domain [0, 3): {point 0 → ⊤, EMPTY → ⊥}.
    let mut with = pool_region(Reductions::TOUTES);
    let vrai = with.intern(ANCHOR, Form::Terminal(true));
    let faux = with.intern(ANCHOR, Form::Terminal(false));
    let root_with = with.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(Interval::point(0), vrai), (EMPTY, faux)]),
        },
    );
    let Form::Node { arcs, .. } = with.resolve(root_with).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 1, "R-default: the EMPTY arc to ⊥ is absorbed");
    let report = measure(&with, root_with, None).expect("interned node");
    assert_eq!(report.nodes_partages, 2, "node + ⊤");
    assert_eq!(report.paths, 1);

    let mut without = pool_region(Reductions::without_defaut());
    let vrai = without.intern(ANCHOR, Form::Terminal(true));
    let faux = without.intern(ANCHOR, Form::Terminal(false));
    let root_without = without.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(Interval::point(0), vrai), (EMPTY, faux)]),
        },
    );
    let Form::Node { arcs, .. } = without.resolve(root_without).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 2, "without R-default: the EMPTY arc is kept");
    let report = measure(&without, root_without, None).expect("interned node");
    assert_eq!(report.nodes_partages, 3, "node + ⊤ + ⊥");
    assert_eq!(report.paths, 2);
}

/// Metrics on a diamond: 4 shared nodes (root, n1, ⊤, ⊥) and 4 paths — each
/// root × n1 arc combination is a path of the unfolded DAG, even when the
/// sub-paths are shared.
#[test]
fn test_paths_contre_nodes() {
    let mut pool = pool_bdd(Reductions::without_collapse());
    let vrai = pool.intern(ANCHOR, Form::Terminal(true));
    let faux = pool.intern(ANCHOR, Form::Terminal(false));
    let n1 = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let root = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, n1), (1, n1)]),
        },
    );
    assert_eq!(count_nodes(&pool, root).expect("interned node"), 4);
    assert_eq!(count_paths(&pool, root).expect("interned node"), 4);
    // Unfolded size: root (1) + 2 × n1 (1 + ⊤ + ⊥ = 3) = 7.
    assert_eq!(count_size_depliee(&pool, root).expect("interned node"), 7);
    // A non-interned NodeId is a propagated error.
    assert!(count_nodes(&pool, NodeId(u32::MAX)).is_err());
    assert!(count_paths(&pool, NodeId(u32::MAX)).is_err());
    assert!(count_size_depliee(&pool, NodeId(u32::MAX)).is_err());
    assert!(measure(&pool, NodeId(u32::MAX), None).is_err());
}

/// Property (LCG): on random BDDs, disabling reductions can only increase
/// (or leave equal) shared nodes and paths — the same form is rebuilt in both
/// pools (same seed).
#[test]
fn test_propriete_commutation_lcg() {
    const SEED_LCG: u64 = 0x0A7E_11E5_2026_1110;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut seed = SEED_LCG;
    let mut next = || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed
    };
    // Builds a random binary tree of depth ≤ 3 in the pool, with its own LCG
    // re-seeded at `amorce` (same form guaranteed).
    fn arbre(pool: &mut PoolBdd, depth: u32, amorce: u64) -> NodeId {
        let mut seed = amorce;
        fn next(seed: &mut u64) -> u64 {
            *seed = seed
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            *seed
        }
        fn construit(pool: &mut PoolBdd, depth: u32, seed: &mut u64) -> NodeId {
            if depth == 0 || !next(seed).is_multiple_of(3) {
                return pool.intern(ANCHOR, Form::Terminal(next(seed) % 2 == 1));
            }
            let gauche = construit(pool, depth - 1, seed);
            let droite = construit(pool, depth - 1, seed);
            pool.intern(
                ANCHOR,
                Form::Node {
                    level: Level(depth),
                    arcs: BTreeMap::from([(0, gauche), (1, droite)]),
                },
            )
        }
        construit(pool, depth, &mut seed)
    }
    for _ in 0..NUMBER_CAS {
        let amorce = next();
        let mut with = pool_bdd(Reductions::TOUTES);
        let root_with = arbre(&mut with, 3, amorce);
        let mut without = pool_bdd(Reductions::AUCUNE);
        let root_without = arbre(&mut without, 3, amorce);
        let report_with = measure(&with, root_with, None).expect("interned node");
        let report_without = measure(&without, root_without, None).expect("interned node");
        assert!(report_with.paths >= 1, "at least one path");
        assert!(
            report_without.nodes_partages >= report_with.nodes_partages,
            "without reduction: nodes ≥ (same form)"
        );
        assert!(
            report_without.paths >= report_with.paths,
            "without reduction: paths ≥ (same form)"
        );
    }
}

/// Narration (ADR-0100 decision 6): `measure` emits an `Attr`/`Synthesized`
/// event carrying the full report — pinned by a collecting observer.
#[test]
fn test_narration_measure() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttr, TraceEvent};

    #[derive(Clone)]
    struct Collector {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collector {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::without_collapse());
    let vrai = pool.intern(ANCHOR, Form::Terminal(true));
    let faux = pool.intern(ANCHOR, Form::Terminal(false));
    let n1 = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let root = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, n1), (1, n1)]),
        },
    );
    let collector = Collector {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let report = measure(&pool, root, Some(&collector)).expect("interned node");
    assert_eq!(report.nodes_partages, 4);
    let events = collector.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
                value,
                ..
            } if value == "nodes_partages=4 paths=4 size_depliee=7"
        )),
        "the narration event carries the full report"
    );
}

/// Sliding window, absolute vs relative encoding (controller parameter,
/// backlog § 9): same paths (same logical object), but the absolute encoding
/// creates one node per base while the relative one shares a single form —
/// the window's "4 vs infinity", measured.
#[test]
fn test_window_absolu_croit_relatif_constant() {
    use lisage_core::atelier::comparer_encodages_window;

    const SIZE: u64 = 4;
    // 10 bases: absolute = root + 10 windows + ⊤ = 12 nodes; relative = 3.
    // (R-collapse commuted: an all-true window would otherwise collapse.)
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report =
        comparer_encodages_window(&mut pool, ANCHOR, SIZE, 10, None).expect("interned nodes");
    assert_eq!(report.absolu.nodes_partages, 12);
    assert_eq!(report.relatif.nodes_partages, 3);
    // Same logical object: same paths (10 bases × 4 keys) and same unfolded
    // size (root + 10 × (window + ⊤)).
    assert_eq!(report.absolu.paths, 40);
    assert_eq!(report.relatif.paths, 40);
    assert_eq!(report.absolu.size_depliee, 51);
    assert_eq!(report.relatif.size_depliee, 51);

    // 100 bases: the relative encoding stays at 3 nodes (constant), the absolute one grows to 102.
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report =
        comparer_encodages_window(&mut pool, ANCHOR, SIZE, 100, None).expect("interned nodes");
    assert_eq!(
        report.relatif.nodes_partages, 3,
        "the relative encoding is O(1)"
    );
    assert_eq!(
        report.absolu.nodes_partages, 102,
        "the absolute encoding grows with the bases"
    );
    assert_eq!(report.relatif.paths, 400);
}

/// Narration (ADR-0100): the encoding comparison emits an `Attr`/`Synthesized`
/// event carrying the two node counts.
#[test]
fn test_narration_comparaison_encodages() {
    use std::sync::{Arc, Mutex};

    use lisage_core::atelier::comparer_encodages_window;
    use lisage_core::observer::{ChannelAttr, TraceEvent};

    #[derive(Clone)]
    struct Collector {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collector {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::without_collapse());
    let collector = Collector {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let report = comparer_encodages_window(&mut pool, ANCHOR, 4, 10, Some(&collector))
        .expect("interned nodes");
    assert_eq!(report.relatif.nodes_partages, 3);
    let events = collector.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
                value,
                ..
            } if value == "encodage=relatif_vs_absolu nodes=3 contre 12"
        )),
        "the narration event carries the encoding comparison"
    );
}

/// Truth table of f = (x₀ ∧ x₁) ∨ (x₂ ∧ x₃) over 4 variables:
/// `table[a]` = bit₀(a) ∧ bit₁(a) ∨ bit₂(a) ∧ bit₃(a).
fn table_ou_deux_paires() -> Vec<bool> {
    (0..16)
        .map(|a| {
            let bit = |v: u32| (a >> v) & 1 == 1;
            (bit(0) && bit(1)) || (bit(2) && bit(3))
        })
        .collect()
}

/// Order parameter (backlog § 9): the same function compiled under two
/// orders has the same paths (2^n, one per assignment — same logical object)
/// and the same unfolded size (the full tree 2^{n+1}-1), but different
/// shared nodes — the order changes the diagram, not the function.
/// (R-collapse commuted: otherwise the paths would merge.)
/// Natural order [0,1,2,3]: 10 nodes; interleaved [0,2,1,3]: 12 nodes.
#[test]
fn test_order_change_nodes_pas_paths() {
    use lisage_core::atelier::{Order, comparer_orders};

    let table = table_ou_deux_paires();
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report = comparer_orders(
        &mut pool,
        ANCHOR,
        &table,
        &Order::naturel(4),
        &Order::personnalise(vec![0, 2, 1, 3]),
        None,
    )
    .expect("interned nodes");
    assert_eq!(report.first.nodes_partages, 10, "natural order");
    assert_eq!(report.second.nodes_partages, 12, "interleaved order");
    // Same logical object: 2^4 = 16 paths and unfolded tree 2^5-1 = 31.
    assert_eq!(report.first.paths, 16);
    assert_eq!(report.second.paths, 16);
    assert_eq!(report.first.size_depliee, 31);
    assert_eq!(report.second.size_depliee, 31);
}

/// The workshop's natural order is exactly the reference `compiler_bdd`
/// (same interned forms, same `NodeId` in the same pool).
#[test]
fn test_order_naturel_equivaut_a_compiler_bdd() {
    use lisage_core::atelier::{Order, compiler_bdd_order};

    let table = table_ou_deux_paires();
    let mut pool = pool_bdd(Reductions::TOUTES);
    let via_atelier = compiler_bdd_order(&mut pool, ANCHOR, &Order::naturel(4), &table);
    let via_bdd = compiler_bdd(&mut pool, ANCHOR, &BddKeys::new(4), &table);
    assert_eq!(via_atelier, via_bdd);
}

/// Didactic growth: over 6 variables, f = (x₀∧x₁)∨(x₂∧x₃)∨(x₄∧x₅), the
/// natural order groups the pairs (15 nodes), the interleaved order
/// [0,2,4,1,3,5] separates them (25 nodes) — same 64 paths on both sides.
#[test]
fn test_order_intercale_croissance_six_variables() {
    use lisage_core::atelier::{Order, comparer_orders};

    let table: Vec<bool> = (0..64)
        .map(|a| {
            let bit = |v: u32| (a >> v) & 1 == 1;
            (bit(0) && bit(1)) || (bit(2) && bit(3)) || (bit(4) && bit(5))
        })
        .collect();
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report = comparer_orders(
        &mut pool,
        ANCHOR,
        &table,
        &Order::naturel(6),
        &Order::personnalise(vec![0, 2, 4, 1, 3, 5]),
        None,
    )
    .expect("interned nodes");
    assert_eq!(report.first.nodes_partages, 15, "natural order");
    assert_eq!(report.second.nodes_partages, 25, "interleaved order");
    assert_eq!(report.first.paths, 64);
    assert_eq!(report.second.paths, 64);
}

/// Narration (ADR-0100): the order comparison emits an `Attr`/`Synthesized`
/// event carrying the two order labels and the two node counts.
#[test]
fn test_narration_comparaison_orders() {
    use std::sync::{Arc, Mutex};

    use lisage_core::atelier::{Order, comparer_orders};
    use lisage_core::observer::{ChannelAttr, TraceEvent};

    #[derive(Clone)]
    struct Collector {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collector {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let table = table_ou_deux_paires();
    let mut pool = pool_bdd(Reductions::without_collapse());
    let collector = Collector {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let report = comparer_orders(
        &mut pool,
        ANCHOR,
        &table,
        &Order::naturel(4),
        &Order::personnalise(vec![0, 2, 1, 3]),
        Some(&collector),
    )
    .expect("interned nodes");
    assert_eq!(report.first.nodes_partages, 10);
    let events = collector.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
                value,
                ..
            } if value == "order=[0,1,2,3]_vs_[0,2,1,3] nodes=10 contre 12"
        )),
        "the narration event carries the order comparison"
    );
}

/// Composite anchor parameter (backlog § 9, SWP thread): the slide is the
/// movement of the base within the anchor `{base, horizon}`; the horizon does
/// not move and the absolute reading of the window advances.
#[test]
fn test_anchor_window_glisse() {
    use lisage_core::atelier::AnchorWindow;

    let anchor = AnchorWindow::new(0, 4);
    assert_eq!(anchor.base(), 0);
    assert_eq!(anchor.horizon(), 4);
    assert_eq!(anchor.window_absolue(), (0, 4));
    assert_eq!(
        anchor.key_input(),
        0,
        "the base is the argument of the entry arc"
    );
    assert_eq!(anchor.label(), "{base: 0, horizon: 4}");
    let glissee = anchor.glessee(2);
    assert_eq!(glissee.base(), 2);
    assert_eq!(glissee.horizon(), 4, "the slide only affects the base");
    assert_eq!(glissee.window_absolue(), (2, 6));
    assert_eq!(glissee.key_input(), 2);
    assert_eq!(glissee.label(), "{base: 2, horizon: 4}");
    // The original anchor is unchanged (the slide produces a new anchor).
    assert_eq!(anchor.base(), 0);
}

/// The window read under two composite anchors: each anchor has its root
/// (the entry arc carries its base), but both reach the same window form
/// (`same_form`) — same nodes, same paths, different absolute readings:
/// "the anchor moves, the pool never does". Over 50 slides, only the root is
/// created at each step, never the form.
#[test]
fn test_glisse_pool_constant_lectures_differentes() {
    use lisage_core::atelier::{AnchorWindow, build_window_anchored, comparer_anchors_window};

    const SIZE: u64 = 4;
    // (R-collapse commuted: an all-true window would otherwise collapse.)
    let mut pool = pool_bdd(Reductions::without_collapse());
    let report = comparer_anchors_window(
        &mut pool,
        ANCHOR,
        SIZE,
        &AnchorWindow::new(0, 4),
        &AnchorWindow::new(7, 4),
        None,
    )
    .expect("interned nodes");
    assert!(report.same_form, "both anchors reach the same window form");
    assert_eq!(report.avant, report.apres, "same metrics on both sides");
    assert_eq!(report.avant.nodes_partages, 3, "root + window + ⊤");
    assert_eq!(report.avant.paths, 4, "one path per window key");
    assert_eq!(report.avant.size_depliee, 6);
    assert_eq!(
        report.nodes_pool, 4,
        "⊤ + shared window + one root per anchor"
    );
    assert_eq!(report.lecture_avant, (0, 4));
    assert_eq!(report.lecture_apres, (7, 11));

    // 50 slides (bases 100..150, all new): at each step the base enters via
    // the entry arc, the window form reached is always the same and the pool
    // creates only one root per slide — never a copy of the window.
    let nodes_avant = pool.node_count();
    let (_, window) = build_window_anchored(&mut pool, ANCHOR, SIZE, &AnchorWindow::new(0, 4));
    let anchor = AnchorWindow::new(100, SIZE as i64);
    for delta in 0..50 {
        let glissee = anchor.glessee(delta);
        assert_eq!(
            glissee.window_absolue(),
            (100 + delta, 100 + delta + SIZE as i64)
        );
        let (root, window_glissee) = build_window_anchored(&mut pool, ANCHOR, SIZE, &glissee);
        assert_eq!(window_glissee, window, "window form shared at each slide");
        assert_ne!(root, window, "each base has its own root");
    }
    assert_eq!(
        pool.node_count(),
        nodes_avant + 50,
        "a single root per slide: the window form is never duplicated"
    );
}

/// Property (LCG): the slide composes — sliding by `a` then by `b` equals
/// sliding by `a + b` — and the horizon is invariant (30 cases).
#[test]
fn test_propriete_glisse_composition_lcg() {
    use lisage_core::atelier::AnchorWindow;

    const SEED_LCG: u64 = 0x5EED_2026_1112_4E11;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut seed = SEED_LCG;
    let mut next = || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed
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

/// Narration (ADR-0100): the composite anchor comparison emits an
/// `Attr`/`Synthesized` event carrying the two labels, the two node counts and
/// the two absolute readings.
#[test]
fn test_narration_comparaison_anchors() {
    use std::sync::{Arc, Mutex};

    use lisage_core::atelier::{AnchorWindow, comparer_anchors_window};
    use lisage_core::observer::{ChannelAttr, TraceEvent};

    #[derive(Clone)]
    struct Collector {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collector {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::without_collapse());
    let collector = Collector {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let report = comparer_anchors_window(
        &mut pool,
        ANCHOR,
        4,
        &AnchorWindow::new(0, 4),
        &AnchorWindow::new(2, 4),
        Some(&collector),
    )
    .expect("interned nodes");
    assert_eq!(report.avant.nodes_partages, 3);
    let events = collector.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
                value,
                ..
            } if value == "anchor={base: 0, horizon: 4}_vs_{base: 2, horizon: 4} nodes=3 contre 3 lecture=[0..4) contre [2..6) pool=4"
        )),
        "the narration event carries the anchor comparison"
    );
}
