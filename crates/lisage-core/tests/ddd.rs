//! Tests for the DDD-difference test instance (backlog § 9, open question 9
//! of the spec, vein #83): key contract on the controller side (intersection =
//! meet of DBMs, coverage of the bounded box), then the diagram of the band
//! `x − y ≤ 1 ∧ y − x ≤ 2` built by explicit regions — membership differential
//! against naive enumeration over a small box, and pool sharing (C1 relative
//! to the region algebra).

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCOHERENT};
use lisage_core::ddd::{compiler_bande, compter_noeuds, evaluer};
use lisage_core::observer::NoOpObserver;

type PoolDbm = Arena<DbmController>;

const ANCRE: Anchor = Anchor(0);
const TAILLE_DOMAINE: u64 = 4;

fn pool() -> PoolDbm {
    Arena::new(
        DbmController::nouveau(DbmKeys::nouvelle(TAILLE_DOMAINE)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_intersection_meet_canonique() {
    // meet of x−y ≤ 1 and y−x ≤ 2 = the explicit band, normal form included.
    let a = Dbm::nouveau(2, &[(0, 1, 1)]);
    let b = Dbm::nouveau(2, &[(1, 0, 2)]);
    let meet = a.intersection(&b);
    let bande = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(meet, bande, "the meet of two regions is the band, closed");
    // Commutativity, idempotence.
    assert_eq!(a.intersection(&b), b.intersection(&a));
    assert_eq!(a.intersection(&a), a);
    // Meet with a disjoint region: inconsistent → canonical empty region.
    let disjoint = Dbm::nouveau(2, &[(1, 0, -3)]);
    assert_eq!(a.intersection(&disjoint), INCOHERENT);
    // Any empty region gives the empty region.
    assert_eq!(a.intersection(&INCOHERENT), INCOHERENT);
    assert_eq!(INCOHERENT.intersection(&a), INCOHERENT);
}

#[test]
fn test_couverture_boite_bornee() {
    let keys = DbmKeys::nouvelle(TAILLE_DOMAINE);
    let dessous = Dbm::nouveau(2, &[(0, 1, 0)]);
    let dessus = Dbm::nouveau(2, &[(1, 0, -1)]);
    // The partition {x ≤ y, x > y} covers the box [0, 4)².
    assert!(keys.couverture(&[dessous.clone(), dessus.clone()]));
    // A single half does not cover; neither does the empty family; nor the empty region.
    assert!(!keys.couverture(&[dessous]));
    assert!(!keys.couverture(&[]));
    assert!(!keys.couverture(&[INCOHERENT]));
    // The band alone does not cover the box.
    let bande = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 2)]);
    assert!(!keys.couverture(&[bande]));
}

/// Test instance: the band compiled by explicit regions has the expected
/// semantics over the whole box, and fits in O(1) nodes vs T² points — the
/// universe sweep does not compile, the free DDD kingdom is assumed.
#[test]
fn test_bande_differentiel_contre_enumeration() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    // Four nodes: true, false, level 1, the root — vs 16 points.
    assert_eq!(compter_noeuds(&pool, racine), 4);
    for x in 0..TAILLE_DOMAINE {
        for y in 0..TAILLE_DOMAINE {
            let point = [x, y];
            let attendu = i128::from(x) - i128::from(y) <= 1 && i128::from(y) - i128::from(x) <= 2;
            let lu = evaluer(&pool, racine, &point).expect("interned node");
            assert_eq!(
                lu.unwrap_or(false),
                attendu,
                "point {point:?}: the continuous absent is false"
            );
        }
    }
}

/// Relative C1: two constructions of the same band share the root in the pool
/// (R-share after the regions' normal form).
#[test]
fn test_partage_pool_bande() {
    let mut pool = pool();
    let racine_a = compiler_bande(&mut pool, ANCRE);
    let racine_b = compiler_bande(&mut pool, ANCRE);
    assert_eq!(racine_a, racine_b, "same band = same root");
    // The root indeed has two arcs at level 0 (region + complement).
    let Form::Node { arcs, .. } = pool.resolve(racine_a).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 2);
    let keys = DbmKeys::nouvelle(TAILLE_DOMAINE);
    let regions: Vec<Dbm> = arcs.keys().cloned().collect();
    assert!(keys.couverture(&regions), "the level-0 arcs cover the box");
}

/// Property (LCG): the intersection is commutative, idempotent, and every
/// point of the meet belongs to both regions.
#[test]
fn test_propriete_intersection_lcg() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    let mut suivant = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    let tire = |suivant: &mut dyn FnMut() -> u64| {
        let nombre_contraintes = suivant() % 4;
        let contraintes: Vec<(usize, usize, i64)> = (0..nombre_contraintes)
            .map(|_| {
                (
                    (suivant() % 2) as usize,
                    (suivant() % 2) as usize,
                    (suivant() % 7) as i64 - 3,
                )
            })
            .collect();
        Dbm::nouveau(2, &contraintes)
    };
    for _ in 0..NOMBRE_CAS {
        let a = tire(&mut suivant);
        let b = tire(&mut suivant);
        let meet = a.intersection(&b);
        assert_eq!(meet, b.intersection(&a), "commutativity");
        assert_eq!(a.intersection(&a), a, "idempotence");
        let point = [suivant() % TAILLE_DOMAINE, suivant() % TAILLE_DOMAINE];
        if meet.contient(&point) {
            assert!(a.contient(&point) && b.contient(&point));
        }
    }
}

/// The key contract remains declared on the controller side: cardinality and
/// level default are those of the DBM algebra (coverage is tested separately).
#[test]
fn test_contrat_cle_niveau() {
    let keys = DbmKeys::nouvelle(TAILLE_DOMAINE);
    assert_eq!(keys.cardinality(Level(0)), TAILLE_DOMAINE);
    assert_eq!(keys.default(Level(0)), Some(INCOHERENT));
}
