//! Metrics of the two-plane pool (M4-1, statement C5): cache counters
//! (creations/sharings/hit rate) and GC counters (live, dead, compactions)
//! exposed by `MetricsPool`, measured on a deterministic scenario built
//! via the schema helpers.

use lisage_algebra::interface::{Anchor, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::biplan::threshold_reached;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiled};
use lisage_lang::value::{Atom, Value};

/// Size of the atom domain of the test schema.
const SIZE_DOMAINE: u64 = 4;
/// Test anchor.
const ANCRE_TEST: Anchor = Anchor(0);
/// Atoms of the first compiled set (cache hit afterwards).
const SET_PARTAGE: [u64; 2] = [0, 1];
/// Atoms of the root set of the GC scenario.
const SET_ROOT: [u64; 3] = [0, 1, 2];
/// Number of orphan sets of the GC scenario.
const NB_ORPHELINS: u64 = 8;
/// Last atom of the domain, present in each orphan.
const ATOME_FIXE_ORPHELIN: u64 = 3;

type PoolRef = Arena<ReferenceController>;

/// `Set` schema over the domain `{0, 1, 2, 3}`.
fn schema_test() -> Schema {
    Schema::Set(Domain::new((0..SIZE_DOMAINE).map(Atom).collect()))
}

/// Builds a fresh pool for the given schema.
fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms into the pool.
fn compile_set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> NodeId {
    compiled(
        pool,
        anchor,
        schema,
        &Value::Set(atoms.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M4-1: the pool's cache metrics are fed by hash-consing (creations at
/// the first interning, sharings at the following internings of the same
/// form) and the hit rate is consistent with the counters.
#[test]
fn test_metrics_cache_hash_consing() {
    let schema = schema_test();
    let mut pool = new_pool(&schema);
    let first = compile_set(&mut pool, ANCRE_TEST, &schema, &SET_PARTAGE);
    let metrics = pool.metrics();
    let creations_after_first = metrics.creations;
    assert!(creations_after_first > 0, "creations counted");
    assert_eq!(metrics.shared, 0, "no sharing yet");

    // Recompile the same set: R-share, no new node created.
    let shared = compile_set(&mut pool, ANCRE_TEST, &schema, &SET_PARTAGE);
    assert_eq!(shared, first, "same form = same handle");
    let metrics = pool.metrics();
    assert_eq!(metrics.creations, creations_after_first, "hit: no creation");
    assert!(metrics.shared > 0, "sharings counted");

    // Hit rate in percent: sharings * 100 / (creations + sharings).
    let rate = metrics.rate_hit();
    let expected = metrics.shared * 100 / (metrics.creations + metrics.shared);
    assert_eq!(rate, expected);
    assert!(rate > 0 && rate < 100, "hit rate: {rate}");
}

/// M4-1: the GC feeds the metrics (live, dead, gc, compactions)
/// consistently with the profiled threshold, on a scenario where a root
/// shares sub-nodes with orphans.
#[test]
fn test_metrics_gc_and_compaction() {
    let schema = schema_test();
    let mut pool = new_pool(&schema);
    let root = compile_set(&mut pool, ANCRE_TEST, &schema, &SET_ROOT);
    // Orphans: many forms not reachable from the root.
    for i in 0..NB_ORPHELINS {
        compile_set(
            &mut pool,
            ANCRE_TEST,
            &schema,
            &[i % SIZE_DOMAINE, ATOME_FIXE_ORPHELIN],
        );
    }
    let total = pool.node_count() as u64;
    let stats = pool.gc(&[root]);
    assert!(stats.collected > 0, "orphans are collected");
    assert_eq!(stats.live + stats.collected, total);

    let metrics = pool.metrics();
    assert_eq!(metrics.gc, 1, "a single GC was run");
    assert_eq!(metrics.live, stats.live);
    assert_eq!(metrics.dead, stats.collected);
    assert_eq!(
        metrics.compactions,
        if threshold_reached(stats.collected, total) {
            1
        } else {
            0
        },
        "profiled compaction consistent with the threshold"
    );
    assert_eq!(metrics.compactions, if stats.compaction { 1 } else { 0 });
    assert_eq!(pool.live(), stats.live as usize);
}
