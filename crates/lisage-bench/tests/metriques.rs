//! Bi-plan pool metrics (M4-1, statement C5): cache counters
//! (creations/shares/hit rate) and GC counters (live, dead, compactions)
//! exposed by `MetriquesPool`, measured on a deterministic scenario
//! built via the schema helpers.

use lisage_algebra::interface::{Anchor, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::biplan::threshold_atteint;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiler};
use lisage_lang::value::{Atom, Value};

/// Size of the atom domain of the test schema.
const SIZE_DOMAIN: u64 = 4;
/// Test anchor.
const ANCHOR_TEST: Anchor = Anchor(0);
/// Atoms of the first compiled set (cache hit afterwards).
const SET_PARTAGE: [u64; 2] = [0, 1];
/// Atoms of the root set of the GC scenario.
const SET_ROOT: [u64; 3] = [0, 1, 2];
/// Number of orphan sets in the GC scenario.
const NB_ORPHANS: u64 = 8;
/// Last atom of the domain, present in each orphan.
const ATOME_FIXE_ORPHAN: u64 = 3;

type PoolRef = Arena<ReferenceController>;

/// `Set` schema over the domain `{0, 1, 2, 3}`.
fn schema_test() -> Schema {
    Schema::Set(Domain::new((0..SIZE_DOMAIN).map(Atom).collect()))
}

/// Builds a fresh pool for the given schema.
fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms into the pool.
fn compiler_set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atomes: &[u64]) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atomes.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M4-1: the pool's cache metrics are fed by the
/// hash-consing (creations at the first interning, shares at the
/// following internings of the same form) and the hit rate is
/// consistent with the counters.
#[test]
fn test_metriques_cache_hash_consing() {
    let schema = schema_test();
    let mut pool = new_pool(&schema);
    let first = compiler_set(&mut pool, ANCHOR_TEST, &schema, &SET_PARTAGE);
    let metriques = pool.metriques();
    let creations_apres_first = metriques.creations;
    assert!(creations_apres_first > 0, "creations counted");
    assert_eq!(metriques.partages, 0, "no share yet");

    // Recompile the same set: R-share, no new node created.
    let partage = compiler_set(&mut pool, ANCHOR_TEST, &schema, &SET_PARTAGE);
    assert_eq!(partage, first, "same form = same handle");
    let metriques = pool.metriques();
    assert_eq!(
        metriques.creations, creations_apres_first,
        "hit: no creation"
    );
    assert!(metriques.partages > 0, "shares counted");

    // Hit rate as a percentage: shares * 100 / (creations + shares).
    let taux = metriques.taux_hit();
    let expected = metriques.partages * 100 / (metriques.creations + metriques.partages);
    assert_eq!(taux, expected);
    assert!(taux > 0 && taux < 100, "hit rate: {taux}");
}

/// M4-1: the GC feeds the metrics (live, dead, gc, compactions) in a
/// way consistent with the profiled threshold, on a scenario where a root
/// shares sub-nodes with orphans.
#[test]
fn test_metriques_gc_et_compaction() {
    let schema = schema_test();
    let mut pool = new_pool(&schema);
    let root = compiler_set(&mut pool, ANCHOR_TEST, &schema, &SET_ROOT);
    // Orphans: many forms not reachable from the root.
    for i in 0..NB_ORPHANS {
        compiler_set(
            &mut pool,
            ANCHOR_TEST,
            &schema,
            &[i % SIZE_DOMAIN, ATOME_FIXE_ORPHAN],
        );
    }
    let total = pool.node_count() as u64;
    let stats = pool.gc(&[root]);
    assert!(stats.collected > 0, "orphans are collected");
    assert_eq!(stats.live + stats.collected, total);

    let metriques = pool.metriques();
    assert_eq!(metriques.gc, 1, "a single GC performed");
    assert_eq!(metriques.live, stats.live);
    assert_eq!(metriques.dead, stats.collected);
    assert_eq!(
        metriques.compactions,
        if threshold_atteint(stats.collected, total) {
            1
        } else {
            0
        },
        "profiled compaction consistent with the threshold"
    );
    assert_eq!(metriques.compactions, if stats.compaction { 1 } else { 0 });
    assert_eq!(pool.live(), stats.live as usize);
}
