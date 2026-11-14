//! Tests for the fold integral over **DBM** region-keys (backlog § 9, open
//! question 9 (ii) of the spec, "polyhedral regions"): the Lebesgue measure
//! of a 2D zone is the area of the band ∩ box (returned doubled, exact), and
//! the integral of density 1 over the diagram of the band
//! `x − y ≤ 1 ∧ y − x ≤ 2` equals the analytic area of the band.
//! Differential: continuous area vs integer point count.
//! Narration (ADR-0100): `Attr`/`Synthétise` events pinned by a collecting
//! observer.

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

/// Full region of the box: no constraint off the diagonal.
fn boite() -> Dbm {
    Dbm::nouveau(2, &[])
}

/// Collecting observer to pin the narration events.
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

/// Analytic area (doubled): a 2D DBM is a band −b ≤ x−y ≤ a, and the area of
/// the band ∩ [0, N]² equals f(−b) − f(a), with f(t) = area of {x−y > t}.
#[test]
fn test_mesure_zone_analytique() {
    let keys = keys();
    // Full box: area N², doubled 2N² = 32.
    assert_eq!(keys.mesure_zone(&boite()), 32);
    // Empty zone → 0.
    assert_eq!(keys.mesure_zone(&INCOHERENT), 0);
    // Band −2 ≤ x−y ≤ 1: 16 − (4−1)²/2 − (4−2)²/2 = 16 − 4.5 − 2 = 9.5 → 19.
    let bande = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(keys.mesure_zone(&bande), 19);
    // Diagonal x = y: zero area.
    let diagonale = Dbm::nouveau(2, &[(0, 1, 0), (1, 0, 0)]);
    assert_eq!(keys.mesure_zone(&diagonale), 0);
    // Constraint outside the box (a ≥ N): the full box.
    let large = Dbm::nouveau(2, &[(0, 1, 5)]);
    assert_eq!(keys.mesure_zone(&large), 32);
    // Half-band 0 ≤ x−y ≤ 1: 16 − 8 − 4.5 = 3.5 → 7 (half-integer area).
    let demi = Dbm::nouveau(2, &[(0, 1, 1), (1, 0, 0)]);
    assert_eq!(keys.mesure_zone(&demi), 7);
}

/// The integral of density 1 over the band's diagram equals the analytic area
/// of the band ∩ box: 9.5, returned doubled as 19.
#[test]
fn test_integrale_bande_aire_analytique() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    let mesure =
        integrale_dbm_constante(&pool, racine, &boite(), &keys(), None).expect("interned node");
    assert_eq!(mesure, 19, "integral = doubled area of the band ∩ box");
    // A non-interned NodeId is a propagated error, not a silent value.
    assert!(integrale_dbm_constante(&pool, NodeId(u32::MAX), &boite(), &keys(), None).is_err());
}

/// The integral restricted to a sub-region only counts the part of the band
/// that falls into that sub-region.
#[test]
fn test_integrale_restreinte_a_une_region() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    // Restriction to {x − y ≤ 0}: band ∩ {−2 ≤ x−y ≤ 0}, area 6 → 12.
    let sous_region = Dbm::nouveau(2, &[(0, 1, 0)]);
    let mesure =
        integrale_dbm_constante(&pool, racine, &sous_region, &keys(), None).expect("interned node");
    assert_eq!(mesure, 12);
    // Restriction to a region disjoint from the band: zero measure.
    let disjointe = Dbm::nouveau(2, &[(1, 0, -3)]); // x − y ≥ 3
    let mesure_nulle =
        integrale_dbm_constante(&pool, racine, &disjointe, &keys(), None).expect("interned node");
    assert_eq!(mesure_nulle, 0);
}

/// The terminal ⊤ alone yields the area of the whole current region (full
/// box: 2N² = 32); the terminal ⊥ yields 0.
#[test]
fn test_integrale_terminaux_dbm() {
    let mut pool = pool();
    let vrai = pool.intern(ANCRE, Form::Terminal(true));
    let faux = pool.intern(ANCRE, Form::Terminal(false));
    assert_eq!(
        integrale_dbm_constante(&pool, vrai, &boite(), &keys(), None).expect("interned node"),
        32,
        "⊤ yields the doubled area of the full box"
    );
    assert_eq!(
        integrale_dbm_constante(&pool, faux, &boite(), &keys(), None).expect("interned node"),
        0,
        "⊥ yields 0"
    );
}

/// Differential count → measure: the cardinality fold counts the integer
/// points of the band (12 points in [0,4)²), the integral measures the
/// continuous area (9.5) — measure ≤ cardinality, and the two are computed
/// independently.
#[test]
fn test_differentiel_cardinal_contre_mesure() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    let cardinal: u64 = (0..TAILLE_DOMAINE)
        .flat_map(|x| (0..TAILLE_DOMAINE).map(move |y| [x, y]))
        .filter(|point| {
            evaluer(&pool, racine, point)
                .expect("interned node")
                .unwrap_or(false)
        })
        .count() as u64;
    assert_eq!(cardinal, 12, "integer points of the band in [0,4)²");
    let mesure =
        integrale_dbm_constante(&pool, racine, &boite(), &keys(), None).expect("interned node");
    assert_eq!(mesure, 19);
    assert!(
        mesure <= 2 * i128::from(cardinal),
        "doubled area ≤ 2 × cardinality (each point has zero measure)"
    );
}

/// Narration (ADR-0100 decision 6): the DBM integral emits an
/// `Attr`/`Synthétise` event per visited node, with the computed measure.
#[test]
fn test_narration_evenements_attr_synthétise() {
    let mut pool = pool();
    let racine = compiler_bande(&mut pool, ANCRE);
    let collecteur = Collecteur::nouveau();
    let mesure = integrale_dbm_constante(&pool, racine, &boite(), &keys(), Some(&collecteur))
        .expect("interned node");
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
    assert!(!attrs.is_empty(), "at least one Attr/Synthétise event");
    assert!(
        attrs
            .iter()
            .any(|e| matches!(e, TraceEvent::Attr { valeur, .. } if valeur == "mesure=19")),
        "the final event carries the total measure 19"
    );
}

/// Property (LCG): the measure is monotone under intersection —
/// measure(A ∩ B) ≤ min(measure(A), measure(B)) — and the empty zone has zero
/// measure.
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
            "monotonicity: measure(A ∩ B) ≤ min(measure(A), measure(B))"
        );
    }
}
