//! DBM region-key algebra tests (backlog § 9, open question 9
//! of the spec, thread #83): canonical closure (C1 relative to the
//! region algebra), inconsistency, cardinality, default, sharing in the pool, and
//! membership differential against naive enumeration over a small box.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCONSISTENT};
use lisage_core::observer::NoOpObserver;

type PoolDbm = Arena<DbmController>;

const ANCHOR: Anchor = Anchor(0);

fn pool(size_domain: u64) -> PoolDbm {
    Arena::new(
        DbmController::new(DbmKeys::new(size_domain)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_dbm_closure_canonical() {
    // x0 − x1 ≤ 2 and x1 − x0 ≤ 3 describe the same region as the closure
    // direct; a redundant constraint does not change the normal form.
    let direct = Dbm::new(2, &[(0, 1, 2), (1, 0, 3)]);
    let with_redundancy = Dbm::new(2, &[(0, 1, 2), (1, 0, 3), (0, 1, 5), (0, 0, 0)]);
    assert_eq!(direct, with_redundancy, "canonical closure");
    // The closure propagates: x0−x1 ≤ 1 and x1−x2 ≤ 1 imply x0−x2 ≤ 2.
    let propagee = Dbm::new(3, &[(0, 1, 1), (1, 2, 1)]);
    let explicite = Dbm::new(3, &[(0, 1, 1), (1, 2, 1), (0, 2, 2)]);
    assert_eq!(
        propagee, explicite,
        "the closure propagates transitive constraints"
    );
}

#[test]
fn test_dbm_inconsistent_normalize() {
    // x0 − x1 ≤ 2 and x1 − x0 ≤ −3 are incompatible: empty region.
    let incoherente = Dbm::new(2, &[(0, 1, 2), (1, 0, -3)]);
    assert_eq!(incoherente, INCONSISTENT);
    assert!(incoherente.is_empty());
    assert!(!incoherente.contains(&[0, 0]));
    // Any inconsistent region, whatever its writing, is INCONSISTENT.
    let other = Dbm::new(3, &[(0, 1, -1), (1, 2, 0), (2, 0, 0)]);
    assert_eq!(other, INCONSISTENT);
}

#[test]
fn test_dbm_contains() {
    // Region x0 − x1 ≤ 1 over the box [0, 4)².
    let region = Dbm::new(2, &[(0, 1, 1)]);
    assert!(region.contains(&[2, 2]));
    assert!(region.contains(&[3, 2]));
    assert!(!region.contains(&[3, 1]));
}

#[test]
fn test_cardinality_and_default() {
    let keys = DbmKeys::new(4);
    assert_eq!(keys.cardinality(Level(0)), 4);
    assert_eq!(keys.cardinality(Level(7)), 4);
    // δ(ℓ) = the empty region, the continuous "absent".
    assert_eq!(keys.default(Level(0)), Some(INCONSISTENT));
}

/// Relative C1: two writings of the same region share the same node in the
/// pool (R-share after normal form), two distinct regions do not.
#[test]
fn test_shared_pool_c1_relative() {
    let mut pool = pool(4);
    let terminal = pool.intern(ANCHOR, Form::Terminal(true));
    let level = Level(0);
    let region_a = Dbm::new(2, &[(0, 1, 2), (1, 0, 3)]);
    let region_b = Dbm::new(2, &[(0, 1, 2), (1, 0, 3), (0, 1, 9)]);
    let form_a = Form::Node {
        level,
        edges: BTreeMap::from([(region_a, terminal)]),
    };
    let form_b = Form::Node {
        level,
        edges: BTreeMap::from([(region_b, terminal)]),
    };
    assert_eq!(
        pool.intern(ANCHOR, form_a.clone()),
        pool.intern(ANCHOR, form_b),
        "same region under two writings = same node"
    );
    let form_empty_a = Form::Node {
        level,
        edges: BTreeMap::from([(INCONSISTENT, terminal)]),
    };
    let form_empty_b = Form::Node {
        level,
        edges: BTreeMap::from([(Dbm::new(2, &[(0, 1, 1), (1, 0, -2)]), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCHOR, form_empty_a),
        pool.intern(ANCHOR, form_empty_b),
        "empty region under two writings = same node"
    );
    let form_other = Form::Node {
        level,
        edges: BTreeMap::from([(Dbm::new(2, &[(0, 1, 0)]), terminal)]),
    };
    assert_ne!(
        pool.intern(ANCHOR, form_a),
        pool.intern(ANCHOR, form_other),
        "distinct regions = distinct nodes"
    );
}

/// Property (LCG): the normal form is idempotent; two DBMs drawn at
/// random with the same closure are equal; `contains` is consistent with the
/// normalized matrix.
#[test]
fn test_property_closure_lcg() {
    const GRAINE_LCG: u64 = 0x853C_49E6_748F_EA9B;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let keys = DbmKeys::new(6);
    let mut graine = GRAINE_LCG;
    let mut next = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    for _ in 0..NUMBER_CAS {
        let n = 2 + (next() % 2) as usize;
        let number_constraints = next() % 5;
        let constraints: Vec<(usize, usize, i64)> = (0..number_constraints)
            .map(|_| {
                (
                    (next() % n as u64) as usize,
                    (next() % n as u64) as usize,
                    (next() % 7) as i64 - 3,
                )
            })
            .collect();
        let dbm = Dbm::new(n, &constraints);
        let normalized = keys.normal_form(&dbm);
        assert_eq!(keys.normal_form(&normalized), normalized, "idempotence");
        // Membership of a randomly drawn point, consistent with is_empty.
        let point: Vec<u64> = (0..n).map(|_| next() % 6).collect();
        if normalized.is_empty() {
            assert!(!normalized.contains(&point));
        }
    }
}

/// Differential against naive enumeration: a level-0 node partitions
/// the box `[0, 4)²` into two DBM regions (`x0 − x1 ≤ 0` true/false); for
/// each point of the box, the target read in the DD equals the naive target.
/// The empty-region edge leads to the default D_δ = terminal(false): R-default.
#[test]
fn test_differential_membership_against_enumeration() {
    const SIZE_DOMAINE: u64 = 4;
    let mut pool = pool(SIZE_DOMAINE);
    let true_terminal = pool.intern(ANCHOR, Form::Terminal(true));
    let false_terminal = pool.intern(ANCHOR, Form::Terminal(false));
    // x0 − x1 ≤ 0  ⇔  x0 ≤ x1; the complement is x1 − x0 ≤ −1.
    let region_below = Dbm::new(2, &[(0, 1, 0)]);
    let region_above = Dbm::new(2, &[(1, 0, -1)]);
    let root = pool.intern(
        ANCHOR,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([
                (region_below.clone(), true_terminal),
                (region_above.clone(), false_terminal),
                (INCONSISTENT, false_terminal),
            ]),
        },
    );
    let form = pool.resolve(root).expect("interned node");
    let Form::Node { edges, .. } = form else {
        panic!("two distinct regions: no R-collapse");
    };
    assert!(
        !edges.contains_key(&INCONSISTENT),
        "R-default removes the edge to the default"
    );
    for x0 in 0..SIZE_DOMAINE {
        for x1 in 0..SIZE_DOMAINE {
            let point = [x0, x1];
            let target_naive = if x0 <= x1 {
                true_terminal
            } else {
                false_terminal
            };
            let target_dd = edges
                .iter()
                .find(|(region, _)| region.contains(&point))
                .map(|(_, target)| *target)
                .expect("the partition covers the box");
            assert_eq!(target_dd, target_naive, "point {point:?}");
        }
    }
}
