//! Tests for the DDD-difference instance-test (backlog § 9, open question 9
//! of the spec, thread #83): key contract on the controller side
//! (intersection = DBM meet, coverage of the bounded box), then the diagram
//! of the band `x − y ≤ 1 ∧ y − x ≤ 2` built by explicit regions —
//! membership differential against naive enumeration on a small box, and
//! pool sharing (C1 relative to the region algebra).

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCOHERENT};
use lisage_core::ddd::{compiler_bande, compter_nodes, evaluer};
use lisage_core::observer::NoOpObserver;

type PoolDbm = Arena<DbmController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAIN: u64 = 4;

fn pool() -> PoolDbm {
    Arena::new(
        DbmController::new(DbmKeys::new(SIZE_DOMAIN)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_intersection_meet_canonique() {
    // meet of x−y ≤ 1 and y−x ≤ 2 = the explicit band, normal form included.
    let a = Dbm::new(2, &[(0, 1, 1)]);
    let b = Dbm::new(2, &[(1, 0, 2)]);
    let meet = a.intersection(&b);
    let bande = Dbm::new(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(meet, bande, "the meet of two regions is the band, closed");
    // Commutativity, idempotence.
    assert_eq!(a.intersection(&b), b.intersection(&a));
    assert_eq!(a.intersection(&a), a);
    // Meet with a disjoint region: incoherent → canonical empty region.
    let disjoint = Dbm::new(2, &[(1, 0, -3)]);
    assert_eq!(a.intersection(&disjoint), INCOHERENT);
    // Any empty region gives the empty region.
    assert_eq!(a.intersection(&INCOHERENT), INCOHERENT);
    assert_eq!(INCOHERENT.intersection(&a), INCOHERENT);
}

#[test]
fn test_couverture_boite_bounded() {
    let keys = DbmKeys::new(SIZE_DOMAIN);
    let dessous = Dbm::new(2, &[(0, 1, 0)]);
    let dessus = Dbm::new(2, &[(1, 0, -1)]);
    // The partition {x ≤ y, x > y} covers the box [0, 4)².
    assert!(keys.couverture(&[dessous.clone(), dessus.clone()]));
    // A single half does not cover; neither does the empty family; neither does the empty region.
    assert!(!keys.couverture(&[dessous]));
    assert!(!keys.couverture(&[]));
    assert!(!keys.couverture(&[INCOHERENT]));
    // The band alone does not cover the box.
    let bande = Dbm::new(2, &[(0, 1, 1), (1, 0, 2)]);
    assert!(!keys.couverture(&[bande]));
}

/// Instance-test: the band compiled by explicit regions has the expected
/// semantics over the whole box, and fits in O(1) nodes against T² points —
/// universe sweep does not compile, the free DDD domain is assumed.
#[test]
fn test_bande_differentiel_contre_enumeration() {
    let mut pool = pool();
    let root = compiler_bande(&mut pool, ANCHOR);
    // Four nodes: true, false, level 1, the root — against 16 points.
    assert_eq!(compter_nodes(&pool, root), 4);
    for x in 0..SIZE_DOMAIN {
        for y in 0..SIZE_DOMAIN {
            let point = [x, y];
            let expected = i128::from(x) - i128::from(y) <= 1 && i128::from(y) - i128::from(x) <= 2;
            let lu = evaluer(&pool, root, &point).expect("interned node");
            assert_eq!(
                lu.unwrap_or(false),
                expected,
                "point {point:?}: the continuous absent means false"
            );
        }
    }
}

/// Relative C1: two constructions of the same band share the root in the
/// pool (R-share after normal form of the regions).
#[test]
fn test_partage_pool_bande() {
    let mut pool = pool();
    let root_a = compiler_bande(&mut pool, ANCHOR);
    let root_b = compiler_bande(&mut pool, ANCHOR);
    assert_eq!(root_a, root_b, "same band = same root");
    // The root really has two arcs at level 0 (region + complement).
    let Form::Node { arcs, .. } = pool.resolve(root_a).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 2);
    let keys = DbmKeys::new(SIZE_DOMAIN);
    let regions: Vec<Dbm> = arcs.keys().cloned().collect();
    assert!(keys.couverture(&regions), "the level-0 arcs cover the box");
}

/// Property (LCG): the intersection is commutative, idempotent, and every
/// point of the meet belongs to both regions.
#[test]
fn test_propriete_intersection_lcg() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
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
    let tire = |next: &mut dyn FnMut() -> u64| {
        let number_contraintes = next() % 4;
        let contraintes: Vec<(usize, usize, i64)> = (0..number_contraintes)
            .map(|_| {
                (
                    (next() % 2) as usize,
                    (next() % 2) as usize,
                    (next() % 7) as i64 - 3,
                )
            })
            .collect();
        Dbm::new(2, &contraintes)
    };
    for _ in 0..NUMBER_CAS {
        let a = tire(&mut next);
        let b = tire(&mut next);
        let meet = a.intersection(&b);
        assert_eq!(meet, b.intersection(&a), "commutativity");
        assert_eq!(a.intersection(&a), a, "idempotence");
        let point = [next() % SIZE_DOMAIN, next() % SIZE_DOMAIN];
        if meet.contient(&point) {
            assert!(a.contient(&point) && b.contient(&point));
        }
    }
}

/// The key contract stays declared on the controller side: level cardinality
/// and default are those of the DBM algebra (coverage is tested separately).
#[test]
fn test_contrat_key_level() {
    let keys = DbmKeys::new(SIZE_DOMAIN);
    assert_eq!(keys.cardinality(Level(0)), SIZE_DOMAIN);
    assert_eq!(keys.default(Level(0)), Some(INCOHERENT));
}
