//! Tests of the unique table without form clone (MEM-14, ADR-0103): open
//! addressing of `(hash, anchor, NodeId)` at load factor 0.5; the
//! hash-consing key is still the semantic form (compared through the stored
//! entry, never cloned); `ControllerId` is out of the key (constant per
//! pool). End-to-end bytes/node counter on a BDD instance.

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::bdd::BddController;
use lisage_core::observer::NoOpObserver;

/// Baseline of the issue: ~420 bytes per node with the cloned-form
/// `HashMap` key on a BDD instance. MEM-14 target: factor 5 at least.
const BASELINE_BYTES_PER_NODE: f64 = 420.0;
const FACTEUR_CIBLE: f64 = 5.0;

fn pool_bdd() -> Arena<BddController> {
    Arena::new(BddController::new(64), Box::new(NoOpObserver))
}

/// Intern `count` distinct degree-2 nodes at distinct levels (arcs to two
/// distinct terminals — no R-collapse), BDD cardinality 2.
fn intern_2n(pool: &mut Arena<BddController>, count: u32) -> Vec<NodeId> {
    let t0 = pool.terminal(Anchor(0), false);
    let t1 = pool.terminal(Anchor(0), true);
    (0..count)
        .map(|i| {
            pool.intern(
                Anchor(0),
                Form::Node {
                    level: Level(i),
                    arcs: ArcRun::from_pairs([(0, t0), (1, t1)]),
                },
            )
        })
        .collect()
}

#[test]
fn test_bytes_per_node_bdd_factor_5() {
    // End-to-end counter on a BDD instance: unique table + arenas + shadow
    // index over live nodes. Target: at least factor 5 under the ~420 B
    // baseline of the cloned-form key.
    let mut pool = pool_bdd();
    let nodes = intern_2n(&mut pool, 2000);
    assert_eq!(pool.live(), nodes.len() + 2, "2000 nodes + 2 terminals");
    let bytes_per_node = pool.bytes_per_node();
    assert!(
        bytes_per_node <= BASELINE_BYTES_PER_NODE / FACTEUR_CIBLE,
        "bytes/node {bytes_per_node:.1} must be ≤ {}/{FACTEUR_CIBLE} = {:.1}",
        BASELINE_BYTES_PER_NODE,
        BASELINE_BYTES_PER_NODE / FACTEUR_CIBLE
    );
}

#[test]
fn test_zero_allocation_on_reinterning() {
    // Re-interning an existing form allocates nothing in the unique table:
    // the table footprint is unchanged and the hit is counted as a share.
    let mut pool = pool_bdd();
    let nodes = intern_2n(&mut pool, 100);
    let bytes_before = pool.unique_len();
    let again = intern_2n(&mut pool, 100);
    assert_eq!(again, nodes, "same forms: same NodeIds (C1)");
    assert_eq!(pool.unique_len(), bytes_before, "no new key inserted");
    assert!(
        pool.metriques().partages >= 100,
        "every re-interning counted as a share"
    );
}

#[test]
fn test_c1_anchor_still_in_key_controller_out() {
    // The anchor stays in the hash-consing key (canonicity by handle);
    // the controller id is constant per pool and out of the key.
    let mut pool = pool_bdd();
    let t0 = pool.terminal(Anchor(0), false);
    let t1 = pool.terminal(Anchor(0), true);
    let form = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(0, t0), (1, t1)]),
    };
    let a = pool.intern(Anchor(0), form.clone());
    let b = pool.intern(Anchor(0), form.clone());
    let other_anchor = pool.intern(Anchor(1), form);
    assert_eq!(a, b, "same anchor, same form: same NodeId");
    assert_ne!(a, other_anchor, "distinct anchors: distinct handles");
    // A different form of the same degree: different NodeId.
    let t2 = pool.terminal(Anchor(0), true);
    let other = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(0),
            arcs: ArcRun::from_pairs([(0, t1), (1, t2)]),
        },
    );
    assert_ne!(a, other);
}

#[test]
fn test_gc_sweeps_the_unique_table() {
    let mut pool = pool_bdd();
    let nodes = intern_2n(&mut pool, 50);
    let len_before = pool.unique_len();
    assert_eq!(
        len_before, 50,
        "one key per node (terminals are singletons)"
    );
    // Keep 40 of the 50 nodes: 10 dead < 52/4, below the compaction
    // threshold — tombstones keep their slots, no index recycled.
    let stats = pool.gc(&nodes[..40]);
    assert!(!stats.compaction);
    assert_eq!(stats.live, 42, "40 nodes + 2 terminals");
    assert_eq!(pool.unique_len(), 40, "dead keys swept from the table");
    // Re-interning a collected form creates a fresh key and a fresh index
    // (no stale alias below the compaction threshold).
    let recreated = intern_2n(&mut pool, 50);
    assert_eq!(pool.unique_len(), 50);
    assert!(pool.resolve(recreated[49]).is_ok());
    assert_ne!(
        recreated[49], nodes[49],
        "fresh index for the recreated form"
    );
    assert!(pool.resolve(nodes[49]).is_err(), "stale id is a tombstone");
}
