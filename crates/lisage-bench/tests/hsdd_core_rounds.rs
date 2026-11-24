//! Tests of the HSDD-6 core brick: per-round materialized diagrams and
//! per-fragment memory versions (the issue #388 core remainder), stacked
//! on the policy brick (PR #396). The falsifiable claims:
//! (1) **Closure conservation**: the materialized saturation — every
//!     round a simultaneous firing of the set through the core saturate
//!     — reaches the same closure as the PR #395 fold and the
//!     enumeration oracle, over every set and ring size;
//! (2) **Real round diagrams**: the core `Round` events carry the real
//!     `before`/`image`/`after` handles and the real pool `growth`; the
//!     continuity holds (`after` one round IS `before` the next), and
//!     every handle reads back as the coupling chain it claims to be
//!     (the seed sheets are the think-think singletons, the final sheets
//!     the 8 legal pairs without eat-eat);
//! (3) **Materialization is hash-consed**: a full-ring saturation
//!     interns `2n` coupling sheets (seed chain + image chain), the
//!     confirming re-intern is pure R-share; re-running the engine
//!     interns NOTHING (the whole diagram is one big pool share);
//! (4) **Cross-round memo reuse**: the runner persists across the
//!     rounds (core M2-5), measured: on the full ring every memo hit of
//!     the saturation happens in round >= 2 (`7n + 1` hits, `2n + 1`
//!     misses, all hits late) — the seed round misses only;
//! (5) **Chain handoff**: a two-set chain [hungry; eat] saturates to the
//!     golden thread in 2 rounds per set (4 events, numbers restarting
//!     per set), each set handed the fixpoint state of the previous
//!     one, count equal to the PR #396 engines.
//! Any contrary measurement falsifies the core port of #388.

use std::sync::Arc;

use lisage_algebra::interface::{Anchor, NodeId};

use lisage_core::arena::Arena;
use lisage_core::observer::{Observer, SpanKind, TraceEvent};

use lisage_bench::hsdd::{
    DiagrammedClosure, EAT, EVENT_EAT, EVENT_HUNGRY, EVENT_THINK, LEVEL_COUPLING_BASE, RING_COUNTS,
    TowerController, TraceCollector, build_module, build_ring, count_closure,
    count_closure_reference, pack_event_set, pool_tower, pool_tower_observed, read_cells,
    read_coupling, read_saturation, saturate, saturate_diagrammed, saturate_observed,
};

/// Test anchor of the tower.
const ANCHOR: Anchor = Anchor(0);

/// The full firing set.
fn full() -> u64 {
    pack_event_set(&[EVENT_HUNGRY, EVENT_EAT, EVENT_THINK])
}

/// Observed tower saturated with the given chain of sets: the collector
/// receives the pool storage events (the pool port) and the saturation
/// trace (the runner port) — both observers share the same collector.
fn observed_tower(n: u64, sets: &[u64]) -> (Arena<TowerController>, NodeId, TraceCollector) {
    let collector = TraceCollector::new();
    let mut pool = pool_tower_observed(Box::new(collector.clone()));
    let module = build_module(&mut pool, ANCHOR);
    let mut root = build_ring(&mut pool, ANCHOR, module, n).unwrap();
    for &set in sets {
        root = saturate(&mut pool, ANCHOR, root, set).unwrap();
    }
    (pool, root, collector)
}

/// Runs the diagrammed engine on the tower, through the shared collector.
fn diagrammed(
    pool: &mut Arena<TowerController>,
    root: NodeId,
    collector: &TraceCollector,
) -> DiagrammedClosure {
    let observer: Arc<dyn Observer> = Arc::new(collector.clone());
    saturate_diagrammed(pool, root, ANCHOR, Some(observer)).unwrap()
}

/// The round events of the trace, in order.
fn round_events(collector: &TraceCollector) -> Vec<TraceEvent> {
    collector
        .events()
        .iter()
        .filter(|event| matches!(event, TraceEvent::Round { .. }))
        .cloned()
        .collect()
}

/// The span ids of the Round spans, in order.
fn round_spans(collector: &TraceCollector) -> Vec<u64> {
    collector
        .events()
        .iter()
        .filter_map(|event| match event {
            TraceEvent::SpanOpen { span, kind, .. } if *kind == SpanKind::Round => Some(span.0),
            _ => None,
        })
        .collect()
}

/// Total number of pairs of a read-back sheet.
fn pair_sum(matrix: &[Vec<u64>]) -> u64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .copied()
        .sum::<u64>()
}

/// The materialized saturation conserves the closure of the fold of
/// PR #395 everywhere: over every firing set and ring size the count
/// read back from the final diagram equals the fold and the
/// enumeration oracle; only the sets able to leave the all-think seed
/// pay a confirming round (the others saturate on the seed itself).
#[test]
fn test_diagrammed_closure_matches_references_everywhere() {
    for &(n, golden) in RING_COUNTS {
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR);
        let cells = read_cells(&pool, module).unwrap();
        let ring = build_ring(&mut pool, ANCHOR, module, n).unwrap();
        for set in 1u64..=7 {
            let root = saturate(&mut pool, ANCHOR, ring, set).unwrap();
            let closed = saturate_diagrammed(&mut pool, root, ANCHOR, None).unwrap();
            let oracle = count_closure_reference(n, &[set]);
            assert_eq!(
                closed.count, oracle,
                "diagrammed vs oracle: n={n} set={set}"
            );
            assert_eq!(
                count_closure(&cells, n, &[set]).unwrap().count,
                oracle,
                "fold vs oracle: n={n} set={set}"
            );
            // The seed is all-think: a set without hungry leaves it
            // untouched in one confirming-only iteration; a set with
            // hungry builds the coupling in one round then confirms.
            let expected_rounds = if set & (1 << EVENT_HUNGRY) == 0 { 1 } else { 2 };
            assert_eq!(closed.rounds, expected_rounds, "rounds: n={n} set={set}");
            assert_eq!(closed.states.len(), 1, "one set, one fixpoint state");
        }
        // The full set reaches the golden thread.
        let root = saturate(&mut pool, ANCHOR, ring, full()).unwrap();
        let closed = saturate_diagrammed(&mut pool, root, ANCHOR, None).unwrap();
        assert_eq!(closed.count, golden, "golden thread: n={n}");
    }
}

/// The core Round events carry the real materialized handles: round 1
/// rebuilds every sheet (image distinct from before, growth n, all
/// positions modified), round 2 confirms on pure R-share (growth 0,
/// nothing modified), and every handle reads back as the coupling
/// chain it claims to be — the round-trip witness of the
/// materialization: the diagram IS the coupling state.
#[test]
fn test_round_events_materialize_real_diagrams() {
    let (mut pool, root, collector) = observed_tower(5, &[full()]);
    let closed = diagrammed(&mut pool, root, &collector);
    assert_eq!(closed.count, 152, "the golden thread at n=5");

    let rounds = round_events(&collector);
    assert_eq!(rounds.len(), 2, "one building round, one confirming");
    let TraceEvent::Round {
        number: number_1,
        active_levels,
        modified_levels,
        before: before_1,
        image: image_1,
        after: after_1,
        growth: growth_1,
        ..
    } = &rounds[0]
    else {
        unreachable!("filtered above");
    };
    assert_eq!(*number_1, 1);
    assert_eq!(
        *active_levels,
        vec![
            LEVEL_COUPLING_BASE,
            LEVEL_COUPLING_BASE + 1,
            LEVEL_COUPLING_BASE + 2,
            LEVEL_COUPLING_BASE + 3,
            LEVEL_COUPLING_BASE + 4
        ],
        "one fragment level per position"
    );
    assert_eq!(modified_levels, active_levels, "every sheet is rewritten");
    assert_ne!(*image_1, *before_1, "the image chain is new");
    assert_eq!(*after_1, *image_1, "the union keeps the image");
    assert_eq!(*growth_1, 5, "one new sheet per position");

    let TraceEvent::Round {
        number: number_2,
        modified_levels: modified_2,
        before: before_2,
        image: image_2,
        after: after_2,
        growth: growth_2,
        ..
    } = &rounds[1]
    else {
        unreachable!("filtered above");
    };
    assert_eq!(*number_2, 2);
    assert_eq!(*before_2, *after_1, "continuity: after IS the next before");
    assert!(
        modified_2.is_empty(),
        "the confirming round modifies nothing"
    );
    assert_eq!(*image_2, *before_2, "the image reproduces the chain");
    assert_eq!(*after_2, *before_2, "the union is identity");
    assert_eq!(*growth_2, 0, "the confirming round interns nothing");
    assert_eq!(*after_2, closed.state, "the final round is the final state");

    // The read-back round trip: the seed sheets are the think-think
    // singletons (the golden alphabet is (think, hungry, eat), so the
    // pc v sits at index v; EAT = 2), the final sheets carry the 8
    // legal pairs without eat-eat.
    let seed = read_coupling(&pool, *before_1, 5, 3).unwrap();
    for (position, matrix) in seed.iter().enumerate() {
        assert_eq!(pair_sum(matrix), 1, "seed sheet {position}");
    }
    let final_matrices = read_coupling(&pool, closed.state, 5, 3).unwrap();
    for (position, matrix) in final_matrices.iter().enumerate() {
        assert_eq!(pair_sum(matrix), 8, "final sheet {position}");
        assert_eq!(matrix[EAT as usize][EAT as usize], 0, "no eat-eat pair");
    }
}

/// The measured pins of the materialized storage: the full-set
/// saturation interns exactly two coupling chains (the seed and the
/// image, 2n sheets), the confirming re-intern costs n R-shares
/// (the terminal hit not counted, it is below the coupling levels),
/// and the runner persists across the rounds: on the uniform ring the
/// seed round misses every visit, all `7n + 1` memo hits happen in
/// the confirming round — a single-round execution of the same
/// saturation would answer none of them.
#[test]
fn test_materialized_storage_and_cross_round_memo_pins() {
    for &(n, _) in RING_COUNTS {
        let (mut pool, root, collector) = observed_tower(n, &[full()]);
        let closed = diagrammed(&mut pool, root, &collector);
        assert_eq!(
            closed.count,
            count_closure_reference(n, &[full()]),
            "oracle n={n}"
        );

        let events = collector.events();
        let spans = round_spans(&collector);
        assert_eq!(spans.len(), 2, "two rounds: n={n}");
        let created = events
            .iter()
            .filter(|event| match event {
                TraceEvent::NodeCreated { level, .. } => {
                    matches!(level, Some(level) if level.0 >= LEVEL_COUPLING_BASE)
                }
                _ => false,
            })
            .count();
        assert_eq!(created, 2 * n as usize, "seed chain + image chain: n={n}");
        let shared = events
            .iter()
            .filter(|event| matches!(event, TraceEvent::NodeShared { .. }))
            .count();
        assert_eq!(shared, 2 * n as usize + 2, "the rest is R-share: n={n}");
        let confirming_shares = events
            .iter()
            .filter(|event| match event {
                TraceEvent::NodeShared { span, .. } => span.is_some_and(|s| s.0 >= spans[1]),
                _ => false,
            })
            .count();
        assert_eq!(confirming_shares, n as usize, "confirming round: n={n}");

        // The memo: the runner survives the rounds (core M2-5).
        let hits = events
            .iter()
            .filter(|event| matches!(event, TraceEvent::MemoHit { .. }))
            .count();
        let misses = events
            .iter()
            .filter(|event| matches!(event, TraceEvent::MemoMiss { .. }))
            .count();
        let late_hits = events
            .iter()
            .filter(|event| match event {
                TraceEvent::MemoHit { span, .. } => span.is_some_and(|s| s.0 > spans[1]),
                _ => false,
            })
            .count();
        assert_eq!(misses, 2 * n as usize + 1, "seed-round visits: n={n}");
        assert_eq!(hits, 7 * n as usize + 1, "cross-round reuse: n={n}");
        assert_eq!(
            late_hits, hits,
            "on the uniform ring every hit answers from round 1: n={n}"
        );
    }
}

/// A two-set chain [hungry; eat] saturates the materialized state to
/// the golden thread: two rounds per set (the numbers restarting at
/// each set), the fixpoint state of one set handed over as the first
/// `before` of the next, the continuity `after == next before`
/// holding through the whole trace, and the count equal to the
/// unmaterialized engines of PR #396.
#[test]
fn test_chain_sets_hand_states_over() {
    let hungry = pack_event_set(&[EVENT_HUNGRY]);
    let eat = pack_event_set(&[EVENT_EAT]);
    let chain = [hungry, eat];
    for &(n, golden) in RING_COUNTS {
        let (mut pool, root, collector) = observed_tower(n, &chain);
        let closed = diagrammed(&mut pool, root, &collector);
        assert_eq!(closed.count, golden, "chain golden n={n}");
        assert_eq!(
            read_saturation(&pool, root).unwrap().sets,
            vec![hungry, eat]
        );

        let rounds = round_events(&collector);
        assert_eq!(rounds.len(), 4, "two rounds per set: n={n}");
        assert_eq!(closed.rounds, 4, "n={n}");
        assert_eq!(closed.states.len(), 2, "n={n}");
        let numbers: Vec<u32> = rounds
            .iter()
            .map(|event| match event {
                TraceEvent::Round { number, .. } => *number,
                _ => unreachable!("filtered above"),
            })
            .collect();
        assert_eq!(numbers, vec![1, 2, 1, 2], "numbers restart per set: n={n}");

        // Continuity: after IS the next before, through the whole trace.
        let mut previous_after: Option<NodeId> = None;
        for (index, event) in rounds.iter().enumerate() {
            let TraceEvent::Round { before, after, .. } = event else {
                unreachable!("filtered above");
            };
            if let Some(previous) = previous_after {
                assert_eq!(*before, previous, "continuity within the trace: n={n}");
            }
            if index == 2 {
                assert_eq!(*before, closed.states[0], "the handoff: n={n}");
            }
            previous_after = Some(*after);
        }
        assert_eq!(closed.states[1], closed.state, "the last fixpoint: n={n}");

        // The memo pins: the reuse is visible inside each saturate
        // call (the hits of the confirming rounds).
        let spans = round_spans(&collector);
        assert_eq!(spans.len(), 4, "n={n}");
        let late = |from: usize| {
            collector
                .events()
                .iter()
                .filter(|event| match event {
                    TraceEvent::MemoHit { span, .. } => span.is_some_and(|s| s.0 > spans[from]),
                    _ => false,
                })
                .count()
        };
        assert!(late(1) >= 1, "cross-round reuse in the first set: n={n}");
        assert!(late(3) >= 1, "cross-round reuse in the second set: n={n}");

        // The count equals the unmaterialized fold evaluated on the
        // same tower (the PR #396 engine, same pool).
        let observed = saturate_observed(&pool, root, &collector).unwrap();
        assert_eq!(observed.count, golden, "vs the observed engine: n={n}");
    }
}

/// The materialization is hash-consed: re-running the engine on the
/// same tower rebuilds the identical diagram without interning a
/// single node (the whole run is one big pool share), the handles
/// deterministic, the count read back in the same rounds.
#[test]
fn test_rerun_is_one_pool_share() {
    let (mut pool, root, collector) = observed_tower(5, &[full()]);
    let first = diagrammed(&mut pool, root, &collector);
    assert_eq!(first.count, 152);

    let before = collector.events().len();
    let second = diagrammed(&mut pool, root, &collector);
    assert_eq!(first.state, second.state, "hash-consed determinism");
    assert_eq!(first.count, second.count, "same closure");
    assert_eq!(first.rounds, second.rounds, "same schedule");
    assert_eq!(first.states, second.states, "same per-set fixpoints");

    let created = collector.events()[before..]
        .iter()
        .filter(|event| matches!(event, TraceEvent::NodeCreated { .. }))
        .count();
    assert_eq!(created, 0, "the rerun interns nothing");
}
