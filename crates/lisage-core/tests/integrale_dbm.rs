//! Tests of the integral at the fold over **DBM** region-keys (backlog § 9,
//! open question 9 (ii) of the spec, "polyhedral regions"): the
//! Lebesgue measure of a 2D zone is the area of the band ∩ box
//! (rendered doubled, exact), and the integral of density 1 over the diagram
//! of the band `x − y ≤ 1 ∧ y − x ≤ 2` equals the analytic area of the band.
//! Differential: continuous area against counting of integer points.
//! Narration (ADR-0100): `Attribute`/`Synthesize` events pinned by a
//! collector observer.

use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys, INCONSISTENT};
use lisage_core::ddd::{compile_band, evaluer};
use lisage_core::integral::integral_dbm_constant;
use lisage_core::observer::{ChannelAttribute, NoOpObserver, Observer, TraceEvent};

type PoolDbm = Arena<DbmController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAINE: u64 = 4;

fn pool() -> PoolDbm {
    Arena::new(
        DbmController::new(DbmKeys::new(SIZE_DOMAINE)),
        Box::new(NoOpObserver),
    )
}

fn keys() -> DbmKeys {
    DbmKeys::new(SIZE_DOMAINE)
}

/// Full region of the box: no off-diagonal constraint.
fn box_dbm() -> Dbm {
    Dbm::new(2, &[])
}

/// Collector observer to pin the narration events.
#[derive(Clone)]
struct Collecteur {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collecteur {
    fn new() -> Self {
        Collecteur {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn events(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("mutex").clone()
    }
}

impl Observer for Collecteur {
    fn on_event(&self, event: &TraceEvent) {
        self.events.lock().expect("mutex").push(event.clone());
    }
}

/// Analytic area (doubled): a 2D DBM is a band −b ≤ x−y ≤ a, and
/// the area of the band ∩ [0, N]² is f(−b) − f(a), with f(t) = area of
/// {x−y > t}.
#[test]
fn test_measure_zone_analytic() {
    let keys = keys();
    // Full box: area N², doubled 2N² = 32.
    assert_eq!(keys.measure_zone(&box_dbm()), 32);
    // Empty zone → 0.
    assert_eq!(keys.measure_zone(&INCONSISTENT), 0);
    // Band −2 ≤ x−y ≤ 1 : 16 − (4−1)²/2 − (4−2)²/2 = 16 − 4.5 − 2 = 9.5 → 19.
    let band = Dbm::new(2, &[(0, 1, 1), (1, 0, 2)]);
    assert_eq!(keys.measure_zone(&band), 19);
    // Diagonal x = y : zero area.
    let diagonale = Dbm::new(2, &[(0, 1, 0), (1, 0, 0)]);
    assert_eq!(keys.measure_zone(&diagonale), 0);
    // Constraint outside the box (a ≥ N) : the full box.
    let large = Dbm::new(2, &[(0, 1, 5)]);
    assert_eq!(keys.measure_zone(&large), 32);
    // Half-band 0 ≤ x−y ≤ 1 : 16 − 8 − 4.5 = 3.5 → 7 (half-integer area).
    let demi = Dbm::new(2, &[(0, 1, 1), (1, 0, 0)]);
    assert_eq!(keys.measure_zone(&demi), 7);
}

/// The integral of density 1 over the diagram of the band equals the
/// analytic area of the band ∩ box: 9.5, rendered doubled 19.
#[test]
fn test_integral_band_area_analytic() {
    let mut pool = pool();
    let root = compile_band(&mut pool, ANCHOR);
    let measure =
        integral_dbm_constant(&pool, root, &box_dbm(), &keys(), None).expect("interned node");
    assert_eq!(measure, 19, "integral = doubled area of the band ∩ box");
    // A not-interned NodeId is a propagated error, not a silent value.
    assert!(integral_dbm_constant(&pool, NodeId(u32::MAX), &box_dbm(), &keys(), None).is_err());
}

/// The integral restricted to a sub-region only counts the part of the
/// band that falls into that sub-region.
#[test]
fn test_integral_restricted_a_one_region() {
    let mut pool = pool();
    let root = compile_band(&mut pool, ANCHOR);
    // Restriction to {x − y ≤ 0} : band ∩ {−2 ≤ x−y ≤ 0}, area 6 → 12.
    let under_region = Dbm::new(2, &[(0, 1, 0)]);
    let measure =
        integral_dbm_constant(&pool, root, &under_region, &keys(), None).expect("interned node");
    assert_eq!(measure, 12);
    // Restriction to a region disjoint from the band: zero measure.
    let disjointe = Dbm::new(2, &[(1, 0, -3)]); // x − y ≥ 3
    let measure_null =
        integral_dbm_constant(&pool, root, &disjointe, &keys(), None).expect("interned node");
    assert_eq!(measure_null, 0);
}

/// The terminal ⊤ alone returns the area of the whole current region (full
/// box: 2N² = 32); the terminal ⊥ returns 0.
#[test]
fn test_integral_terminals_dbm() {
    let mut pool = pool();
    let true_node = pool.intern(ANCHOR, Form::Terminal(true));
    let false_node = pool.intern(ANCHOR, Form::Terminal(false));
    assert_eq!(
        integral_dbm_constant(&pool, true_node, &box_dbm(), &keys(), None).expect("interned node"),
        32,
        "⊤ returns the doubled area of the full box"
    );
    assert_eq!(
        integral_dbm_constant(&pool, false_node, &box_dbm(), &keys(), None).expect("interned node"),
        0,
        "⊥ rend 0"
    );
}

/// Differential count → measure: the cardinal fold counts the integer
/// points of the band (12 points in [0,4)²), the integral measures the continuous
/// area (9.5) — measure ≤ cardinality, and the two are computed independently.
#[test]
fn test_differential_cardinal_against_measure() {
    let mut pool = pool();
    let root = compile_band(&mut pool, ANCHOR);
    let cardinal: u64 = (0..SIZE_DOMAINE)
        .flat_map(|x| (0..SIZE_DOMAINE).map(move |y| [x, y]))
        .filter(|point| {
            evaluer(&pool, root, point)
                .expect("interned node")
                .unwrap_or(false)
        })
        .count() as u64;
    assert_eq!(cardinal, 12, "integer points of the band in [0,4)²");
    let measure =
        integral_dbm_constant(&pool, root, &box_dbm(), &keys(), None).expect("interned node");
    assert_eq!(measure, 19);
    assert!(
        measure <= 2 * i128::from(cardinal),
        "doubled area ≤ 2 × cardinality (each point has zero measure)"
    );
}

/// Narration (ADR-0100 decision 6): the DBM integral emits an
/// `Attribute`/`Synthesize` event per visitd node, with the computed measure.
#[test]
fn test_narration_events_attribute_synthesize() {
    let mut pool = pool();
    let root = compile_band(&mut pool, ANCHOR);
    let collecteur = Collecteur::new();
    let measure = integral_dbm_constant(&pool, root, &box_dbm(), &keys(), Some(&collecteur))
        .expect("interned node");
    assert_eq!(measure, 19);
    let events = collecteur.events();
    let attrs: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::Attribute {
                    channel: ChannelAttribute::Synthesize,
                    ..
                }
            )
        })
        .collect();
    assert!(!attrs.is_empty(), "at least one Attribute/Synthesize event");
    assert!(
        attrs
            .iter()
            .any(|e| matches!(e, TraceEvent::Attribute { value, .. } if value == "measure=19")),
        "the final event carries the total measure 19"
    );
}

/// Property (LCG): the measure is monotone under intersection —
/// measure(A ∩ B) ≤ min(measure(A), measure(B)) — and the empty zone has
/// zero measure.
#[test]
fn test_property_measure_intersection_lcg() {
    const GRAINE_LCG: u64 = 0xDEAD_BEEF_CAFE_F00D;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let keys = keys();
    let mut graine = GRAINE_LCG;
    let mut next = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    let drawn = |next: &mut dyn FnMut() -> u64| {
        let number_constraints = next() % 4;
        let constraints: Vec<(usize, usize, i64)> = (0..number_constraints)
            .map(|_| {
                (
                    (next() % 2) as usize,
                    (next() % 2) as usize,
                    (next() % 9) as i64 - 4,
                )
            })
            .collect();
        Dbm::new(2, &constraints)
    };
    for _ in 0..NUMBER_CAS {
        let a = drawn(&mut next);
        let b = drawn(&mut next);
        let meet = a.intersection(&b);
        let measure_meet = keys.measure_zone(&meet);
        assert_eq!(keys.measure_zone(&INCONSISTENT), 0);
        assert!(
            measure_meet <= keys.measure_zone(&a) && measure_meet <= keys.measure_zone(&b),
            "monotonicity: measure(A ∩ B) ≤ min(measure(A), measure(B))"
        );
    }
}
