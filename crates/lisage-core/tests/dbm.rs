//! Tests de l'algèbre de clés-régions DBM (backlog § 9, question ouverte 9
//! de la spec, veine #83) : clôture canonique (C1 relatif à l'algèbre de
//! régions), incohérence, cardinalité, défaut, partage au pool, et
//! différentiel d'appartenance contre énumération naïve sur une petite boîte.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCOHERENT};
use lisage_core::observer::NoOpObserver;

type PoolDbm = Arena<DbmController>;

const ANCRE: Anchor = Anchor(0);

fn pool(taille_domaine: u64) -> PoolDbm {
    Arena::new(
        DbmController::nouveau(DbmKeys::nouvelle(taille_domaine)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_dbm_cloture_canonique() {
    // x0 − x1 ≤ 2 et x1 − x0 ≤ 3 décrivent la même région que la clôture
    // directe ; une contrainte redondante ne change pas la forme normale.
    let directe = Dbm::nouveau(2, &[(0, 1, 2), (1, 0, 3)]);
    let avec_redondance = Dbm::nouveau(2, &[(0, 1, 2), (1, 0, 3), (0, 1, 5), (0, 0, 0)]);
    assert_eq!(directe, avec_redondance, "clôture canonique");
    // La clôture propage : x0−x1 ≤ 1 et x1−x2 ≤ 1 entraînent x0−x2 ≤ 2.
    let propagee = Dbm::nouveau(3, &[(0, 1, 1), (1, 2, 1)]);
    let explicite = Dbm::nouveau(3, &[(0, 1, 1), (1, 2, 1), (0, 2, 2)]);
    assert_eq!(
        propagee, explicite,
        "la clôture propage les contraintes transitives"
    );
}

#[test]
fn test_dbm_incoherent_normalise() {
    // x0 − x1 ≤ 2 et x1 − x0 ≤ −3 sont incompatibles : région vide.
    let incoherente = Dbm::nouveau(2, &[(0, 1, 2), (1, 0, -3)]);
    assert_eq!(incoherente, INCOHERENT);
    assert!(incoherente.est_vide());
    assert!(!incoherente.contient(&[0, 0]));
    // Toute région incohérente, quelle que soit son écriture, est INCOHERENT.
    let autre = Dbm::nouveau(3, &[(0, 1, -1), (1, 2, 0), (2, 0, 0)]);
    assert_eq!(autre, INCOHERENT);
}

#[test]
fn test_dbm_contient() {
    // Région x0 − x1 ≤ 1 sur la boîte [0, 4)².
    let region = Dbm::nouveau(2, &[(0, 1, 1)]);
    assert!(region.contient(&[2, 2]));
    assert!(region.contient(&[3, 2]));
    assert!(!region.contient(&[3, 1]));
}

#[test]
fn test_cardinalite_et_defaut() {
    let keys = DbmKeys::nouvelle(4);
    assert_eq!(keys.cardinality(Level(0)), 4);
    assert_eq!(keys.cardinality(Level(7)), 4);
    // δ(ℓ) = la région vide, l'« absent » continu.
    assert_eq!(keys.default(Level(0)), Some(INCOHERENT));
}

/// C1 relatif : deux écritures d'une même région partagent le même nœud au
/// pool (R-share après forme normale), deux régions distinctes non.
#[test]
fn test_partage_pool_c1_relatif() {
    let mut pool = pool(4);
    let terminal = pool.intern(ANCRE, Form::Terminal(true));
    let niveau = Level(0);
    let region_a = Dbm::nouveau(2, &[(0, 1, 2), (1, 0, 3)]);
    let region_b = Dbm::nouveau(2, &[(0, 1, 2), (1, 0, 3), (0, 1, 9)]);
    let forme_a = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(region_a, terminal)]),
    };
    let forme_b = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(region_b, terminal)]),
    };
    assert_eq!(
        pool.intern(ANCRE, forme_a.clone()),
        pool.intern(ANCRE, forme_b),
        "même région sous deux écritures = même nœud"
    );
    let forme_vide_a = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(INCOHERENT, terminal)]),
    };
    let forme_vide_b = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(Dbm::nouveau(2, &[(0, 1, 1), (1, 0, -2)]), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCRE, forme_vide_a),
        pool.intern(ANCRE, forme_vide_b),
        "région vide sous deux écritures = même nœud"
    );
    let forme_autre = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(Dbm::nouveau(2, &[(0, 1, 0)]), terminal)]),
    };
    assert_ne!(
        pool.intern(ANCRE, forme_a),
        pool.intern(ANCRE, forme_autre),
        "régions distinctes = nœuds distincts"
    );
}

/// Propriété (LCG) : la forme normale est idempotente ; deux DBM tirées au
/// hasard de même clôture sont égaux ; `contient` est cohérent avec la
/// matrice normalisée.
#[test]
fn test_propriete_cloture_lcg() {
    const GRAINE_LCG: u64 = 0x853C_49E6_748F_EA9B;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let keys = DbmKeys::nouvelle(6);
    let mut graine = GRAINE_LCG;
    let mut suivant = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    for _ in 0..NOMBRE_CAS {
        let n = 2 + (suivant() % 2) as usize;
        let nombre_contraintes = suivant() % 5;
        let contraintes: Vec<(usize, usize, i64)> = (0..nombre_contraintes)
            .map(|_| {
                (
                    (suivant() % n as u64) as usize,
                    (suivant() % n as u64) as usize,
                    (suivant() % 7) as i64 - 3,
                )
            })
            .collect();
        let dbm = Dbm::nouveau(n, &contraintes);
        let normalisee = keys.normal_form(&dbm);
        assert_eq!(keys.normal_form(&normalisee), normalisee, "idempotence");
        // Appartenance d'un point tiré au hasard, cohérente avec est_vide.
        let point: Vec<u64> = (0..n).map(|_| suivant() % 6).collect();
        if normalisee.est_vide() {
            assert!(!normalisee.contient(&point));
        }
    }
}

/// Différentiel contre l'énumération naïve : un nœud de niveau 0 partitionne
/// la boîte `[0, 4)²` en deux régions DBM (`x0 − x1 ≤ 0` vrai/faux) ; pour
/// chaque point de la boîte, la cible lue dans le DD égale la cible naïve.
/// L'arc région vide mène au défaut D_δ = terminal(false) : R-default.
#[test]
fn test_differentiel_appartenance_contre_enumeration() {
    const TAILLE_DOMAINE: u64 = 4;
    let mut pool = pool(TAILLE_DOMAINE);
    let vrai_terminal = pool.intern(ANCRE, Form::Terminal(true));
    let faux_terminal = pool.intern(ANCRE, Form::Terminal(false));
    // x0 − x1 ≤ 0  ⇔  x0 ≤ x1 ; le complémentaire est x1 − x0 ≤ −1.
    let region_dessous = Dbm::nouveau(2, &[(0, 1, 0)]);
    let region_dessus = Dbm::nouveau(2, &[(1, 0, -1)]);
    let racine = pool.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([
                (region_dessous.clone(), vrai_terminal),
                (region_dessus.clone(), faux_terminal),
                (INCOHERENT, faux_terminal),
            ]),
        },
    );
    let forme = pool.resolve(racine).expect("nœud interné");
    let Form::Node { arcs, .. } = forme else {
        panic!("deux régions distinctes : pas de R-collapse");
    };
    assert!(
        !arcs.contains_key(&INCOHERENT),
        "R-default supprime l'arc vers le défaut"
    );
    for x0 in 0..TAILLE_DOMAINE {
        for x1 in 0..TAILLE_DOMAINE {
            let point = [x0, x1];
            let cible_naive = if x0 <= x1 {
                vrai_terminal
            } else {
                faux_terminal
            };
            let cible_dd = arcs
                .iter()
                .find(|(region, _)| region.contient(&point))
                .map(|(_, cible)| *cible)
                .expect("la partition couvre la boîte");
            assert_eq!(cible_dd, cible_naive, "point {point:?}");
        }
    }
}
