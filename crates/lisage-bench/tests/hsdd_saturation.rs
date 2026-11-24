//! Tests of the HSDD saturation brick (HSDD-6, issue #388) stacked on the
//! transductions brick (PR #394, issue #387) and the tower spike (PR #393,
//! issue #382). The falsifiable claims:
//! (1) **Storage**: one node per firing set — a hit on re-saturation —
//!     with the ring and the module shared, never duplicated, and nested
//!     chains stacking one node per level (recursive saturation);
//! (2) **Semantics**: the structural fold of the closure equals the BFS
//!     enumeration oracle for n = 3..=7 and every event set — the product
//!     is never expanded;
//! (3) **The golden thread**: the full event set saturates the initial
//!     state to exactly |R_n| (20/56/152/416/1136) — the closure IS the
//!     reachability set the spike folded — and the recursion is
//!     idempotent (a nested set saturates the closure again: same count,
//!     no extra sweep).
//! Any contrary measurement falsifies the spike's mirroring of the HSDD
//! saturation gains.

use lisage_algebra::interface::{Anchor, Form, Level, Pool};

use lisage_bench::hsdd::{
    EVENT_EAT, EVENT_HUNGRY, EVENT_THINK, ErrorHsdd, LEVEL_SATURATION, RING_COUNTS, build_module,
    build_ring, count_closure, count_closure_reference, count_ring, count_words_reference,
    coupling, pack_event_set, pool_tower, read_cells, read_saturation, saturate,
};

/// Test anchor of the tower.
const ANCHOR_TOWER: Anchor = Anchor(0);

/// The saturated tower stores one node per firing set and shares
/// everything else: the re-saturation is a hash-consing hit, the ring and
/// the module are the same nodes as before, nested chains stack one node
/// per level.
#[test]
fn test_saturation_storage_one_node_per_set() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let ring = build_ring(&mut pool, ANCHOR_TOWER, module, 5).unwrap();
    let live_ring = pool.live();
    let hungry_think = pack_event_set(&[EVENT_HUNGRY, EVENT_THINK]);

    let saturated = saturate(&mut pool, ANCHOR_TOWER, ring, hungry_think).unwrap();
    assert_eq!(pool.live(), live_ring + 1, "one node per firing set");

    // Hash-consing hit: the same set on the same tower is the same node.
    let again = saturate(&mut pool, ANCHOR_TOWER, ring, hungry_think).unwrap();
    assert_eq!(again, saturated);
    assert_eq!(pool.live(), live_ring + 1);

    // The set is data on the arc, the target is the untouched ring.
    let Form::Node { level, arcs } = pool.resolve(saturated).unwrap() else {
        panic!("the saturation is a node");
    };
    assert_eq!(level, Level(LEVEL_SATURATION));
    let Some((key, target)) = arcs.iter().next() else {
        panic!("one arc");
    };
    assert_eq!(*target, ring, "the ring is shared, not duplicated");
    assert_eq!(*key, hungry_think);
    assert_eq!(
        pack_event_set(&[EVENT_HUNGRY, EVENT_EAT, EVENT_THINK]),
        7,
        "the full set packs to one key"
    );

    // Sets outside the event alphabet are rejected.
    assert_eq!(
        saturate(&mut pool, ANCHOR_TOWER, ring, 0).unwrap_err(),
        ErrorHsdd::EventSetOutOfRange(0)
    );
    assert_eq!(
        saturate(&mut pool, ANCHOR_TOWER, ring, 8).unwrap_err(),
        ErrorHsdd::EventSetOutOfRange(8)
    );

    // Nested chains stack saturation nodes: one per level, the ring still
    // shared (recursive saturation).
    let eat = pack_event_set(&[EVENT_EAT]);
    let nested = saturate(&mut pool, ANCHOR_TOWER, saturated, eat).unwrap();
    assert_eq!(pool.live(), live_ring + 2);
    let read = read_saturation(&pool, nested).unwrap();
    assert_eq!(
        read.sets,
        vec![hungry_think, eat],
        "application order, innermost first"
    );
    assert_eq!(read.module, module);
    assert_eq!(read.ring_size(), 5);

    // The single saturation reads one set.
    let read_single = read_saturation(&pool, saturated).unwrap();
    assert_eq!(read_single.sets, vec![hungry_think]);
}

/// Fold == BFS oracle on the closure: every event set, n = 3..=7 (sweep
/// depth is bounded, position symmetry is measured, not assumed).
#[test]
fn test_saturation_closure_equals_bfs_oracle_all_sets() {
    for &(n, _) in RING_COUNTS {
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR_TOWER);
        let cells = read_cells(&pool, module).unwrap();
        for set in 1..=7u64 {
            let closed = count_closure(&cells, n, &[set]).unwrap();
            let enumerated = count_closure_reference(n, &[set]);
            assert_eq!(closed.count, enumerated, "fold != oracle: n={n} set={set}");
            assert!(
                closed.rounds <= 3 * 3,
                "unexpected sweep depth: n={n} set={set} rounds={}",
                closed.rounds
            );
        }
    }
}

/// The full event set saturates the initial state to the golden thread:
/// the closed count is |R_n| over the whole pinned range, equal to the
/// enumeration of the legal words, to the circular trace of the spike's
/// coupling, and to the BFS oracle of the closure.
#[test]
fn test_full_saturation_recovers_the_golden_coupling() {
    let full = pack_event_set(&[EVENT_HUNGRY, EVENT_EAT, EVENT_THINK]);
    assert_eq!(full, 7);
    for &(n, golden) in RING_COUNTS {
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR_TOWER);
        let cells = read_cells(&pool, module).unwrap();
        let ring = build_ring(&mut pool, ANCHOR_TOWER, module, n).unwrap();
        let saturated = saturate(&mut pool, ANCHOR_TOWER, ring, full).unwrap();
        let read = read_saturation(&pool, saturated).unwrap();
        assert_eq!(read.sets, vec![full]);
        let closed = count_closure(&cells, n, &read.sets).unwrap();
        assert_eq!(closed.count, golden, "n={n}");
        assert_eq!(closed.count, count_words_reference(n), "n={n}");
        assert_eq!(closed.count, count_ring(&coupling(&cells), n), "n={n}");
        assert_eq!(
            closed.count,
            count_closure_reference(n, &read.sets),
            "n={n}"
        );
    }
}

/// Concrete pins on ring 3 (all oracle-checked): hungry alone closes on
/// the {think, hungry} words (8, two sweeps), eat or think alone never
/// fire from the initial state (1, zero sweeps), the sets with hungry and
/// eat recover the golden 20 (two sweeps).
#[test]
fn test_saturation_pins_ring_three() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let cells = read_cells(&pool, module).unwrap();
    let pins: &[(u64, u64)] = &[
        (pack_event_set(&[EVENT_HUNGRY]), 8),
        (pack_event_set(&[EVENT_HUNGRY, EVENT_THINK]), 8),
        (pack_event_set(&[EVENT_EAT]), 1),
        (pack_event_set(&[EVENT_THINK]), 1),
        (pack_event_set(&[EVENT_EAT, EVENT_THINK]), 1),
        (pack_event_set(&[EVENT_HUNGRY, EVENT_EAT]), 20),
        (pack_event_set(&[EVENT_HUNGRY, EVENT_EAT, EVENT_THINK]), 20),
    ];
    for &(set, expected) in pins {
        let closed = count_closure(&cells, 3, &[set]).unwrap();
        assert_eq!(closed.count, expected, "set={set}");
        assert_eq!(
            closed.count,
            count_closure_reference(3, &[set]),
            "set={set}"
        );
    }
}

/// Recursive saturation is idempotent: nesting the full set over its own
/// closure reaches the same fixpoint — same count, no extra sweep — both
/// on the fold of the chain and through the tower itself.
#[test]
fn test_recursive_saturation_is_idempotent() {
    let full = pack_event_set(&[EVENT_HUNGRY, EVENT_EAT, EVENT_THINK]);
    let n = 6;
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let cells = read_cells(&pool, module).unwrap();

    let single = count_closure(&cells, n, &[full]).unwrap();
    assert_eq!(single.count, 416, "the golden thread at n=6");
    let nested = count_closure(&cells, n, &[full, full]).unwrap();
    assert_eq!(nested.count, single.count, "the closure is closed");
    assert_eq!(nested.rounds, single.rounds, "the nested set does no sweep");

    // Through the tower: a saturated saturation reads two levels and
    // folds the same fixpoint.
    let ring = build_ring(&mut pool, ANCHOR_TOWER, module, n).unwrap();
    let saturated = saturate(&mut pool, ANCHOR_TOWER, ring, full).unwrap();
    let nested_node = saturate(&mut pool, ANCHOR_TOWER, saturated, full).unwrap();
    let read = read_saturation(&pool, nested_node).unwrap();
    assert_eq!(read.sets, vec![full, full]);
    assert_eq!(read.module, module);
    assert_eq!(read.ring_size(), n);
    let folded = count_closure(&cells, n, &read.sets).unwrap();
    assert_eq!(folded.count, single.count);
    assert_eq!(folded.rounds, single.rounds);
}
