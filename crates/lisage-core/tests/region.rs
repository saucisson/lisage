//! Region key algebra tests, intervals fragment (backlog § 9, open question
//! 9 of the spec): canonical normal form (C1 relative to the region
//! algebra), cardinality, default, pool sharing, and differential against
//! naive enumeration on one level.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{EMPTY, Intervalle, RegionController, RegionKeys};

type PoolRegion = Arena<RegionController>;

const ANCHOR: Anchor = Anchor(0);

fn pool(domains: Vec<u64>) -> PoolRegion {
    Arena::new(
        RegionController::new(RegionKeys::new(domains)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_intervalle_empty_normalise() {
    let empty = Intervalle::new(5, 3);
    assert_eq!(empty, EMPTY);
    assert!(empty.est_empty());
    assert_eq!(empty.size(), 0);
    assert!(!empty.contains(3));
    let point = Intervalle::point(4);
    assert!(!point.est_empty());
    assert_eq!(point.size(), 1);
    assert!(point.contains(4));
    assert!(!point.contains(5));
    assert_eq!(Intervalle::new(2, 5).size(), 4);
}

#[test]
fn test_normal_form_canonique_et_idempotente() {
    let keys = RegionKeys::new(vec![8]);
    // Any writing of an empty interval has the same normal form.
    assert_eq!(keys.normal_form(&Intervalle::new(7, 2)), EMPTY);
    assert_eq!(keys.normal_form(&EMPTY), EMPTY);
    // Form normale idempotente, et stable sur les non-empty.
    let intervalle = Intervalle::new(2, 5);
    assert_eq!(keys.normal_form(&keys.normal_form(&intervalle)), intervalle);
}

#[test]
fn test_cardinality_et_defaut_par_level() {
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
fn test_partage_pool_c1_relatif() {
    let mut pool = pool(vec![8]);
    let terminal = pool.intern(ANCHOR, Form::Terminal(true));
    let level = Level(0);
    let form_empty_a = Form::Node {
        level,
        arcs: BTreeMap::from([(EMPTY, terminal)]),
    };
    let form_empty_b = Form::Node {
        level,
        arcs: BTreeMap::from([(Intervalle::new(9, 2), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCHOR, form_empty_a.clone()),
        pool.intern(ANCHOR, form_empty_b),
        "empty interval under two writings = same node"
    );
    let form_point_a = Form::Node {
        level,
        arcs: BTreeMap::from([(Intervalle::point(3), terminal)]),
    };
    let form_point_b = Form::Node {
        level,
        arcs: BTreeMap::from([(Intervalle::new(3, 3), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCHOR, form_point_a.clone()),
        pool.intern(ANCHOR, form_point_b),
        "point written two ways = same node"
    );
    let form_other = Form::Node {
        level,
        arcs: BTreeMap::from([(Intervalle::point(4), terminal)]),
    };
    assert_ne!(
        pool.intern(ANCHOR, form_point_a),
        pool.intern(ANCHOR, form_other),
        "distinct regions = distinct nodes"
    );
}

/// Property (LCG): the normal form is idempotent, and size/est_empty/
/// contains are consistent with the normalized representation.
#[test]
fn test_propriete_normalisation_lcg() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let keys = RegionKeys::new(vec![16]);
    let mut seed = SEED_LCG;
    for _ in 0..NUMBER_CAS {
        let mut next = || {
            seed = seed
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            seed
        };
        let inf = next() % 16;
        let sup = next() % 16;
        let intervalle = Intervalle::new(inf, sup);
        let normalisee = keys.normal_form(&intervalle);
        assert_eq!(keys.normal_form(&normalisee), normalisee, "idempotence");
        assert_eq!(normalisee.est_empty(), inf > sup);
        assert_eq!(normalisee.size(), if inf > sup { 0 } else { sup - inf + 1 });
        let point = next() % 16;
        assert_eq!(
            normalisee.contains(point),
            !normalisee.est_empty() && inf <= point && point <= sup
        );
    }
}

/// Differential against naive enumeration: a level-0 node whose arcs are
/// disjoint intervals to `terminal(true)` (the set S) or `terminal(false)`
/// (its complement) represents S; the point count of the DD's true arcs
/// equals |S|. The `EMPTY` arc to the default is removed by R-default.
#[test]
fn test_differentiel_union_intervalles_contre_enumeration() {
    const SEED_LCG: u64 = 0x2545_F491_4F6C_DD1D;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    const SIZE_DOMAIN: u64 = 12;
    let mut pool = pool(vec![SIZE_DOMAIN]);
    let vrai_terminal = pool.intern(ANCHOR, Form::Terminal(true));
    let faux_terminal = pool.intern(ANCHOR, Form::Terminal(false));
    let mut seed = SEED_LCG;
    for _ in 0..NUMBER_CAS {
        let mut next = || {
            seed = seed
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            seed
        };
        // Subset S of [0, SIZE_DOMAIN) drawn with the LCG.
        let set: BTreeSet<u64> = (0..SIZE_DOMAIN).filter(|_| next() % 2 == 1).collect();
        // Compression of S and its complement into disjoint intervals.
        let mut arcs = BTreeMap::new();
        let mut point = 0;
        while point < SIZE_DOMAIN {
            let dans_s = set.contains(&point);
            let mut fin = point;
            while fin + 1 < SIZE_DOMAIN && set.contains(&(fin + 1)) == dans_s {
                fin += 1;
            }
            arcs.insert(
                Intervalle::new(point, fin),
                if dans_s { vrai_terminal } else { faux_terminal },
            );
            point = fin + 1;
        }
        // The absent arc leads to the default D_δ = terminal(false): R-default.
        arcs.insert(EMPTY, faux_terminal);
        let root = pool.intern(
            ANCHOR,
            Form::Node {
                level: Level(0),
                arcs,
            },
        );
        let count_dd: u64 = match pool.resolve(root).expect("interned node") {
            Form::Terminal(value) => {
                // R-collapse: S empty or full, the node reduces to the terminal.
                if *value { SIZE_DOMAIN } else { 0 }
            }
            Form::Node { arcs, .. } => {
                assert!(
                    !arcs.contains_key(&EMPTY),
                    "R-default removes the arc to the default"
                );
                arcs.iter()
                    .filter(|(_, cible)| **cible == vrai_terminal)
                    .map(|(intervalle, _)| intervalle.size())
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
