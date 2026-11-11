//! Tests de l'instance-test DDD-différence (backlog § 9, question ouverte 9
//! de la spec, veine #83) : contrat de clés côté contrôleur (intersection =
//! meet des DBM, couverture de la boîte bornée), puis le diagramme de la
//! bande `x − y ≤ 1 ∧ y − x ≤ 2` construit par régions explicites —
//! différentiel d'appartenance contre énumération naïve sur une petite
//! boîte, et partage au pool (C1 relatif à l'algèbre de régions).

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
    // meet de x−y ≤ 1 et y−x ≤ 2 = la bande explicite, forme normale incluse.
    let a = Dbm::nouveau(2, &[(0, 1, 1)]);
    let b = Dbm::nouveau(2, &[(1, 0, 2)]);
    let meet = a.intersection(&b);
    let bande = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(
        meet, bande,
        "le meet de deux régions est la bande, clôturée"
    );
    // Commutativité, idempotence.
    assert_eq!(a.intersection(&b), b.intersection(&a));
    assert_eq!(a.intersection(&a), a);
    // Meet avec une région disjointe : incohérent → région vide canonique.
    let disjoint = Dbm::nouveau(2, &[(1, 0, -3)]);
    assert_eq!(a.intersection(&disjoint), INCOHERENT);
    // Toute région vide donne la région vide.
    assert_eq!(a.intersection(&INCOHERENT), INCOHERENT);
    assert_eq!(INCOHERENT.intersection(&a), INCOHERENT);
}

#[test]
fn test_couverture_boite_bornee() {
    let keys = DbmKeys::nouvelle(TAILLE_DOMAINE);
    let dessous = Dbm::nouveau(2, &[(0, 1, 0)]);
    let dessus = Dbm::nouveau(2, &[(1, 0, -1)]);
    // La partition {x ≤ y, x > y} couvre la boîte [0, 4)².
    assert!(keys.couverture(&[dessous.clone(), dessus.clone()]));
    // Une seule moitié ne couvre pas ; famille vide non plus ; région vide non plus.
    assert!(!keys.couverture(&[dessous]));
    assert!(!keys.couverture(&[]));
    assert!(!keys.couverture(&[INCOHERENT]));
    // La bande seule ne couvre pas la boîte.
    let bande = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 2)]);
    assert!(!keys.couverture(&[bande]));
}

/// Instance-test : la bande compilée par régions explicites a la sémantique
/// attendue sur toute la boîte, et tient en O(1) nœuds contre T² points —
/// le balayage d'univers ne compile pas, le royaume libre DDD est assumé.
#[test]
fn test_bande_differentiel_contre_enumeration() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    // Quatre nœuds : true, false, le niveau 1, la racine — contre 16 points.
    assert_eq!(compter_noeuds(&pool, racine), 4);
    for x in 0..TAILLE_DOMAINE {
        for y in 0..TAILLE_DOMAINE {
            let point = [x, y];
            let attendu = i128::from(x) - i128::from(y) <= 1 && i128::from(y) - i128::from(x) <= 2;
            let lu = evaluer(&pool, racine, &point).expect("nœud interné");
            assert_eq!(
                lu.unwrap_or(false),
                attendu,
                "point {point:?} : l'absent continu vaut false"
            );
        }
    }
}

/// C1 relatif : deux constructions de la même bande partagent la racine au
/// pool (R-share après forme normale des régions).
#[test]
fn test_partage_pool_bande() {
    let mut pool = pool();
    let racine_a = compiler_bande(&mut pool, ANCRE);
    let racine_b = compiler_bande(&mut pool, ANCRE);
    assert_eq!(racine_a, racine_b, "même bande = même racine");
    // La racine a bien deux arcs au niveau 0 (région + complémentaire).
    let Form::Node { arcs, .. } = pool.resolve(racine_a).expect("nœud interné") else {
        panic!("la racine est un nœud");
    };
    assert_eq!(arcs.len(), 2);
    let keys = DbmKeys::nouvelle(TAILLE_DOMAINE);
    let regions: Vec<Dbm> = arcs.keys().cloned().collect();
    assert!(
        keys.couverture(&regions),
        "les arcs du niveau 0 couvrent la boîte"
    );
}

/// Propriété (LCG) : l'intersection est commutative, idempotente, et tout
/// point du meet appartient aux deux régions.
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
        assert_eq!(meet, b.intersection(&a), "commutativité");
        assert_eq!(a.intersection(&a), a, "idempotence");
        let point = [suivant() % TAILLE_DOMAINE, suivant() % TAILLE_DOMAINE];
        if meet.contient(&point) {
            assert!(a.contient(&point) && b.contient(&point));
        }
    }
}

/// Le contrat de clés reste déclaré côté contrôleur : cardinalité et défaut
/// du niveau sont ceux de l'algèbre DBM (la couverture se teste à part).
#[test]
fn test_contrat_cle_niveau() {
    let keys = DbmKeys::nouvelle(TAILLE_DOMAINE);
    assert_eq!(keys.cardinality(Level(0)), TAILLE_DOMAINE);
    assert_eq!(keys.default(Level(0)), Some(INCOHERENT));
}
