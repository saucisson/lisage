//! Fold integral tests (backlog § 9, open question 9 of the
//! spec): over 1D region keys, the discrete fold (cardinality) becomes the
//! Lebesgue integral (measure). Differential: continuous measure ==
//! analytic length, and discrete cardinality == point count.
//! Narration (ADR-0100): the `Attribute`/`Synthesize` events emitted by
//! the integral are pinned by a collecting observer.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Pool};
use lisage_core::arena::Arena;
use lisage_core::integral::{compile_union, integral_constant, intersection, measure_interval};
use lisage_core::observer::{ChannelAttribute, NoOpObserver, Observer, TraceEvent};
use lisage_core::region::{EMPTY, Interval, RegionController, RegionKeys};

type PoolRegion = Arena<RegionController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAINE: u64 = 12;

fn pool() -> PoolRegion {
    Arena::new(
        RegionController::new(RegionKeys::new(vec![SIZE_DOMAINE])),
        Box::new(NoOpObserver),
    )
}

/// Collecting observer to pin the narration events.
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

/// Full region of the universe: [0, SIZE_DOMAINE).
fn region_full() -> Interval {
    Interval::new(0, SIZE_DOMAINE - 1)
}

#[test]
fn test_measure_interval_continue() {
    // Continuous length: [a, b] has measure b − a (the point has measure 0).
    assert_eq!(measure_interval(&Interval::new(2, 5)), 3);
    assert_eq!(measure_interval(&Interval::point(4)), 0);
    assert_eq!(measure_interval(&EMPTY), 0);
    assert_eq!(measure_interval(&Interval::new(0, 11)), 11);
    // The discrete cardinality counts the points: [2, 5] has 4.
    assert_eq!(Interval::new(2, 5).size(), 4);
}

#[test]
fn test_intersection_intervals() {
    let a = Interval::new(2, 7);
    let b = Interval::new(5, 10);
    assert_eq!(intersection(&a, &b), Interval::new(5, 7));
    // Disjoint → empty.
    let c = Interval::new(8, 10);
    assert_eq!(intersection(&a, &c), EMPTY);
    // Empty with anything → empty.
    assert_eq!(intersection(&EMPTY, &a), EMPTY);
    // Commutativity.
    assert_eq!(intersection(&a, &b), intersection(&b, &a));
}

/// Fundamental differential: the integral of density 1 over the union of
/// two disjoint intervals is the sum of their lengths, while
/// the cardinal fold (discrete) is the sum of their sizes. This is the
/// passage count → measure.
#[test]
fn test_integral_union_two_intervals() {
    let mut pool = pool();
    let intervals = vec![Interval::new(2, 5), Interval::new(7, 9)];
    let root = compile_union(&mut pool, ANCHOR, &intervals);
    let measure = integral_constant(&pool, root, &region_full(), None).expect("interned node");
    // Measure continue : (5−2) + (9−7) = 3 + 2 = 5.
    assert_eq!(measure, 5, "integral = sum of lengths");
    // Discrete cardinality: (5−2+1) + (9−7+1) = 4 + 3 = 7.
    let cardinal: u64 = intervals.iter().map(Interval::size).sum();
    assert_eq!(cardinal, 7, "cardinality = sum of the sizes (points)");
    assert_ne!(measure, cardinal, "count ≠ measure on the continuous");
}

/// The integral restricted to a sub-region only counts the part
/// of the denoted set that falls into this sub-region.
#[test]
fn test_integral_restricted_a_one_region() {
    let mut pool = pool();
    let intervals = vec![Interval::new(2, 8)];
    let root = compile_union(&mut pool, ANCHOR, &intervals);
    // Restriction to [5, 11]: measure of [2,8] ∩ [5,11] = [5,8], length 3.
    let under_region = Interval::new(5, 11);
    let measure = integral_constant(&pool, root, &under_region, None).expect("interned node");
    assert_eq!(measure, 3);
    // Restriction to a disjoint region: zero measure.
    let disjointe = Interval::new(9, 11);
    let measure_null = integral_constant(&pool, root, &disjointe, None).expect("interned node");
    assert_eq!(measure_null, 0);
}

/// The terminal ⊤ alone returns the measure of the whole current region;
/// the terminal ⊥ returns 0.
#[test]
fn test_integral_terminals() {
    let mut pool = pool();
    let true_node = pool.intern(ANCHOR, Form::Terminal(true));
    let false_node = pool.intern(ANCHOR, Form::Terminal(false));
    let full = region_full();
    assert_eq!(
        integral_constant(&pool, true_node, &full, None).expect("interned node"),
        SIZE_DOMAINE - 1,
        "⊤ returns the measure of the full region"
    );
    assert_eq!(
        integral_constant(&pool, false_node, &full, None).expect("interned node"),
        0,
        "⊥ rend 0"
    );
}

/// Narration (ADR-0100 decision 6): the integral emits one
/// `Attribute`/`Synthesize` event per visitd node, with the computed measure —
/// pinned by the collecting observer.
#[test]
fn test_narration_events_attribute_synthesize() {
    let mut pool = pool();
    let intervals = vec![Interval::new(2, 5), Interval::new(7, 9)];
    let root = compile_union(&mut pool, ANCHOR, &intervals);
    let collecteur = Collecteur::new();
    let measure =
        integral_constant(&pool, root, &region_full(), Some(&collecteur)).expect("interned node");
    assert_eq!(measure, 5);
    let events = collecteur.events();
    // At least one Attribute/Synthesize event emitted, with the final measure.
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
            .any(|e| matches!(e, TraceEvent::Attribute { value, .. } if value == "measure=5")),
        "the final event carries the total measure 5"
    );
}

/// Property (LCG): for a subset S of [0, T) drawn at random,
/// the integral of the union diagram of the intervals of S equals the sum
/// of the interval lengths, and the discrete cardinality equals |S|.
#[test]
fn test_property_integral_against_cardinal_lcg() {
    const GRAINE_LCG: u64 = 0xC0FF_EE00_1234_5678;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    for _ in 0..NUMBER_CAS {
        let mut next = || {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            graine
        };
        // Subset S of [0, SIZE_DOMAINE) drawn from the LCG.
        let set: BTreeSet<u64> = (0..SIZE_DOMAINE).filter(|_| next() % 2 == 1).collect();
        // Compression into disjoint intervals.
        let mut intervals = Vec::new();
        let mut point = 0;
        while point < SIZE_DOMAINE {
            if set.contains(&point) {
                let mut end = point;
                while end + 1 < SIZE_DOMAINE && set.contains(&(end + 1)) {
                    end += 1;
                }
                intervals.push(Interval::new(point, end));
                point = end + 1;
            } else {
                point += 1;
            }
        }
        let mut pool = pool();
        let root = compile_union(&mut pool, ANCHOR, &intervals);
        let measure = integral_constant(&pool, root, &region_full(), None).expect("interned node");
        // Analytic measure: sum of the (end − start) of the intervals.
        let measure_expected: u64 = intervals
            .iter()
            .map(|i| i.bound_sup().saturating_sub(i.bound_inf()))
            .sum();
        assert_eq!(measure, measure_expected, "integral == sum of lengths");
        // Cardinal analytic : |S|.
        let cardinal_expected = set.len() as u64;
        let cardinal: u64 = intervals.iter().map(Interval::size).sum();
        assert_eq!(cardinal, cardinal_expected, "cardinal == |S|");
        // In the continuous case, measure ≤ cardinality (each point has measure 0).
        assert!(measure <= cardinal);
    }
}
