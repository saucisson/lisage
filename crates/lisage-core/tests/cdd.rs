//! CDD instance-test tests (backlog § 9, open question 9 of the
//! spec, thread #83): not-convex unions of DBM zones as clock region-keys
//! — syntactic normal form (sort + dedup, assumed not-canonicity),
//! decidable membership, distributive intersection, coverage
//! of the bounded box — then the diagram of the not-convex region
//! `{x − y ≤ 0} ∪ {x − y ≥ 2}` built by explicit regions:
//! membership differential against naive enumeration over a small
//! box, and sharing in the pool (C1 relative to the normal form of the union).

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::cdd::{
    Cdd, CddController, CddKeys, EMPTY, compile_union_clocks, count_nodes, evaluer,
};
use lisage_core::dbm::Dbm;
use lisage_core::observer::NoOpObserver;

type PoolCdd = Arena<CddController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAINE: u64 = 4;

fn pool() -> PoolCdd {
    Arena::new(
        CddController::new(CddKeys::new(SIZE_DOMAINE)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_normalisation_union() {
    let a = Dbm::new(2, &[(0, 1, 0)]);
    let b = Dbm::new(2, &[(1, 0, -2)]);
    // Sort + dedup: the writing [a, b, a] normalizes to the derived order [b, a].
    let union = Cdd::new(&[a.clone(), b.clone(), a.clone()]);
    assert_eq!(union.zones(), &[b.clone(), a.clone()]);
    assert!(!union.is_empty());
    // Empty zones are removed; zero zones = canonical empty union.
    let with_empty = Cdd::new(&[a.clone(), lisage_core::dbm::INCONSISTENT]);
    assert_eq!(with_empty.zones(), &[a]);
    assert_eq!(Cdd::new(&[]), EMPTY);
    assert!(EMPTY.is_empty());
    assert!(Cdd::new(&[lisage_core::dbm::INCONSISTENT]).is_empty());
}

/// Assumed not-canonicity (spec Q9 (i)): two writings of the same
/// not-convex set remain distinct keys if the syntactic normal form
/// differs — here a zone included in another.
#[test]
fn test_not_canonicity_assumed() {
    let big = Dbm::new(2, &[(0, 1, 5)]);
    let incluse = Dbm::new(2, &[(0, 1, 0)]);
    assert!(big.contains(&[0, 0]) && incluse.contains(&[0, 0]));
    let union_with_redundancy = Cdd::new(&[big.clone(), incluse.clone()]);
    let union_only = Cdd::new(std::slice::from_ref(&big));
    // Semantically the same region (the included one is in the big one)…
    for x in 0..SIZE_DOMAINE {
        for y in 0..SIZE_DOMAINE {
            assert_eq!(
                union_with_redundancy.contains(&[x, y]),
                union_only.contains(&[x, y])
            );
        }
    }
    // …but not the same key: no known set normal form.
    assert_ne!(union_with_redundancy, union_only);
}

#[test]
fn test_contains_semantic() {
    let a = Dbm::new(2, &[(0, 1, 0)]); // x − y ≤ 0
    let b = Dbm::new(2, &[(1, 0, -2)]); // x − y ≥ 2
    let union = Cdd::new(&[a, b]);
    assert!(union.contains(&[0, 0]));
    assert!(union.contains(&[1, 3]));
    assert!(union.contains(&[3, 1]));
    assert!(!union.contains(&[2, 1])); // 0 < x − y < 2
    assert!(!EMPTY.contains(&[0, 0]));
}

#[test]
fn test_intersection_distributive() {
    let a = Dbm::new(2, &[(0, 1, 0)]);
    let b = Dbm::new(2, &[(1, 0, -2)]);
    let c = Dbm::new(2, &[(0, 1, 1)]); // x − y ≤ 1
    let union_ab = Cdd::new(&[a.clone(), b.clone()]);
    let union_c = Cdd::new(std::slice::from_ref(&c));
    // (A ∪ B) ∩ C = (A ∩ C) ∪ (B ∩ C) ; B ∩ C is empty (x−y ≥ 2 and x−y ≤ 1).
    let meet = union_ab.intersection(&union_c);
    let expected = Cdd::new(&[a.intersection(&c)]);
    assert_eq!(meet, expected);
    // Commutativity, idempotence on the normalized forms.
    assert_eq!(
        union_ab.intersection(&union_c),
        union_c.intersection(&union_ab)
    );
    assert_eq!(union_ab.intersection(&union_ab), union_ab);
    // Meet with the empty union → empty union.
    assert_eq!(union_ab.intersection(&EMPTY), EMPTY);
    assert_eq!(EMPTY.intersection(&union_ab), EMPTY);
    // Meet of two disjoint unions → empty union.
    let loin = Cdd::new(&[Dbm::new(2, &[(1, 0, -3)])]); // x − y ≥ 3
    assert_eq!(union_c.intersection(&loin), EMPTY);
}

#[test]
fn test_coverage_box_bounded() {
    let keys = CddKeys::new(SIZE_DOMAINE);
    let a = Dbm::new(2, &[(0, 1, 0)]);
    let b = Dbm::new(2, &[(1, 0, -2)]);
    let union = Cdd::new(&[a, b]);
    let complementary = Cdd::new(&[Dbm::new(2, &[(1, 0, -1), (0, 1, 1)])]);
    // The partition {not-convex union, complement} covers the box [0, 4)².
    assert!(keys.coverage(&[union.clone(), complementary.clone()]));
    // The union alone does not cover; neither does the empty family; nor the empty union.
    assert!(!keys.coverage(&[union]));
    assert!(!keys.coverage(&[]));
    assert!(!keys.coverage(&[EMPTY]));
}

/// Instance-test: the not-convex region compiled by explicit regions has the
/// expected semantics over the whole box, and fits in O(1) nodes against
/// T² points — the universe sweep does not compiled, the free DDD kingdom
/// is assumed.
#[test]
fn test_union_clocks_differential_against_enumeration() {
    let mut pool = pool();
    let root = compile_union_clocks(&mut pool, ANCHOR);
    // Three nodes: true, false, the root — against 16 points.
    assert_eq!(count_nodes(&pool, root).expect("interned node"), 3);
    // A not-interned NodeId is a propagated error, not a silent leaf.
    assert!(count_nodes(&pool, NodeId(u32::MAX)).is_err());
    for x in 0..SIZE_DOMAINE {
        for y in 0..SIZE_DOMAINE {
            let point = [x, y];
            let expected = i128::from(x) - i128::from(y) <= 0 || i128::from(x) - i128::from(y) >= 2;
            let lu = evaluer(&pool, root, &point).expect("interned node");
            assert_eq!(
                lu.unwrap_or(false),
                expected,
                "point {point:?} : the continuous absent is false"
            );
        }
    }
}

/// Relative C1: two constructions of the same union share the root in the
/// pool (R-share after the syntactic normal form of the union).
#[test]
fn test_shared_pool_union() {
    let mut pool = pool();
    let root_a = compile_union_clocks(&mut pool, ANCHOR);
    let root_b = compile_union_clocks(&mut pool, ANCHOR);
    assert_eq!(root_a, root_b, "same union = same root");
    // The root does have two edges at level 0 (union + complement).
    let Form::Node { edges, .. } = pool.resolve(root_a).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(edges.len(), 2);
    let keys = CddKeys::new(SIZE_DOMAINE);
    let regions: Vec<Cdd> = edges.keys().cloned().collect();
    assert!(keys.coverage(&regions), "the level-0 edges cover the box");
}

/// Property (LCG): the intersection of unions is commutative, idempotent,
/// and every point of the meet belongs to both unions.
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
    let drawn_zone = |next: &mut dyn FnMut() -> u64| {
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
    let drawn_union = |next: &mut dyn FnMut() -> u64| {
        let number_zones = 1 + next() % 3;
        let zones: Vec<Dbm> = (0..number_zones).map(|_| drawn_zone(next)).collect();
        Cdd::new(&zones)
    };
    for _ in 0..NUMBER_CAS {
        let a = drawn_union(&mut next);
        let b = drawn_union(&mut next);
        let meet = a.intersection(&b);
        assert_eq!(meet, b.intersection(&a), "commutativity");
        // Semantic idempotence (not syntactic: the meet adds the
        // cross-intersections of the zones, assumed not-canonicity).
        let auto_meet = a.intersection(&a);
        for x in 0..SIZE_DOMAINE {
            for y in 0..SIZE_DOMAINE {
                let point = [x, y];
                assert_eq!(
                    auto_meet.contains(&point),
                    a.contains(&point),
                    "semantic idempotence at point {point:?}"
                );
            }
        }
        let point = [next() % SIZE_DOMAINE, next() % SIZE_DOMAINE];
        if meet.contains(&point) {
            assert!(a.contains(&point) && b.contains(&point));
        }
    }
}

/// The key contract remains declared on the controller side: cardinality and
/// default of the level are those of the CDD algebra (coverage is tested separately).
#[test]
fn test_contract_key_level() {
    let keys = CddKeys::new(SIZE_DOMAINE);
    assert_eq!(keys.cardinality(Level(0)), SIZE_DOMAINE);
    assert_eq!(keys.default(Level(0)), Some(EMPTY));
}
