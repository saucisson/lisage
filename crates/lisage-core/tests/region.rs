//! Region-key algebra tests, interval fragment (backlog § 9,
//! open question 9 of the spec): canonical normal form (C1 relative to
//! the region algebra), cardinality, default, sharing in the pool, and
//! differential against naive enumeration over one level.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{EMPTY, Interval, RegionController, RegionKeys};

type PoolRegion = Arena<RegionController>;

const ANCHOR: Anchor = Anchor(0);

fn pool(domaines: Vec<u64>) -> PoolRegion {
    Arena::new(
        RegionController::new(RegionKeys::new(domaines)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_interval_empty_normalize() {
    let empty = Interval::new(5, 3);
    assert_eq!(empty, EMPTY);
    assert!(empty.is_empty());
    assert_eq!(empty.size(), 0);
    assert!(!empty.contains(3));
    let point = Interval::point(4);
    assert!(!point.is_empty());
    assert_eq!(point.size(), 1);
    assert!(point.contains(4));
    assert!(!point.contains(5));
    assert_eq!(Interval::new(2, 5).size(), 4);
}

#[test]
fn test_normal_form_canonical_and_idempotent() {
    let keys = RegionKeys::new(vec![8]);
    // Any writing of an empty interval has the same normal form.
    assert_eq!(keys.normal_form(&Interval::new(7, 2)), EMPTY);
    assert_eq!(keys.normal_form(&EMPTY), EMPTY);
    // Idempotent normal form, and stable on not-empty ones.
    let interval = Interval::new(2, 5);
    assert_eq!(keys.normal_form(&keys.normal_form(&interval)), interval);
}

#[test]
fn test_cardinality_and_default_by_level() {
    let keys = RegionKeys::new(vec![8, 3]);
    assert_eq!(keys.cardinality(Level(0)), 8);
    assert_eq!(keys.cardinality(Level(1)), 3);
    // δ(ℓ) = the empty interval, the continuous "absent".
    assert_eq!(keys.default(Level(0)), Some(EMPTY));
    assert_eq!(keys.default(Level(1)), Some(EMPTY));
}

/// Relative C1: two writings of the same region share the same node in the
/// pool (R-share after normal form), two distinct regions do not.
#[test]
fn test_shared_pool_c1_relative() {
    let mut pool = pool(vec![8]);
    let terminal = pool.intern(ANCHOR, Form::Terminal(true));
    let level = Level(0);
    let form_empty_a = Form::Node {
        level,
        edges: BTreeMap::from([(EMPTY, terminal)]),
    };
    let form_empty_b = Form::Node {
        level,
        edges: BTreeMap::from([(Interval::new(9, 2), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCHOR, form_empty_a.clone()),
        pool.intern(ANCHOR, form_empty_b),
        "empty interval under two writings = same node"
    );
    let form_point_a = Form::Node {
        level,
        edges: BTreeMap::from([(Interval::point(3), terminal)]),
    };
    let form_point_b = Form::Node {
        level,
        edges: BTreeMap::from([(Interval::new(3, 3), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCHOR, form_point_a.clone()),
        pool.intern(ANCHOR, form_point_b),
        "point written two ways = same node"
    );
    let form_other = Form::Node {
        level,
        edges: BTreeMap::from([(Interval::point(4), terminal)]),
    };
    assert_ne!(
        pool.intern(ANCHOR, form_point_a),
        pool.intern(ANCHOR, form_other),
        "distinct regions = distinct nodes"
    );
}

/// Property (LCG): the normal form is idempotent, and size/is_empty/
/// contains are consistent with the normalized representation.
#[test]
fn test_property_normalisation_lcg() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let keys = RegionKeys::new(vec![16]);
    let mut graine = GRAINE_LCG;
    for _ in 0..NUMBER_CAS {
        let mut next = || {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            graine
        };
        let inf = next() % 16;
        let sup = next() % 16;
        let interval = Interval::new(inf, sup);
        let normalized = keys.normal_form(&interval);
        assert_eq!(keys.normal_form(&normalized), normalized, "idempotence");
        assert_eq!(normalized.is_empty(), inf > sup);
        assert_eq!(normalized.size(), if inf > sup { 0 } else { sup - inf + 1 });
        let point = next() % 16;
        assert_eq!(
            normalized.contains(point),
            !normalized.is_empty() && inf <= point && point <= sup
        );
    }
}

/// Differential against naive enumeration: a level-0 node whose
/// edges are disjoint intervals to `terminal(true)` (the set S)
/// or `terminal(false)` (its complement) represents S; the point count
/// of the true edges of the DD equals |S|. The `EMPTY` edge to the default is
/// removed by R-default.
#[test]
fn test_differential_union_intervals_against_enumeration() {
    const GRAINE_LCG: u64 = 0x2545_F491_4F6C_DD1D;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    const SIZE_DOMAINE: u64 = 12;
    let mut pool = pool(vec![SIZE_DOMAINE]);
    let true_terminal = pool.intern(ANCHOR, Form::Terminal(true));
    let false_terminal = pool.intern(ANCHOR, Form::Terminal(false));
    let mut graine = GRAINE_LCG;
    for _ in 0..NUMBER_CAS {
        let mut next = || {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            graine
        };
        // Subset S of [0, SIZE_DOMAINE) drawn from the LCG.
        let set: BTreeSet<u64> = (0..SIZE_DOMAINE).filter(|_| next() % 2 == 1).collect();
        // Compression of S and its complement into disjoint intervals.
        let mut edges = BTreeMap::new();
        let mut point = 0;
        while point < SIZE_DOMAINE {
            let in_s = set.contains(&point);
            let mut end = point;
            while end + 1 < SIZE_DOMAINE && set.contains(&(end + 1)) == in_s {
                end += 1;
            }
            edges.insert(
                Interval::new(point, end),
                if in_s { true_terminal } else { false_terminal },
            );
            point = end + 1;
        }
        // The absent edge leads to the default D_δ = terminal(false): R-default.
        edges.insert(EMPTY, false_terminal);
        let root = pool.intern(
            ANCHOR,
            Form::Node {
                level: Level(0),
                edges,
            },
        );
        let count_dd: u64 = match pool.resolve(root).expect("interned node") {
            Form::Terminal(value) => {
                // R-collapse: S empty or full, the node reduces to the terminal.
                if *value { SIZE_DOMAINE } else { 0 }
            }
            Form::Node { edges, .. } => {
                assert!(
                    !edges.contains_key(&EMPTY),
                    "R-default removes the edge to the default"
                );
                edges
                    .iter()
                    .filter(|(_, target)| **target == true_terminal)
                    .map(|(interval, _)| interval.size())
                    .sum()
            }
        };
        assert_eq!(
            count_dd,
            set.len() as u64,
            "DD count == naive cardinality of S"
        );
    }
}
