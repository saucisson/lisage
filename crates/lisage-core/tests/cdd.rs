//! Tests de l'instance-test CDD (backlog § 9, question ouverte 9 de la
//! spec, veine #83) : unions non convexes de zones DBM comme clés-régions
//! d'horloges — forme normale syntaxique (tri + dédup, non-canonicité
//! assumée), appartenance décidable, intersection distributive, couverture
//! de la boîte bornée — puis le diagramme de la région non convexe
//! `{x − y ≤ 0} ∪ {x − y ≥ 2}` construit par régions explicites :
//! différentiel d'appartenance contre énumération naïve sur une petite
//! boîte, et partage au pool (C1 relatif à la forme normale de l'union).

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
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
    // Tri + dédup : l'écriture [a, b, a] normalise à l'ordre dérivé [b, a].
    let union = Cdd::nouveau(&[a.clone(), b.clone(), a.clone()]);
    assert_eq!(union.zones(), &[b.clone(), a.clone()]);
    assert!(!union.est_vide());
    // Les zones vides sont retirées ; zéro zone = union vide canonique.
    let avec_vide = Cdd::nouveau(&[a.clone(), lisage_core::dbm::INCOHERENT]);
    assert_eq!(avec_vide.zones(), &[a]);
    assert_eq!(Cdd::nouveau(&[]), VIDE);
    assert!(VIDE.est_vide());
    assert!(Cdd::nouveau(&[lisage_core::dbm::INCOHERENT]).est_vide());
}

/// Non-canonicité assumée (spec Q9 (i)) : deux écritures du même ensemble
/// non convexe restent des clés distinctes si la forme normale syntaxique
/// diffère — ici une zone incluse dans une autre.
#[test]
fn test_non_canonicite_assumee() {
    let grande = Dbm::nouveau(2, &[(0, 1, 5)]);
    let incluse = Dbm::nouveau(2, &[(0, 1, 0)]);
    assert!(grande.contient(&[0, 0]) && incluse.contient(&[0, 0]));
    let union_avec_redondance = Cdd::nouveau(&[grande.clone(), incluse.clone()]);
    let union_seule = Cdd::nouveau(std::slice::from_ref(&grande));
    // Sémantiquement la même région (l'incluse est dans la grande)…
    for x in 0..TAILLE_DOMAINE {
        for y in 0..TAILLE_DOMAINE {
            assert_eq!(
                union_avec_redondance.contient(&[x, y]),
                union_seule.contient(&[x, y])
            );
        }
    }
    // …mais pas la même clé : pas de forme normale ensembliste connue.
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
    // (A ∪ B) ∩ C = (A ∩ C) ∪ (B ∩ C) ; B ∩ C est vide (x−y ≥ 2 et x−y ≤ 1).
    let meet = union_ab.intersection(&union_c);
    let attendu = Cdd::nouveau(&[a.intersection(&c)]);
    assert_eq!(meet, attendu);
    // Commutativité, idempotence sur les formes normalisées.
    assert_eq!(
        union_ab.intersection(&union_c),
        union_c.intersection(&union_ab)
    );
    assert_eq!(union_ab.intersection(&union_ab), union_ab);
    // Meet avec l'union vide → union vide.
    assert_eq!(union_ab.intersection(&VIDE), VIDE);
    assert_eq!(VIDE.intersection(&union_ab), VIDE);
    // Meet de deux unions disjointes → union vide.
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
    // La partition {union non convexe, complémentaire} couvre la boîte [0, 4)².
    assert!(keys.couverture(&[union.clone(), complementaire.clone()]));
    // L'union seule ne couvre pas ; famille vide non plus ; union vide non plus.
    assert!(!keys.couverture(&[union]));
    assert!(!keys.couverture(&[]));
    assert!(!keys.couverture(&[VIDE]));
}

/// Instance-test : la région non convexe compilée par régions explicites a la
/// sémantique attendue sur toute la boîte, et tient en O(1) nœuds contre
/// T² points — le balayage d'univers ne compile pas, le royaume libre DDD
/// est assumé.
#[test]
fn test_union_horloges_differentiel_contre_enumeration() {
    let mut pool = pool();
    let racine = compiler_union_horloges(&mut pool, ANCRE);
    // Trois nœuds : true, false, la racine — contre 16 points.
    assert_eq!(compter_noeuds(&pool, racine), 3);
    for x in 0..TAILLE_DOMAINE {
        for y in 0..TAILLE_DOMAINE {
            let point = [x, y];
            let attendu = i128::from(x) - i128::from(y) <= 0 || i128::from(x) - i128::from(y) >= 2;
            let lu = evaluer(&pool, racine, &point).expect("nœud interné");
            assert_eq!(
                lu.unwrap_or(false),
                attendu,
                "point {point:?} : l'absent continu vaut false"
            );
        }
    }
}

/// C1 relatif : deux constructions de la même union partagent la racine au
/// pool (R-share après forme normale syntaxique de l'union).
#[test]
fn test_partage_pool_union() {
    let mut pool = pool();
    let racine_a = compiler_union_horloges(&mut pool, ANCRE);
    let racine_b = compiler_union_horloges(&mut pool, ANCRE);
    assert_eq!(racine_a, racine_b, "même union = même racine");
    // La racine a bien deux arcs au niveau 0 (union + complémentaire).
    let Form::Node { arcs, .. } = pool.resolve(racine_a).expect("nœud interné") else {
        panic!("la racine est un nœud");
    };
    assert_eq!(arcs.len(), 2);
    let keys = CddKeys::nouvelle(TAILLE_DOMAINE);
    let regions: Vec<Cdd> = arcs.keys().cloned().collect();
    assert!(
        keys.couverture(&regions),
        "les arcs du niveau 0 couvrent la boîte"
    );
}

/// Propriété (LCG) : l'intersection d'unions est commutative, idempotente,
/// et tout point du meet appartient aux deux unions.
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
        assert_eq!(meet, b.intersection(&a), "commutativité");
        // Idempotence sémantique (et non syntaxique : le meet ajoute les
        // intersections croisées des zones, non-canonicité assumée).
        let auto_meet = a.intersection(&a);
        for x in 0..TAILLE_DOMAINE {
            for y in 0..TAILLE_DOMAINE {
                let point = [x, y];
                assert_eq!(
                    auto_meet.contient(&point),
                    a.contient(&point),
                    "idempotence sémantique au point {point:?}"
                );
            }
        }
        let point = [suivant() % TAILLE_DOMAINE, suivant() % TAILLE_DOMAINE];
        if meet.contient(&point) {
            assert!(a.contient(&point) && b.contient(&point));
        }
    }
}

/// Le contrat de clés reste déclaré côté contrôleur : cardinalité et défaut
/// du niveau sont ceux de l'algèbre CDD (la couverture se teste à part).
#[test]
fn test_contrat_cle_niveau() {
    let keys = CddKeys::nouvelle(TAILLE_DOMAINE);
    assert_eq!(keys.cardinality(Level(0)), TAILLE_DOMAINE);
    assert_eq!(keys.default(Level(0)), Some(VIDE));
}
