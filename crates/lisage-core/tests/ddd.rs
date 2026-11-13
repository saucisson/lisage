//! DDD-difference instance-test tests (backlog § 9, open question 9
//! of the spec, thread #83): key contract on the controller side (intersection =
//! meet of DBMs, coverage of the bounded box), then the diagram of the
//! band `x − y ≤ 1 ∧ y − x ≤ 2` built by explicit regions —
//! membership differential against naive enumeration over a small
//! box, and sharing in the pool (C1 relative to the region algebra).

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCONSISTENT};
use lisage_core::ddd::{compile_band, count_nodes, evaluer};
use lisage_core::observer::NoOpObserver;

type PoolDbm = Arena<DbmController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAINE: u64 = 4;

fn pool() -> PoolDbm {
    Arena::new(
        DbmController::new(DbmKeys::new(SIZE_DOMAINE)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_intersection_meet_canonical() {
    // meet of x−y ≤ 1 and y−x ≤ 2 = the explicit band, normal form included.
    let a = Dbm::new(2, &[(0, 1, 1)]);
    let b = Dbm::new(2, &[(1, 0, 2)]);
    let meet = a.intersection(&b);
    let band = Dbm::new(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(meet, band, "the meet of two regions is the band, closed");
    // Commutativity, idempotence.
    assert_eq!(a.intersection(&b), b.intersection(&a));
    assert_eq!(a.intersection(&a), a);
    // Meet with a disjoint region: inconsistent → canonical empty region.
    let disjoint = Dbm::new(2, &[(1, 0, -3)]);
    assert_eq!(a.intersection(&disjoint), INCONSISTENT);
    // Any empty region gives the empty region.
    assert_eq!(a.intersection(&INCONSISTENT), INCONSISTENT);
    assert_eq!(INCONSISTENT.intersection(&a), INCONSISTENT);
}

#[test]
fn test_coverage_box_bounded() {
    let keys = DbmKeys::new(SIZE_DOMAINE);
    let below = Dbm::new(2, &[(0, 1, 0)]);
    let above = Dbm::new(2, &[(1, 0, -1)]);
    // The partition {x ≤ y, x > y} covers the box [0, 4)².
    assert!(keys.coverage(&[below.clone(), above.clone()]));
    // A single half does not cover; neither does an empty family; nor an empty region.
    assert!(!keys.coverage(&[below]));
    assert!(!keys.coverage(&[]));
    assert!(!keys.coverage(&[INCONSISTENT]));
    // The band alone does not cover the box.
    let band = Dbm::new(2, &[(0, 1, 1), (1, 0, 2)]);
    assert!(!keys.coverage(&[band]));
}

/// Instance-test: the band compiled by explicit regions has the expected
/// semantics over the whole box, and fits in O(1) nodes against T² points —
/// the universe sweep does not compiled, the free DDD kingdom is assumed.
#[test]
fn test_band_differential_against_enumeration() {
    let mut pool = pool();
    let root = compile_band(&mut pool, ANCHOR);
    // Four nodes: true, false, level 1, the root — against 16 points.
    assert_eq!(count_nodes(&pool, root), 4);
    for x in 0..SIZE_DOMAINE {
        for y in 0..SIZE_DOMAINE {
            let point = [x, y];
            let expected = i128::from(x) - i128::from(y) <= 1 && i128::from(y) - i128::from(x) <= 2;
            let lu = evaluer(&pool, root, &point).expect("interned node");
            assert_eq!(
                lu.unwrap_or(false),
                expected,
                "point {point:?} : the continuous absent is false"
            );
        }
    }
}

/// Relative C1: two constructions of the same band share the root in the
/// pool (R-share after the normal form of the regions).
#[test]
fn test_shared_pool_band() {
    let mut pool = pool();
    let root_a = compile_band(&mut pool, ANCHOR);
    let root_b = compile_band(&mut pool, ANCHOR);
    assert_eq!(root_a, root_b, "same band = same root");
    // The root does have two edges at level 0 (region + complement).
    let Form::Node { edges, .. } = pool.resolve(root_a).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(edges.len(), 2);
    let keys = DbmKeys::new(SIZE_DOMAINE);
    let regions: Vec<Dbm> = edges.keys().cloned().collect();
    assert!(keys.coverage(&regions), "the level-0 edges cover the box");
}

/// Property (LCG): the intersection is commutative, idempotent, and every
/// point of the meet belongs to both regions.
#[test]
fn test_property_intersection_lcg() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
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
    let drawn = |next: &mut dyn FnMut() -> u64| {
        let number_constraints = next() % 4;
        let constraints: Vec<(usize, usize, i64)> = (0..number_constraints)
            .map(|_| {
                (
                    (next() % 2) as usize,
                    (next() % 2) as usize,
                    (next() % 7) as i64 - 3,
                )
            })
            .collect();
        Dbm::new(2, &constraints)
    };
    for _ in 0..NUMBER_CAS {
        let a = drawn(&mut next);
        let b = drawn(&mut next);
        let meet = a.intersection(&b);
        assert_eq!(meet, b.intersection(&a), "commutativity");
        assert_eq!(a.intersection(&a), a, "idempotence");
        let point = [next() % SIZE_DOMAINE, next() % SIZE_DOMAINE];
        if meet.contains(&point) {
            assert!(a.contains(&point) && b.contains(&point));
        }
    }
}

/// The key contract remains declared on the controller side: cardinality and
/// default of the level are those of the DBM algebra (coverage is tested separately).
#[test]
fn test_contract_key_level() {
    let keys = DbmKeys::new(SIZE_DOMAINE);
    assert_eq!(keys.cardinality(Level(0)), SIZE_DOMAINE);
    assert_eq!(keys.default(Level(0)), Some(INCONSISTENT));
}
