//! Tests for the CDD test instance (backlog § 9, open question 9 of the
//! spec, vein #83): non-convex unions of DBM zones as clock region-keys —
//! syntactic normal form (sort + dedup, non-canonicity assumed), decidable
//! membership, distributive intersection, coverage of the bounded box — then
//! the diagram of the non-convex region `{x − y ≤ 0} ∪ {x − y ≥ 2}` built by
//! explicit regions: membership differential against naive enumeration over a
//! small box, and pool sharing (C1 relative to the union's normal form).

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::cdd::{
    Cdd, CddController, CddKeys, VIDE, compiler_union_horloges, compter_noeuds, evaluer,
};
use lisage_core::dbm::Dbm;
use lisage_core::observer::NoOpObserver;

type PoolCdd = Arena<CddController>;

const ANCRE: Anchor = Anchor(0);
const TAILLE_DOMAINE: u64 = 4;

fn pool() -> PoolCdd {
    Arena::new(
        CddController::nouveau(CddKeys::nouvelle(TAILLE_DOMAINE)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_normalisation_union() {
    let a = Dbm::nouveau(2, &[(0, 1, 0)]);
    let b = Dbm::nouveau(2, &[(1, 0, -2)]);
    // Sort + dedup: the writing [a, b, a] normalizes to the derived order [b, a].
    let union = Cdd::nouveau(&[a.clone(), b.clone(), a.clone()]);
    assert_eq!(union.zones(), &[b.clone(), a.clone()]);
    assert!(!union.est_vide());
    // Empty zones are removed; zero zones = canonical empty union.
    let avec_vide = Cdd::nouveau(&[a.clone(), lisage_core::dbm::INCOHERENT]);
    assert_eq!(avec_vide.zones(), &[a]);
    assert_eq!(Cdd::nouveau(&[]), VIDE);
    assert!(VIDE.est_vide());
    assert!(Cdd::nouveau(&[lisage_core::dbm::INCOHERENT]).est_vide());
}

/// Assumed non-canonicity (spec Q9 (i)): two writings of the same non-convex
/// set remain distinct keys if the syntactic normal form differs — here a
/// zone included in another.
#[test]
fn test_non_canonicite_assumee() {
    let grande = Dbm::nouveau(2, &[(0, 1, 5)]);
    let incluse = Dbm::nouveau(2, &[(0, 1, 0)]);
    assert!(grande.contient(&[0, 0]) && incluse.contient(&[0, 0]));
    let union_avec_redondance = Cdd::nouveau(&[grande.clone(), incluse.clone()]);
    let union_seule = Cdd::nouveau(std::slice::from_ref(&grande));
    // Semantically the same region (the included one is inside the big one)…
    for x in 0..TAILLE_DOMAINE {
        for y in 0..TAILLE_DOMAINE {
            assert_eq!(
                union_avec_redondance.contient(&[x, y]),
                union_seule.contient(&[x, y])
            );
        }
    }
    // …but not the same key: no known set-theoretic normal form.
    assert_ne!(union_avec_redondance, union_seule);
}

#[test]
fn test_contient_semantique() {
    let a = Dbm::nouveau(2, &[(0, 1, 0)]); // x − y ≤ 0
    let b = Dbm::nouveau(2, &[(1, 0, -2)]); // x − y ≥ 2
    let union = Cdd::nouveau(&[a, b]);
    assert!(union.contient(&[0, 0]));
    assert!(union.contient(&[1, 3]));
    assert!(union.contient(&[3, 1]));
    assert!(!union.contient(&[2, 1])); // 0 < x − y < 2
    assert!(!VIDE.contient(&[0, 0]));
}

#[test]
fn test_intersection_distributive() {
    let a = Dbm::nouveau(2, &[(0, 1, 0)]);
    let b = Dbm::nouveau(2, &[(1, 0, -2)]);
    let c = Dbm::nouveau(2, &[(0, 1, 1)]); // x − y ≤ 1
    let union_ab = Cdd::nouveau(&[a.clone(), b.clone()]);
    let union_c = Cdd::nouveau(std::slice::from_ref(&c));
    // (A ∪ B) ∩ C = (A ∩ C) ∪ (B ∩ C); B ∩ C is empty (x−y ≥ 2 and x−y ≤ 1).
    let meet = union_ab.intersection(&union_c);
    let attendu = Cdd::nouveau(&[a.intersection(&c)]);
    assert_eq!(meet, attendu);
    // Commutativity, idempotence on normalized forms.
    assert_eq!(
        union_ab.intersection(&union_c),
        union_c.intersection(&union_ab)
    );
    assert_eq!(union_ab.intersection(&union_ab), union_ab);
    // Meet with the empty union → empty union.
    assert_eq!(union_ab.intersection(&VIDE), VIDE);
    assert_eq!(VIDE.intersection(&union_ab), VIDE);
    // Meet of two disjoint unions → empty union.
    let loin = Cdd::nouveau(&[Dbm::nouveau(2, &[(1, 0, -3)])]); // x − y ≥ 3
    assert_eq!(union_c.intersection(&loin), VIDE);
}

#[test]
fn test_couverture_boite_bornee() {
    let keys = CddKeys::nouvelle(TAILLE_DOMAINE);
    let a = Dbm::nouveau(2, &[(0, 1, 0)]);
    let b = Dbm::nouveau(2, &[(1, 0, -2)]);
    let union = Cdd::nouveau(&[a, b]);
    let complementaire = Cdd::nouveau(&[Dbm::nouveau(2, &[(1, 0, -1), (0, 1, 1)])]);
    // The partition {non-convex union, complement} covers the box [0, 4)².
    assert!(keys.couverture(&[union.clone(), complementaire.clone()]));
    // The union alone does not cover; neither does the empty family; nor the empty union.
    assert!(!keys.couverture(&[union]));
    assert!(!keys.couverture(&[]));
    assert!(!keys.couverture(&[VIDE]));
}

/// Test instance: the non-convex region compiled by explicit regions has the
/// expected semantics over the whole box, and fits in O(1) nodes vs T² points —
/// universe sweep does not compile, the free DDD kingdom is assumed.
#[test]
fn test_union_horloges_differentiel_contre_enumeration() {
    let mut pool = pool();
    let racine = compiler_union_horloges(&mut pool, ANCRE);
    // Three nodes: true, false, the root — vs 16 points.
    assert_eq!(compter_noeuds(&pool, racine).expect("interned node"), 3);
    // A non-interned NodeId is a propagated error, not a silent leaf.
    assert!(compter_noeuds(&pool, NodeId(u32::MAX)).is_err());
    for x in 0..TAILLE_DOMAINE {
        for y in 0..TAILLE_DOMAINE {
            let point = [x, y];
            let attendu = i128::from(x) - i128::from(y) <= 0 || i128::from(x) - i128::from(y) >= 2;
            let lu = evaluer(&pool, racine, &point).expect("interned node");
            assert_eq!(
                lu.unwrap_or(false),
                attendu,
                "point {point:?}: the continuous absent is false"
            );
        }
    }
}

/// Relative C1: two constructions of the same union share the root in the
/// pool (R-share after the union's syntactic normal form).
#[test]
fn test_partage_pool_union() {
    let mut pool = pool();
    let racine_a = compiler_union_horloges(&mut pool, ANCRE);
    let racine_b = compiler_union_horloges(&mut pool, ANCRE);
    assert_eq!(racine_a, racine_b, "same union = same root");
    // The root indeed has two arcs at level 0 (union + complement).
    let Form::Node { arcs, .. } = pool.resolve(racine_a).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 2);
    let keys = CddKeys::nouvelle(TAILLE_DOMAINE);
    let regions: Vec<Cdd> = arcs.keys().cloned().collect();
    assert!(keys.couverture(&regions), "the level-0 arcs cover the box");
}

/// Property (LCG): the intersection of unions is commutative, idempotent,
/// and every point of the meet belongs to both unions.
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
    let tire_zone = |suivant: &mut dyn FnMut() -> u64| {
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
    let tire_union = |suivant: &mut dyn FnMut() -> u64| {
        let nombre_zones = 1 + suivant() % 3;
        let zones: Vec<Dbm> = (0..nombre_zones).map(|_| tire_zone(suivant)).collect();
        Cdd::nouveau(&zones)
    };
    for _ in 0..NOMBRE_CAS {
        let a = tire_union(&mut suivant);
        let b = tire_union(&mut suivant);
        let meet = a.intersection(&b);
        assert_eq!(meet, b.intersection(&a), "commutativity");
        // Semantic idempotence (not syntactic: the meet adds the cross
        // intersections of the zones, assumed non-canonicity).
        let auto_meet = a.intersection(&a);
        for x in 0..TAILLE_DOMAINE {
            for y in 0..TAILLE_DOMAINE {
                let point = [x, y];
                assert_eq!(
                    auto_meet.contient(&point),
                    a.contient(&point),
                    "semantic idempotence at point {point:?}"
                );
            }
        }
        let point = [suivant() % TAILLE_DOMAINE, suivant() % TAILLE_DOMAINE];
        if meet.contient(&point) {
            assert!(a.contient(&point) && b.contient(&point));
        }
    }
}

/// The key contract remains declared on the controller side: cardinality and
/// level default are those of the CDD algebra (coverage is tested separately).
#[test]
fn test_contrat_cle_niveau() {
    let keys = CddKeys::nouvelle(TAILLE_DOMAINE);
    assert_eq!(keys.cardinality(Level(0)), TAILLE_DOMAINE);
    assert_eq!(keys.default(Level(0)), Some(VIDE));
}
