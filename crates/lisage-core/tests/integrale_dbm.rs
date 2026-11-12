//! Tests de l'intégrale au pli sur clés-régions **DBM** (backlog § 9,
//! question ouverte 9 (ii) de la spec, « régions polyédriques ») : la
//! mesure de Lebesgue d'une zone 2D est l'aire de la bande ∩ boîte
//! (rendue doublée, exacte), et l'intégrale de densité 1 sur le diagramme
//! de la bande `x − y ≤ 1 ∧ y − x ≤ 2` vaut l'aire analytique de la bande.
//! Différentiel : aire continue contre comptage des points entiers.
//! Narration (ADR-0100) : événements `Attr`/`Synthétise` épinglés par un
//! observateur collecteur.

use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCOHERENT};
use lisage_core::ddd::{compiler_bande, evaluer};
use lisage_core::integrale::integrale_dbm_constante;
use lisage_core::observer::{CanalAttr, NoOpObserver, Observer, TraceEvent};

type PoolDbm = Arena<DbmController>;

const ANCRE: Anchor = Anchor(0);
const TAILLE_DOMAINE: u64 = 4;

fn pool() -> PoolDbm {
    Arena::new(
        DbmController::nouveau(DbmKeys::nouvelle(TAILLE_DOMAINE)),
        Box::new(NoOpObserver),
    )
}

fn keys() -> DbmKeys {
    DbmKeys::nouvelle(TAILLE_DOMAINE)
}

/// Région pleine de la boîte : aucune contrainte hors diagonale.
fn boite() -> Dbm {
    Dbm::nouveau(2, &[])
}

/// Observateur collecteur pour épingler les événements de narration.
#[derive(Clone)]
struct Collecteur {
    evenements: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collecteur {
    fn nouveau() -> Self {
        Collecteur {
            evenements: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn events(&self) -> Vec<TraceEvent> {
        self.evenements.lock().expect("mutex").clone()
    }
}

impl Observer for Collecteur {
    fn on_event(&self, event: &TraceEvent) {
        self.evenements.lock().expect("mutex").push(event.clone());
    }
}

/// Aire analytique (doublée) : une DBM 2D est une bande −b ≤ x−y ≤ a, et
/// l'aire de la bande ∩ [0, N]² vaut f(−b) − f(a), avec f(t) = aire de
/// {x−y > t}.
#[test]
fn test_mesure_zone_analytique() {
    let keys = keys();
    // Boîte pleine : aire N², doublée 2N² = 32.
    assert_eq!(keys.mesure_zone(&boite()), 32);
    // Zone vide → 0.
    assert_eq!(keys.mesure_zone(&INCOHERENT), 0);
    // Bande −2 ≤ x−y ≤ 1 : 16 − (4−1)²/2 − (4−2)²/2 = 16 − 4.5 − 2 = 9.5 → 19.
    let bande = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(keys.mesure_zone(&bande), 19);
    // Diagonale x = y : aire nulle.
    let diagonale = Dbm::nouveau(2, &[(0, 1, 0), (1, 0, 0)]);
    assert_eq!(keys.mesure_zone(&diagonale), 0);
    // Contrainte hors de la boîte (a ≥ N) : la boîte pleine.
    let large = Dbm::nouveau(2, &[(0, 1, 5)]);
    assert_eq!(keys.mesure_zone(&large), 32);
    // Demi-bande 0 ≤ x−y ≤ 1 : 16 − 8 − 4.5 = 3.5 → 7 (aire demi-entière).
    let demi = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 0)]);
    assert_eq!(keys.mesure_zone(&demi), 7);
}

/// L'intégrale de densité 1 sur le diagramme de la bande vaut l'aire
/// analytique de la bande ∩ boîte : 9.5, rendue doublée 19.
#[test]
fn test_integrale_bande_aire_analytique() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    let mesure =
        integrale_dbm_constante(&pool, racine, &boite(), &keys(), None).expect("nœud interné");
    assert_eq!(mesure, 19, "intégrale = aire doublée de la bande ∩ boîte");
    // Un NodeId non interné est une erreur propagée, pas une valeur muette.
    assert!(integrale_dbm_constante(&pool, NodeId(u32::MAX), &boite(), &keys(), None).is_err());
}

/// L'intégrale restreinte à une sous-région ne compte que la partie de la
/// bande qui tombe dans cette sous-région.
#[test]
fn test_integrale_restreinte_a_une_region() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    // Restriction à {x − y ≤ 0} : bande ∩ {−2 ≤ x−y ≤ 0}, aire 6 → 12.
    let sous_region = Dbm::nouveau(2, &[(0, 1, 0)]);
    let mesure =
        integrale_dbm_constante(&pool, racine, &sous_region, &keys(), None).expect("nœud interné");
    assert_eq!(mesure, 12);
    // Restriction à une région disjointe de la bande : mesure nulle.
    let disjointe = Dbm::nouveau(2, &[(1, 0, -3)]); // x − y ≥ 3
    let mesure_nulle =
        integrale_dbm_constante(&pool, racine, &disjointe, &keys(), None).expect("nœud interné");
    assert_eq!(mesure_nulle, 0);
}

/// Le terminal ⊤ seul rend l'aire de toute la région courante (boîte
/// pleine : 2N² = 32) ; le terminal ⊥ rend 0.
#[test]
fn test_integrale_terminaux_dbm() {
    let mut pool = pool();
    let vrai = pool.intern(ANCRE, Form::Terminal(true));
    let faux = pool.intern(ANCRE, Form::Terminal(false));
    assert_eq!(
        integrale_dbm_constante(&pool, vrai, &boite(), &keys(), None).expect("nœud interné"),
        32,
        "⊤ rend l'aire doublée de la boîte pleine"
    );
    assert_eq!(
        integrale_dbm_constante(&pool, faux, &boite(), &keys(), None).expect("nœud interné"),
        0,
        "⊥ rend 0"
    );
}

/// Différentiel count → mesure : le pli cardinal compte les points entiers
/// de la bande (12 points dans [0,4)²), l'intégrale mesure l'aire continue
/// (9.5) — mesure ≤ cardinal, et les deux sont calculés indépendamment.
#[test]
fn test_differentiel_cardinal_contre_mesure() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    let cardinal: u64 = (0..TAILLE_DOMAINE)
        .flat_map(|x| (0..TAILLE_DOMAINE).map(move |y| [x, y]))
        .filter(|point| {
            evaluer(&pool, racine, point)
                .expect("nœud interné")
                .unwrap_or(false)
        })
        .count() as u64;
    assert_eq!(cardinal, 12, "points entiers de la bande dans [0,4)²");
    let mesure =
        integrale_dbm_constante(&pool, racine, &boite(), &keys(), None).expect("nœud interné");
    assert_eq!(mesure, 19);
    assert!(
        mesure <= 2 * i128::from(cardinal),
        "aire doublée ≤ 2 × cardinal (chaque point a mesure nulle)"
    );
}

/// Narration (ADR-0100 décision 6) : l'intégrale DBM émet un événement
/// `Attr`/`Synthétise` par nœud visité, avec la mesure calculée.
#[test]
fn test_narration_evenements_attr_synthétise() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    let collecteur = Collecteur::nouveau();
    let mesure = integrale_dbm_constante(&pool, racine, &boite(), &keys(), Some(&collecteur))
        .expect("nœud interné");
    assert_eq!(mesure, 19);
    let events = collecteur.events();
    let attrs: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::Attr {
                    canal: CanalAttr::Synthétise,
                    ..
                }
            )
        })
        .collect();
    assert!(!attrs.is_empty(), "au moins un événement Attr/Synthétise");
    assert!(
        attrs
            .iter()
            .any(|e| matches!(e, TraceEvent::Attr { valeur, .. } if valeur == "mesure=19")),
        "l'événement final porte la mesure totale 19"
    );
}

/// Propriété (LCG) : la mesure est monotone pour l'intersection —
/// mesure(A ∩ B) ≤ min(mesure(A), mesure(B)) — et la zone vide a mesure
/// nulle.
#[test]
fn test_propriete_mesure_intersection_lcg() {
    const GRAINE_LCG: u64 = 0xDEAD_BEEF_CAFE_F00D;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let keys = keys();
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
                    (suivant() % 9) as i64 - 4,
                )
            })
            .collect();
        Dbm::nouveau(2, &contraintes)
    };
    for _ in 0..NOMBRE_CAS {
        let a = tire(&mut suivant);
        let b = tire(&mut suivant);
        let meet = a.intersection(&b);
        let mesure_meet = keys.mesure_zone(&meet);
        assert_eq!(keys.mesure_zone(&INCOHERENT), 0);
        assert!(
            mesure_meet <= keys.mesure_zone(&a) && mesure_meet <= keys.mesure_zone(&b),
            "monotonie : mesure(A ∩ B) ≤ min(mesure(A), mesure(B))"
        );
    }
}
