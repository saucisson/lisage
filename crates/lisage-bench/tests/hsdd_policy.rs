//! Tests of the HSDD-6 follow-up brick: observed rounds, self-detected
//! policy and per-fragment sub-fixpoints (the issue #388 remainder),
//! stacked on the saturation brick (PR #395, branch `feat/hsdd-saturation`)
//! and the transductions (#394)/spike (#393). The falsifiable claims:
//! (1) **Observer visibility**: the tower plugged with the recording
//!     observer emits the core trace - the pool storage events
//!     (`NodeCreated` at the saturation level, `NodeShared` on the
//!     re-saturation hit) and, per performed round, the core `Round`
//!     event with the ACTIVE and MODIFIED fragments (the n instantiated
//!     modules) and `growth` 0 (the sweeps intern nothing);
//! (2) **Self-detected policy** (the Hamez lift, no explicit order): the
//!     active set is derived from the change structure alone; on the
//!     regular ring it degenerates to the blind sweep (measured: same
//!     closure, same rounds, `firings == n * max(rounds, 1)` over every
//!     set and n = 3..=7), convergence equal to the enumeration oracle
//!     everywhere;
//! (3) **Per-fragment sub-fixpoints**: window-local fixpoints then
//!     lift-off through the shared boundary sheets converge to the same
//!     closure for fragment sizes 1 and 2 (measured pins: two parent
//!     passes, local fixpoint iterations 3n and 3n - 1 for the full set);
//! (4) **Chain composition**: a two-set chain [hungry; eat] saturates the
//!     initial state to the golden thread under every policy.
//! Any contrary measurement falsifies the tower lift of the policies.

use lisage_algebra::interface::{Anchor, Level, NodeId};

use lisage_core::arena::Arena;
use lisage_core::observer::{NoOpObserver, TraceEvent};

use lisage_bench::hsdd::{
    EVENT_EAT, EVENT_HUNGRY, EVENT_THINK, ErrorHsdd, LEVEL_SATURATION, RING_COUNTS,
    TowerController, TraceCollector, build_module, build_ring, count_closure,
    count_closure_reference, pack_event_set, pool_tower, pool_tower_observed, read_cells,
    read_saturation, saturate, saturate_detected, saturate_fragments, saturate_observed,
};

/// Test anchor of the tower.
const ANCHOR: Anchor = Anchor(0);

/// The full firing set.
fn full() -> u64 {
    pack_event_set(&[EVENT_HUNGRY, EVENT_EAT, EVENT_THINK])
}

/// Observed pool with a saturated tower of the given chain.
fn tower(n: u64, sets: &[u64]) -> (Arena<TowerController>, NodeId) {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR);
    let mut root = build_ring(&mut pool, ANCHOR, module, n).unwrap();
    for &set in sets {
        root = saturate(&mut pool, ANCHOR, root, set).unwrap();
    }
    (pool, root)
}

/// The observed tower emits the pool storage events and the round events
/// of the fold: the saturation node is created at its level, the
/// re-saturation is a shared hit, every performed sweep carries the
/// active and modified fragments with `growth` 0, and the sweep that
/// changes nothing confirms the fixpoint.
#[test]
fn test_observed_tower_storage_and_round_events() {
    let collector = TraceCollector::new();
    let mut pool = pool_tower_observed(Box::new(collector.clone()));
    let module = build_module(&mut pool, ANCHOR);
    let module_cells = read_cells(&pool, module).unwrap();
    let ring = build_ring(&mut pool, ANCHOR, module, 5).unwrap();
    let saturated = saturate(&mut pool, ANCHOR, ring, full()).unwrap();

    // The pool emits its own storage events through the observer.
    let created_levels: Vec<Option<Level>> = collector
        .events()
        .iter()
        .filter_map(|event| match event {
            TraceEvent::NodeCreated { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert!(
        created_levels.contains(&Some(Level(LEVEL_SATURATION))),
        "the saturation node is created at its level: {created_levels:?}"
    );

    // The re-saturation is a pool hit: the node is shared, not rebuilt.
    let again = saturate(&mut pool, ANCHOR, ring, full()).unwrap();
    assert_eq!(again, saturated);
    assert!(
        collector.events().iter().any(
            |event| matches!(*event, TraceEvent::NodeShared { node, .. } if node == saturated)
        ),
        "re-saturation is a storage hit"
    );

    // The observed saturation: the same closure as the fold of PR #395.
    let closed = saturate_observed(&pool, saturated, &collector).unwrap();
    assert_eq!(closed.count, 152, "the golden thread at n=5");
    assert_eq!(
        closed,
        count_closure(&module_cells, 5, &[full()]).unwrap(),
        "same semantics as the unobserved fold"
    );

    // The acceptance triple of #388: number, active and modified
    // fragments, one event per performed sweep.
    let rounds = collector.rounds();
    assert_eq!(rounds.len(), 2, "two performed sweeps at n=5");
    assert_eq!(rounds[0], (1, vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3, 4]));
    assert_eq!(
        rounds[1],
        (2, vec![0, 1, 2, 3, 4], vec![]),
        "the confirming sweep modifies nothing"
    );

    // The sweeps intern nothing: growth 0 and constant handles.
    let round_events: Vec<TraceEvent> = collector
        .events()
        .iter()
        .filter(|event| matches!(event, TraceEvent::Round { .. }))
        .cloned()
        .collect();
    assert_eq!(round_events.len(), 2);
    for event in &round_events {
        let TraceEvent::Round {
            before,
            image,
            after,
            growth,
            ..
        } = event
        else {
            unreachable!("filtered above");
        };
        assert_eq!(*before, saturated, "the tower root is not rebuilt");
        assert_eq!(image, before);
        assert_eq!(after, before);
        assert_eq!(*growth, 0, "the sweeps intern nothing");
    }

    // The op span opens and closes once: a balanced trace.
    let opens = collector
        .events()
        .iter()
        .filter(|event| matches!(event, TraceEvent::SpanOpen { .. }))
        .count();
    let closes = collector
        .events()
        .iter()
        .filter(|event| matches!(event, TraceEvent::SpanClose { .. }))
        .count();
    assert_eq!(opens, closes, "balanced spans");
    assert!(
        collector
            .events()
            .iter()
            .any(|event| matches!(event, TraceEvent::OpStart { .. })),
        "the saturation is one op"
    );
    assert!(
        collector
            .events()
            .iter()
            .any(|event| matches!(event, TraceEvent::OpEnd { .. })),
        "the op closes"
    );
}

/// The self-detected policy converges to the same closure as the blind
/// sweep and the enumeration oracle, everywhere; on the regular ring the
/// schedule degenerates to the blind one (every wave full), and the trace
/// carries one event per performed wave.
#[test]
fn test_self_detected_schedule_converges_with_measured_work() {
    for &(n, _) in RING_COUNTS {
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR);
        let cells = read_cells(&pool, module).unwrap();
        let ring = build_ring(&mut pool, ANCHOR, module, n).unwrap();
        for set in 1u64..=7 {
            let root = saturate(&mut pool, ANCHOR, ring, set).unwrap();
            let collector = TraceCollector::new();
            let detected = saturate_detected(&pool, root, &collector).unwrap();
            let blind = count_closure(&cells, n, &[set]).unwrap();
            let oracle = count_closure_reference(n, &[set]);
            assert_eq!(
                detected.closed.count, oracle,
                "detected vs oracle: n={n} set={set}"
            );
            assert_eq!(blind.count, oracle, "blind vs oracle: n={n} set={set}");
            assert_eq!(
                detected.closed.rounds, blind.rounds,
                "the uniform ring pays the same waves: n={n} set={set}"
            );
            let waves = if blind.rounds > 0 { blind.rounds } else { 1 };
            assert_eq!(
                detected.firings,
                waves * n,
                "the measured schedule work: n={n} set={set}"
            );
            let rounds = collector.rounds();
            assert_eq!(
                rounds.len() as u64,
                waves,
                "one event per performed wave: n={n} set={set}"
            );
            assert!(
                rounds
                    .last()
                    .is_some_and(|(_, _, modified)| modified.is_empty()),
                "the last wave confirms the fixpoint: n={n} set={set}"
            );
        }
    }
}

/// The per-fragment sub-fixpoints converge to the same closure for
/// fragment sizes 1 and 2 over every set and n, and the fragment size is
/// validated against the ring.
#[test]
fn test_fragment_sub_fixpoints_converge_same_closure() {
    for &(n, _) in RING_COUNTS {
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR);
        let ring = build_ring(&mut pool, ANCHOR, module, n).unwrap();
        for set in 1u64..=7 {
            let root = saturate(&mut pool, ANCHOR, ring, set).unwrap();
            let oracle = count_closure_reference(n, &[set]);
            for fragment in [1u64, 2] {
                let closed = saturate_fragments(&pool, root, fragment, &NoOpObserver).unwrap();
                assert_eq!(
                    closed.closed.count, oracle,
                    "fragments vs oracle: n={n} set={set} f={fragment}"
                );
            }
        }
        // The fragment size is validated against the ring.
        let root = saturate(&mut pool, ANCHOR, ring, 1).unwrap();
        assert_eq!(
            saturate_fragments(&pool, root, 0, &NoOpObserver).unwrap_err(),
            ErrorHsdd::RingSizeOutOfRange(0)
        );
        assert_eq!(
            saturate_fragments(&pool, root, n + 1, &NoOpObserver).unwrap_err(),
            ErrorHsdd::RingSizeOutOfRange(n + 1)
        );
    }
}

/// Measured pins of the per-fragment discipline on the full set: two
/// parent passes (build then confirm) at n=3 and n=7, the local fixpoint
/// iterations of the windows growing linearly in n (3n for fragment 1,
/// 3n - 1 for fragment 2 - the wrap of the circular ring).
#[test]
fn test_fragment_policies_measured_pins() {
    let pins: [(u64, u64, u64, u64); 2] = [(3, 20, 9, 8), (7, 1136, 21, 20)];
    for &(n, golden, lf1, lf2) in &pins {
        let (pool, root) = tower(n, &[full()]);
        let collector = TraceCollector::new();
        let one = saturate_fragments(&pool, root, 1, &NoOpObserver).unwrap();
        let two = saturate_fragments(&pool, root, 2, &collector).unwrap();
        assert_eq!(
            count_closure_reference(n, &[full()]),
            golden,
            "oracle n={n}"
        );
        assert_eq!(one.closed.count, golden, "fragment 1 n={n}");
        assert_eq!(two.closed.count, golden, "fragment 2 n={n}");
        assert_eq!(one.closed.rounds, 2, "build then confirm: n={n}");
        assert_eq!(two.closed.rounds, 2, "build then confirm: n={n}");
        assert_eq!(
            one.firings, one.local_fixpoints,
            "fragment 1: one position per window pass: n={n}"
        );
        assert_eq!(
            two.firings,
            two.local_fixpoints * 2,
            "fragment 2: two positions per window pass: n={n}"
        );
        assert_eq!(one.local_fixpoints, lf1, "n={n}");
        assert_eq!(two.local_fixpoints, lf2, "n={n}");
        // The trace: one round event per parent pass, the last quiet.
        let rounds = collector.rounds();
        assert_eq!(rounds.len(), 2, "n={n}");
        assert!(
            rounds
                .last()
                .is_some_and(|(_, _, modified)| modified.is_empty()),
            "the last pass confirms the fixpoint: n={n}"
        );
    }
}

/// A two-set chain [hungry; eat] saturates the initial state to the
/// golden thread under every policy: the nested chain carries the
/// boundary guard across its levels, and the closure of the chain IS
/// |R_n|.
#[test]
fn test_two_set_chain_saturates_to_the_golden() {
    let hungry = pack_event_set(&[EVENT_HUNGRY]);
    let eat = pack_event_set(&[EVENT_EAT]);
    let chain = [hungry, eat];
    for &(n, golden) in RING_COUNTS {
        let (pool, root) = tower(n, &chain);
        assert_eq!(
            read_saturation(&pool, root).unwrap().sets,
            vec![hungry, eat],
            "innermost first"
        );
        assert_eq!(count_closure_reference(n, &chain), golden, "oracle n={n}");

        let observed = saturate_observed(&pool, root, &NoOpObserver).unwrap();
        assert_eq!(observed.count, golden, "observed n={n}");
        assert_eq!(observed.rounds, 4, "two sweeps per chain set: n={n}");

        let detected = saturate_detected(&pool, root, &NoOpObserver).unwrap();
        assert_eq!(detected.closed.count, golden, "detected n={n}");
        assert_eq!(detected.closed.rounds, 4, "n={n}");
        assert_eq!(detected.firings, 4 * n, "n={n}");

        let fragments = saturate_fragments(&pool, root, 2, &NoOpObserver).unwrap();
        assert_eq!(fragments.closed.count, golden, "fragments n={n}");
        assert_eq!(fragments.closed.rounds, 4, "n={n}");
    }
}
