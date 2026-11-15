//! Tests for the integral at the fold (backlog § 9, open question 9 of the
//! spec): on 1D region-keys, the discrete fold (cardinality) becomes the
//! Lebesgue integral (measure). Differential: continuous measure ==
//! analytic length, and discrete cardinality == point count.
//! Narration (ADR-0100): the `Attr`/`Synthesized` events emitted by the
//! integral are pinned by a collecting observer.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Pool};
use lisage_core::arena::Arena;
use lisage_core::integrale::{compiler_union, integrale_constante, intersection, measure_interval};
use lisage_core::observer::{ChannelAttr, NoOpObserver, Observer, TraceEvent};
use lisage_core::region::{EMPTY, Interval, RegionController, RegionKeys};

type PoolRegion = Arena<RegionController>;

const ANCHOR: Anchor = Anchor(0);
const SIZE_DOMAIN: u64 = 12;

fn pool() -> PoolRegion {
    Arena::new(
        RegionController::new(RegionKeys::new(vec![SIZE_DOMAIN])),
        Box::new(NoOpObserver),
    )
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

/// Full region of the universe: [0, SIZE_DOMAIN).
fn region_pleine() -> Interval {
    Interval::new(0, SIZE_DOMAIN - 1)
}

#[test]
fn test_measure_interval_continue() {
    // Continuous length: [a, b] has measure b − a (the point has measure 0).
    assert_eq!(measure_interval(&Interval::new(2, 5)), 3);
    assert_eq!(measure_interval(&Interval::point(4)), 0);
    assert_eq!(measure_interval(&EMPTY), 0);
    assert_eq!(measure_interval(&Interval::new(0, 11)), 11);
    // The discrete cardinality counts points: [2, 5] has 4.
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
/// two disjoint intervals is the sum of their lengths, while the cardinality
/// fold (discrete) is the sum of their sizes. This is the count → measure
/// passage.
#[test]
fn test_integrale_union_deux_intervals() {
    let mut pool = pool();
    let intervals = vec![Interval::new(2, 5), Interval::new(7, 9)];
    let root = compiler_union(&mut pool, ANCHOR, &intervals);
    let measure = integrale_constante(&pool, root, &region_pleine(), None).expect("interned node");
    // Continuous measure: (5−2) + (9−7) = 3 + 2 = 5.
    assert_eq!(measure, 5, "integral = sum of lengths");
    // Discrete cardinality: (5−2+1) + (9−7+1) = 4 + 3 = 7.
    let cardinality: u64 = intervals.iter().map(Interval::size).sum();
    assert_eq!(cardinality, 7, "cardinality = sum of sizes (points)");
    assert_ne!(measure, cardinality, "count ≠ measure on the continuum");
}

/// The integral restricted to a sub-region only counts the part of the
/// denoted set that falls into that sub-region.
#[test]
fn test_integrale_restreinte_a_une_region() {
    let mut pool = pool();
    let intervals = vec![Interval::new(2, 8)];
    let root = compiler_union(&mut pool, ANCHOR, &intervals);
    // Restriction to [5, 11]: measure of [2,8] ∩ [5,11] = [5,8], length 3.
    let sous_region = Interval::new(5, 11);
    let measure = integrale_constante(&pool, root, &sous_region, None).expect("interned node");
    assert_eq!(measure, 3);
    // Restriction to a disjoint region: zero measure.
    let disjointe = Interval::new(9, 11);
    let measure_nulle = integrale_constante(&pool, root, &disjointe, None).expect("interned node");
    assert_eq!(measure_nulle, 0);
}

/// The terminal ⊤ alone returns the measure of the whole current region;
/// the terminal ⊥ returns 0.
#[test]
fn test_integrale_terminaux() {
    let mut pool = pool();
    let vrai = pool.intern(ANCHOR, Form::Terminal(true));
    let faux = pool.intern(ANCHOR, Form::Terminal(false));
    let pleine = region_pleine();
    assert_eq!(
        integrale_constante(&pool, vrai, &pleine, None).expect("interned node"),
        SIZE_DOMAIN - 1,
        "⊤ returns the measure of the full region"
    );
    assert_eq!(
        integrale_constante(&pool, faux, &pleine, None).expect("interned node"),
        0,
        "⊥ returns 0"
    );
}

/// Narration (ADR-0100 decision 6): the integral emits one
/// `Attr`/`Synthesized` event per visited node, with the computed measure —
/// pinned by the collecting observer.
#[test]
fn test_narration_evenements_attr_synthesized() {
    let mut pool = pool();
    let intervals = vec![Interval::new(2, 5), Interval::new(7, 9)];
    let root = compiler_union(&mut pool, ANCHOR, &intervals);
    let collecteur = Collecteur::new();
    let measure = integrale_constante(&pool, root, &region_pleine(), Some(&collecteur))
        .expect("interned node");
    assert_eq!(measure, 5);
    let events = collecteur.events();
    // At least one Attr/Synthesized event emitted, with the final measure.
    let attrs: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    ..
                }
            )
        })
        .collect();
    assert!(!attrs.is_empty(), "at least one Attr/Synthesized event");
    assert!(
        attrs
            .iter()
            .any(|e| matches!(e, TraceEvent::Attr { value, .. } if value == "measure=5")),
        "the final event carries the total measure 5"
    );
}

/// Property (LCG): for a random subset S of [0, T), the integral of the
/// union diagram of S's intervals equals the sum of the interval lengths,
/// and the discrete cardinality equals |S|.
#[test]
fn test_propriete_integrale_contre_cardinality_lcg() {
    const SEED_LCG: u64 = 0xC0FF_EE00_1234_5678;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut seed = SEED_LCG;
    for _ in 0..NUMBER_CAS {
        let mut next = || {
            seed = seed
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            seed
        };
        // Subset S of [0, SIZE_DOMAIN) drawn with the LCG.
        let set: BTreeSet<u64> = (0..SIZE_DOMAIN).filter(|_| next() % 2 == 1).collect();
        // Compression into disjoint intervals.
        let mut intervals = Vec::new();
        let mut point = 0;
        while point < SIZE_DOMAIN {
            if set.contains(&point) {
                let mut fin = point;
                while fin + 1 < SIZE_DOMAIN && set.contains(&(fin + 1)) {
                    fin += 1;
                }
                intervals.push(Interval::new(point, fin));
                point = fin + 1;
            } else {
                point += 1;
            }
        }
        let mut pool = pool();
        let root = compiler_union(&mut pool, ANCHOR, &intervals);
        let measure =
            integrale_constante(&pool, root, &region_pleine(), None).expect("interned node");
        // Analytic measure: sum of (end − start) of the intervals.
        let measure_expectede: u64 = intervals
            .iter()
            .map(|i| i.bound_sup().saturating_sub(i.bound_inf()))
            .sum();
        assert_eq!(measure, measure_expectede, "integral == sum of lengths");
        // Analytic cardinality: |S|.
        let cardinality_expected = set.len() as u64;
        let cardinality: u64 = intervals.iter().map(Interval::size).sum();
        assert_eq!(cardinality, cardinality_expected, "cardinality == |S|");
        // On the continuum, measure ≤ cardinality (each point has measure 0).
        assert!(measure <= cardinality);
    }
}
