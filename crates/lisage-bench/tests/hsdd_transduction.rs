//! Tests of the HSDD transduction brick (HSDD-5, issue #387) stacked on
//! the tower spike (#382). The falsifiable claims:
//! (1) **Storage**: one node per (event, position) transduction — a hit on
//!     retransduction — with the ring and the module shared, never
//!     duplicated, and chains stacking one node per event;
//! (2) **Semantics**: the structural fold of the transduced tower equals
//!     the enumeration oracle for n = 3..=7, every position, the three
//!     module-local events and event chains (image cardinalities, never
//!     expanding the product);
//! (3) **The boundary crossing**: the `eat` guard spans the *neighbours*
//!     of the fired position, outside the fired cell — the fold carries it
//!     (a firing beside an eater would build an illegal level word),
//!     pinned concretely on ring 3, and no image state ever escapes the
//!     five shared module cells (the `left` fields are derived pairwise).
//! Any contrary measurement falsifies the spike's mirroring of the HSDD
//! transduction gains.

use lisage_algebra::interface::{Anchor, Form, Level, Pool};

use lisage_bench::hsdd::{
    Cell, EAT, EVENT_EAT, EVENT_HUNGRY, EVENT_THINK, HUNGRY, LEVEL_EVENT, RING_COUNTS, THINK,
    build_module, build_ring, count_image, count_image_reference, count_words_reference, fire_pc,
    fork_state, pool_tower, read_cells, read_transduction, transduce, unpack_event,
    words_reference,
};

/// Test anchor of the tower.
const ANCHOR_TOWER: Anchor = Anchor(0);

/// The transduced tower stores one node per (event, position) and shares
/// everything else: the retransduction is a hash-consing hit, the ring and
/// the module are the same nodes as before the transduction, chains stack.
#[test]
fn test_transduction_storage_one_node_per_event() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let ring = build_ring(&mut pool, ANCHOR_TOWER, module, 5).unwrap();
    let live_ring = pool.live();

    let fired = transduce(&mut pool, ANCHOR_TOWER, ring, EVENT_HUNGRY, 1).unwrap();
    assert_eq!(pool.live(), live_ring + 1, "one node per transduction");

    // Hash-consing hit: the same event on the same tower is the same node.
    let again = transduce(&mut pool, ANCHOR_TOWER, ring, EVENT_HUNGRY, 1).unwrap();
    assert_eq!(again, fired);
    assert_eq!(pool.live(), live_ring + 1);

    // The event is data on the arc, the target is the untouched ring.
    let Form::Node { level, arcs } = pool.resolve(fired).unwrap() else {
        panic!("the transduction is a node");
    };
    assert_eq!(level, Level(LEVEL_EVENT));
    let Some((key, target)) = arcs.iter().next() else {
        panic!("one arc");
    };
    assert_eq!(*target, ring, "the ring is shared, not duplicated");
    assert_eq!(unpack_event(*key), (EVENT_HUNGRY, 1));

    // The read chain exposes the shared module and the ring size.
    let read = read_transduction(&pool, fired).unwrap();
    assert_eq!(read.events, vec![(EVENT_HUNGRY, 1)]);
    assert_eq!(read.module, module);
    assert_eq!(read.ring_size(), 5);

    // Chains stack event nodes: one per level, the ring still shared.
    let chained = transduce(&mut pool, ANCHOR_TOWER, fired, EVENT_EAT, 1).unwrap();
    assert_eq!(pool.live(), live_ring + 2);
    let read_chain = read_transduction(&pool, chained).unwrap();
    assert_eq!(
        read_chain.events,
        vec![(EVENT_HUNGRY, 1), (EVENT_EAT, 1)],
        "application order, innermost first"
    );
    assert_eq!(read_chain.module, module);
    assert_eq!(read_chain.ring_size(), 5);
}

/// Fold == enumeration on the transduced tower: every position, the three
/// events, n = 3..=7 (position symmetry is measured, not assumed), and
/// every image is a subset of the legal words.
#[test]
fn test_transduction_semantics_single_events_all_positions() {
    for &(n, ring_count) in RING_COUNTS {
        assert!(ring_count <= count_words_reference(n));
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR_TOWER);
        let cells = read_cells(&pool, module).unwrap();
        let ring = build_ring(&mut pool, ANCHOR_TOWER, module, n).unwrap();
        for event in [EVENT_HUNGRY, EVENT_EAT, EVENT_THINK] {
            for position in 0..n {
                let fired = transduce(&mut pool, ANCHOR_TOWER, ring, event, position).unwrap();
                let read = read_transduction(&pool, fired).unwrap();
                assert_eq!(read.events, vec![(event, position)]);
                let folded = count_image(&cells, n, &read.events).unwrap();
                let enumerated = count_image_reference(n, &[(event, position)]);
                assert_eq!(
                    folded, enumerated,
                    "fold != oracle: n={n} event={event} at={position}"
                );
                assert!(folded <= ring_count, "image escapes R_n");
            }
        }
    }
}

/// Chains: the fold composes the events through the shared coupling —
/// same position, spread positions, three levels — and equals the oracle.
#[test]
fn test_transduction_semantics_chains() {
    let chains: [Vec<(u64, u64)>; 4] = [
        vec![(EVENT_HUNGRY, 1), (EVENT_EAT, 1)],
        vec![(EVENT_EAT, 1), (EVENT_THINK, 1)],
        vec![(EVENT_HUNGRY, 0), (EVENT_EAT, 2)],
        vec![(EVENT_HUNGRY, 1), (EVENT_EAT, 0), (EVENT_THINK, 2)],
    ];
    for &(n, _) in RING_COUNTS {
        let mut pool = pool_tower();
        let module = build_module(&mut pool, ANCHOR_TOWER);
        let cells = read_cells(&pool, module).unwrap();
        let ring = build_ring(&mut pool, ANCHOR_TOWER, module, n).unwrap();
        for chain in &chains {
            let mut node = ring;
            for &(event, position) in chain {
                node = transduce(&mut pool, ANCHOR_TOWER, node, event, position).unwrap();
            }
            let read = read_transduction(&pool, node).unwrap();
            assert_eq!(read.events, chain.clone(), "application order preserved");
            let folded = count_image(&cells, n, &read.events).unwrap();
            let enumerated = count_image_reference(n, chain);
            assert_eq!(folded, enumerated, "fold != oracle: n={n} chain={chain:?}");
        }
    }
}

/// The `eat` guard crosses the fragment boundary: enabled words of ring 3
/// at position 0 are exactly the legal words with pc 0 hungry and no eater
/// on either side — the count is measured both by the fold (which carries
/// guards through the level legality) and directly.
#[test]
fn test_eat_guard_crosses_the_boundary() {
    let n = 3;
    // Direct pin (oracle-side, independent of the fold): pc 0 hungry,
    // left neighbour (position 2) and right neighbour (position 1) not
    // eating.
    let enabled = words_reference(n)
        .into_iter()
        .filter(|word| word[0] == HUNGRY && word[1] != EAT && word[2] != EAT)
        .count() as u64;

    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let cells = read_cells(&pool, module).unwrap();
    let ring = build_ring(&mut pool, ANCHOR_TOWER, module, n).unwrap();
    let fired = transduce(&mut pool, ANCHOR_TOWER, ring, EVENT_EAT, 0).unwrap();
    let read = read_transduction(&pool, fired).unwrap();

    let folded = count_image(&cells, n, &read.events).unwrap();
    assert_eq!(folded, enabled, "the fold carries the boundary guard");
    assert_eq!(folded, count_image_reference(n, &[(EVENT_EAT, 0)]));
}

/// The events close the module: every image state of a legal pair stays
/// inside the five shared cells (the `left` field is derived pairwise, so
/// a firing that takes a neighbour fork updates it without new forms).
#[test]
fn test_events_close_the_module_cells() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let cells = read_cells(&pool, module).unwrap();
    assert_eq!(cells.len(), 5, "the module has five cells");
    for event in [EVENT_HUNGRY, EVENT_EAT, EVENT_THINK] {
        for previous in [THINK, HUNGRY, EAT] {
            for pc in [THINK, HUNGRY, EAT] {
                let Some(fired) = fire_pc(event, pc) else {
                    continue;
                };
                // A firing beside an eater does not happen (guard), every
                // other image state of the pair stays in the module.
                if previous != EAT || fired != EAT {
                    let image = Cell {
                        pc: fired,
                        left: fork_state(previous, fired),
                    };
                    assert!(
                        cells.contains(&image),
                        "image cell {image:?} escapes the module (event {event})"
                    );
                }
            }
        }
    }
}
