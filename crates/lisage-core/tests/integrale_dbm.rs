//! Tests for the integral at the fold on **DBM** region-keys (backlog § 9,
//! open question 9 (ii) of the spec, "polyhedral regions"): the Lebesgue
//! measure of a 2D zone is the area of the band ∩ box (returned doubled,
//! exact), and the integral of density 1 over the diagram of the band
//! `x − y ≤ 1 ∧ y − x ≤ 2` equals the analytic area of the band.
//! Differential: continuous area vs integer point count.
//! Narration (ADR-0100): `Attr`/`Synthesized` events pinned by a collecting
//! observer.

use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCOHERENT};
use lisage_core::ddd::{compiler_bande, evaluer};
use lisage_core::integrale::integrale_dbm_constante;
use lisage_core::observer::{CanalAttr, NoOpObserver, Observer, TraceEvent};

type PoolDbm = Arena<DbmController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAIN: u64 = 4;

fn pool() -> PoolDbm {
    Arena::new(
        DbmController::new(DbmKeys::new(SIZE_DOMAIN)),
        Box::new(NoOpObserver),
    )
}

fn keys() -> DbmKeys {
    DbmKeys::new(SIZE_DOMAIN)
}

/// Full region of the box: no off-diagonal constraint.
fn boite() -> Dbm {
    Dbm::new(2, &[])
}

/// Collecting observer to pin the narration events.
#[derive(Clone)]
struct Collecteur {
    evenements: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collecteur {
    fn new() -> Self {
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

/// Analytic area (doubled): a 2D DBM is a band −b ≤ x−y ≤ a, and the area
/// of the band ∩ [0, N]² is f(−b) − f(a), with f(t) = area of {x−y > t}.
#[test]
fn test_measure_zone_analytique() {
    let keys = keys();
    // Full box: area N², doubled 2N² = 32.
    assert_eq!(keys.measure_zone(&boite()), 32);
    // Empty zone → 0.
    assert_eq!(keys.measure_zone(&INCOHERENT), 0);
    // Band −2 ≤ x−y ≤ 1: 16 − (4−1)²/2 − (4−2)²/2 = 16 − 4.5 − 2 = 9.5 → 19.
    let bande = Dbm::new(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(keys.measure_zone(&bande), 19);
    // Diagonal x = y: zero area.
    let diagonale = Dbm::new(2, &[(0, 1, 0), (1, 0, 0)]);
    assert_eq!(keys.measure_zone(&diagonale), 0);
    // Constraint outside the box (a ≥ N): the full box.
    let large = Dbm::new(2, &[(0, 1, 5)]);
    assert_eq!(keys.measure_zone(&large), 32);
    // Half-band 0 ≤ x−y ≤ 1: 16 − 8 − 4.5 = 3.5 → 7 (half-integer area).
    let demi = Dbm::new(2, &[(0, 1, 1), (1, 0, 0)]);
    assert_eq!(keys.measure_zone(&demi), 7);
}

/// The integral of density 1 over the band diagram equals the analytic
/// area of the band ∩ box: 9.5, returned doubled 19.
#[test]
fn test_integrale_bande_aire_analytique() {
    let mut pool = pool();
    let root = compiler_bande(&mut pool, ANCHOR);
    let measure =
        integrale_dbm_constante(&pool, root, &boite(), &keys(), None).expect("interned node");
    assert_eq!(measure, 19, "integral = doubled area of the band ∩ box");
    // A non-interned NodeId is a propagated error, not a silent value.
    assert!(integrale_dbm_constante(&pool, NodeId(u32::MAX), &boite(), &keys(), None).is_err());
}

/// The integral restricted to a sub-region only counts the part of the
/// band that falls into that sub-region.
#[test]
fn test_integrale_restreinte_a_une_region() {
    let mut pool = pool();
    let root = compiler_bande(&mut pool, ANCHOR);
    // Restriction to {x − y ≤ 0}: band ∩ {−2 ≤ x−y ≤ 0}, area 6 → 12.
    let sous_region = Dbm::new(2, &[(0, 1, 0)]);
    let measure =
        integrale_dbm_constante(&pool, root, &sous_region, &keys(), None).expect("interned node");
    assert_eq!(measure, 12);
    // Restriction to a region disjoint from the band: zero measure.
    let disjointe = Dbm::new(2, &[(1, 0, -3)]); // x − y ≥ 3
    let measure_nulle =
        integrale_dbm_constante(&pool, root, &disjointe, &keys(), None).expect("interned node");
    assert_eq!(measure_nulle, 0);
}

/// The terminal ⊤ alone returns the area of the whole current region (full
/// box: 2N² = 32); the terminal ⊥ returns 0.
#[test]
fn test_integrale_terminaux_dbm() {
    let mut pool = pool();
    let vrai = pool.intern(ANCHOR, Form::Terminal(true));
    let faux = pool.intern(ANCHOR, Form::Terminal(false));
    assert_eq!(
        integrale_dbm_constante(&pool, vrai, &boite(), &keys(), None).expect("interned node"),
        32,
        "⊤ returns the doubled area of the full box"
    );
    assert_eq!(
        integrale_dbm_constante(&pool, faux, &boite(), &keys(), None).expect("interned node"),
        0,
        "⊥ returns 0"
    );
}

/// Differential count → measure: the cardinal fold counts the integer
/// points of the band (12 points in [0,4)²), the integral measures the
/// continuous area (9.5) — measure ≤ cardinal, and the two are computed
/// independently.
#[test]
fn test_differentiel_cardinal_contre_measure() {
    let mut pool = pool();
    let root = compiler_bande(&mut pool, ANCHOR);
    let cardinal: u64 = (0..SIZE_DOMAIN)
        .flat_map(|x| (0..SIZE_DOMAIN).map(move |y| [x, y]))
        .filter(|point| {
            evaluer(&pool, root, point)
                .expect("interned node")
                .unwrap_or(false)
        })
        .count() as u64;
    assert_eq!(cardinal, 12, "integer points of the band in [0,4)²");
    let measure =
        integrale_dbm_constante(&pool, root, &boite(), &keys(), None).expect("interned node");
    assert_eq!(measure, 19);
    assert!(
        measure <= 2 * i128::from(cardinal),
        "doubled area ≤ 2 × cardinal (each point has measure zero)"
    );
}

/// Narration (ADR-0100 decision 6): the DBM integral emits one
/// `Attr`/`Synthesized` event per visited node, with the computed measure.
#[test]
fn test_narration_evenements_attr_synthesized() {
    let mut pool = pool();
    let root = compiler_bande(&mut pool, ANCHOR);
    let collecteur = Collecteur::new();
    let measure = integrale_dbm_constante(&pool, root, &boite(), &keys(), Some(&collecteur))
        .expect("interned node");
    assert_eq!(measure, 19);
    let events = collecteur.events();
    let attrs: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::Attr {
                    canal: CanalAttr::Synthesized,
                    ..
                }
            )
        })
        .collect();
    assert!(!attrs.is_empty(), "at least one Attr/Synthesized event");
    assert!(
        attrs
            .iter()
            .any(|e| matches!(e, TraceEvent::Attr { value, .. } if value == "measure=19")),
        "the final event carries the total measure 19"
    );
}

/// Property (LCG): the measure is monotone under intersection —
/// measure(A ∩ B) ≤ min(measure(A), measure(B)) — and the empty zone has
/// measure zero.
#[test]
fn test_propriete_measure_intersection_lcg() {
    const SEED_LCG: u64 = 0xDEAD_BEEF_CAFE_F00D;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let keys = keys();
    let mut seed = SEED_LCG;
    let mut next = || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed
    };
    let tire = |next: &mut dyn FnMut() -> u64| {
        let nombre_contraintes = next() % 4;
        let contraintes: Vec<(usize, usize, i64)> = (0..nombre_contraintes)
            .map(|_| {
                (
                    (next() % 2) as usize,
                    (next() % 2) as usize,
                    (next() % 9) as i64 - 4,
                )
            })
            .collect();
        Dbm::new(2, &contraintes)
    };
    for _ in 0..NOMBRE_CAS {
        let a = tire(&mut next);
        let b = tire(&mut next);
        let meet = a.intersection(&b);
        let measure_meet = keys.measure_zone(&meet);
        assert_eq!(keys.measure_zone(&INCOHERENT), 0);
        assert!(
            measure_meet <= keys.measure_zone(&a) && measure_meet <= keys.measure_zone(&b),
            "monotonicity: measure(A ∩ B) ≤ min(measure(A), measure(B))"
        );
    }
}
