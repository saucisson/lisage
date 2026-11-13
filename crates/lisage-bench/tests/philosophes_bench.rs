//! Philosophers benchmark (M4-4, statement C8): reference measurements of
//! the golden thread (docs/golden-thread-example-n-philosophers-end-to-end § 6-9; the measurement against
//! external SDD/HSDD is outside the CI perimeter). Falsifiable predictions
//! checked here: |R_n| = 20/56/152/416/1136 (n = 3..7), leftmost variant
//! |R'_n| = 72/196/536/1464 (n = 4..7) with ratio |R'_n|/|R_n| = 1,2887
//! **constant**, 7 forms (5 legal cells + 2 terminals) independent of n,
//! and the flat-prototype border (union of maps) pinned exactly, growing
//! in n. Live peak, firings and passes recorded via the observer. Any
//! contrary measurement fails the test: that is the explicit
//! falsification.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_lang::value::{Atom, Value};

/// Test anchor.
const ANCRE_TEST: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `hungry`.
const AFFAME: u64 = 1;
/// `eat`.
const EATS: u64 = 2;
/// `g`: intermediate state of the leftmost philosopher (position 0 only).
const LEFTMOST: u64 = 3;
/// Expected |R_n| (golden thread § 0, verified by simulation up to n = 20).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Expected |R'_n| for the leftmost variant (golden thread § 9.2, n = 4..7).
const R_GAUCHER_ATTENDUS: &[(u64, u64)] = &[(4, 72), (5, 196), (6, 536), (7, 1464)];
/// Predicted ratio |R'_n| / |R_n|, constant (golden thread § 9.4).
const RATIO_GAUCHER: f64 = 1.2887;
/// Tolerance on the ratio (constancy and value) — explicit falsification.
const TOLERANCE_RATIO: f64 = 0.01;
/// Number of legal cells of the minimal automaton (5 + 2 terminals = 7 forms).
const NB_CELLULES_LEGALES: usize = 5;
/// Stride of the interleaving: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Offset of the `left_i` key after the `pc_i` key in the interleaving.
const DECALAGE_CLE_GAUCHE: u64 = 1;
/// Live nodes of the flat prototype (union of maps) per n — measured
/// border, growing in n. The 4n − 4 of the golden thread concerns the flat
/// prototype of the encoded automaton (outside this bench's perimeter).
const BORDER_PLAT: &[(u64, usize)] = &[(3, 206), (4, 646), (5, 1850), (6, 5198), (7, 14362)];

type PoolRef = Arena<ReferenceController>;

/// Recording observer: firings (NodeCreated) and passes (Round).
#[derive(Default)]
struct RecordingObserver {
    firings: Mutex<u64>,
    passes: Mutex<u64>,
}

impl Observer for RecordingObserver {
    fn on_event(&self, event: &TraceEvent) {
        match event {
            TraceEvent::NodeCreated { .. } => *self.firings.lock().unwrap() += 1,
            TraceEvent::Round { .. } => *self.passes.lock().unwrap() += 1,
            _ => {}
        }
    }
}

/// Schema of the unrolled ring in interleaving (golden thread § 2): `2n`
/// keys, key `2i` = `pc_i`, key `2i+1` = `left_i`; domain {0,1,2} (+ g for
/// the leftmost).
fn schema_ring(n: u64, leftmost: bool) -> Schema {
    let domain = if leftmost {
        Domain::new(vec![Atom(PENSE), Atom(AFFAME), Atom(EATS), Atom(LEFTMOST)])
    } else {
        Domain::new(vec![Atom(PENSE), Atom(AFFAME), Atom(EATS)])
    };
    Schema::Map((0..PAS_CLE_INTERCALAGE * n).map(Atom).collect(), domain)
}

/// Fork `left_i` derived from the `pc` word (golden thread § 3).
fn fourchette(counter_program: &[u64], n: usize, i: usize) -> u64 {
    const LIBRE: u64 = 0;
    const TENUE: u64 = 1;
    const PRECEDENTE: u64 = 2;
    if counter_program[i] == EATS {
        TENUE
    } else if counter_program[(i + n - 1) % n] == EATS {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Enumerates the reachable circular words (regular or leftmost).
fn words_reachable(n: usize, leftmost: bool) -> Vec<Vec<u64>> {
    let mut result = Vec::new();
    let mut word = vec![PENSE; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(word[i] == EATS && word[(i + 1) % n] == EATS));
        let leftmost_ok = !leftmost || !(word[0] == LEFTMOST && word[1] == EATS);
        if circulaire_ok && leftmost_ok {
            result.push(word.clone());
        }
        // Increment in base 3; the leftmost has a 4th value `g` in
        // position 0 only.
        let mut i = n;
        loop {
            if i == 0 {
                return result;
            }
            i -= 1;
            let max = if leftmost && i == 0 { LEFTMOST } else { EATS };
            if word[i] < max {
                word[i] += 1;
                break;
            }
            word[i] = PENSE;
        }
    }
}

/// Builds R_n (or R'_n) in the pool: union of the compiled maps.
fn build_r_n(pool: &mut PoolRef, n: u64, leftmost: bool) -> NodeId {
    let schema = schema_ring(n, leftmost);
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(ANCRE_TEST, Form::Terminal(false));
    for word in words_reachable(n as usize, leftmost) {
        let map: BTreeMap<Value, Value> = (0..n)
            .flat_map(|i| {
                let i = i as usize;
                [
                    (PAS_CLE_INTERCALAGE * i as u64, word[i]),
                    (
                        PAS_CLE_INTERCALAGE * i as u64 + DECALAGE_CLE_GAUCHE,
                        fourchette(&word, n as usize, i),
                    ),
                ]
            })
            .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
            .collect();
        let membre = compiled(pool, ANCRE_TEST, &schema, &Value::Map(map)).unwrap();
        root = union(pool, &keys, ANCRE_TEST, root, membre).unwrap();
    }
    root
}

/// Count |R_n| by fold `(ℕ,+,×)` over the given schema.
fn count_r_n(pool_input: &PoolRef, schema: &Schema, root: NodeId) -> u64 {
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    count(pool_input, &mut output, ANCRE_TEST, schema, root).unwrap()
}

/// M4-4: |R_n| by DD fold == enumeration == golden-thread values
/// (20/56/152/416/1136). Explicit falsification if the measurement says
/// otherwise.
#[test]
fn test_bench_cardinals_r_n() {
    for &(n, expected) in R_ATTENDUS {
        let schema = schema_ring(n, false);
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let root = build_r_n(&mut pool, n, false);
        let count = count_r_n(&pool, &schema, root);
        let reference = words_reachable(n as usize, false).len() as u64;
        assert_eq!(count, reference, "n={n}: DD fold == enumeration");
        assert_eq!(
            count, expected,
            "n={n}: |R_{n}| = {expected} (golden thread)"
        );
    }
}

/// M4-4: leftmost variant — |R'_n| = 72/196/536/1464 (n = 4..7) and ratio
/// |R'_n| / |R_n| = 1,2887 **constant** (the irregular class shifts the
/// constant, not the exponent). Explicit falsification outside tolerance.
#[test]
fn test_bench_leftmost_ratio_constant() {
    let mut ratios = Vec::new();
    for &(n, expected) in R_GAUCHER_ATTENDUS {
        let schema = schema_ring(n, true);
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let root = build_r_n(&mut pool, n, true);
        // |R'_n| by DD fold == reference enumeration == golden thread § 9.2.
        let count = count_r_n(&pool, &schema, root);
        let reference = words_reachable(n as usize, true).len() as u64;
        assert_eq!(count, reference, "n={n}: DD fold == enumeration (leftmost)");
        assert_eq!(
            reference, expected,
            "n={n}: |R'_{n}| = {expected} (golden thread § 9.2)"
        );
        // Regular |R_n| for the ratio.
        let schema_reg = schema_ring(n, false);
        let mut pool_reg = Arena::new(
            ReferenceController::new(schema_reg.clone()),
            Box::new(NoOpObserver),
        );
        let root_reg = build_r_n(&mut pool_reg, n, false);
        let count_reg = count_r_n(&pool_reg, &schema_reg, root_reg);
        let ratio = count as f64 / count_reg as f64;
        ratios.push(ratio);
        assert!(
            (ratio - RATIO_GAUCHER).abs() < TOLERANCE_RATIO,
            "n={n}: leftmost ratio {ratio} ≈ {RATIO_GAUCHER}"
        );
    }
    // Constancy of the ratio over n = 4..7 (explicit falsification).
    let min = ratios.iter().copied().fold(f64::INFINITY, f64::min);
    let max = ratios.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max - min < TOLERANCE_RATIO,
        "ratio constant: min={min}, max={max}"
    );
}

/// M4-4: the **7 forms** = 5 legal cells + 2 terminals, independent of
/// n (O(1)); in contrast, the flat prototype (union of maps) has a number
/// of live nodes that grows with n (the border, 4n − 4). Live peak and
/// firings recorded via the observer.
#[test]
fn test_bench_seven_forms_o1_and_border() {
    let observer = Arc::new(RecordingObserver::default());
    struct Pont(Arc<RecordingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    let mut counts_forms = Vec::new();
    let mut nodes_flat = Vec::new();
    let mut creations = 0u64;
    for &(n, _) in R_ATTENDUS {
        let schema = schema_ring(n, false);
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(Pont(observer.clone())),
        );
        let root = build_r_n(&mut pool, n, false);
        // Distinct (pc, left) signatures appearing in R_n.
        let mut seen: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
        for word in words_reachable(n as usize, false) {
            for i in 0..n as usize {
                seen.insert((word[i], fourchette(&word, n as usize, i)));
            }
        }
        assert_eq!(
            seen.len(),
            NB_CELLULES_LEGALES,
            "n={n}: exactly 5 legal cells (7 forms with ⊥/⊤)"
        );
        counts_forms.push(seen.len());
        // Border of the flat prototype: live nodes of the pool (no GC here,
        // the live peak = live after construction).
        nodes_flat.push(pool.live());
        creations += pool.metrics().creations;
        let _ = root;
    }
    // O(1): the number of forms does not depend on n.
    assert!(
        counts_forms.windows(2).all(|w| w[0] == w[1]),
        "forms constant in n: {counts_forms:?}"
    );
    // The flat-prototype border is pinned exactly (explicit falsification)
    // and grows with n.
    for (&(n, expected), &measure) in BORDER_PLAT.iter().zip(&nodes_flat) {
        assert_eq!(measure, expected, "n={n}: flat border = {expected} live");
    }
    assert!(
        nodes_flat.windows(2).all(|w| w[1] > w[0]),
        "the flat prototype grows with n: {nodes_flat:?}"
    );
    // Firings = NodeCreated events == pool creation counter.
    let firings = *observer.firings.lock().unwrap();
    assert_eq!(firings, creations, "firings == hash-consing creations");
    // Passes = Round events: no saturation during the union construction
    // (the Round narration must not silently disappear).
    let passes = *observer.passes.lock().unwrap();
    assert_eq!(passes, 0, "no saturation round during the union");
}
