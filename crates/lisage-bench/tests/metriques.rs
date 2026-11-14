//! Bi-plan pool metrics (M4-1, statement C5): cache counters
//! (creations/shares/hit rate) and GC counters (live, dead, compactions)
//! exposed by `MetriquesPool`, measured on a deterministic scenario
//! built via the schema helpers.

use lisage_algebra::interface::{Anchor, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::biplan::seuil_atteint;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_lang::value::{Atom, Value};

/// Size of the atom domain of the test schema.
const TAILLE_DOMAINE: u64 = 4;
/// Test anchor.
const ANCRE_TEST: Anchor = Anchor(0);
/// Atoms of the first compiled set (cache hit afterwards).
const ENSEMBLE_PARTAGE: [u64; 2] = [0, 1];
/// Atoms of the root set of the GC scenario.
const ENSEMBLE_RACINE: [u64; 3] = [0, 1, 2];
/// Number of orphan sets in the GC scenario.
const NB_ORPHELINS: u64 = 8;
/// Last atom of the domain, present in each orphan.
const ATOME_FIXE_ORPHELIN: u64 = 3;

type PoolRef = Arena<ReferenceController>;

/// `Set` schema over the domain `{0, 1, 2, 3}`.
fn schema_test() -> Schema {
    Schema::Set(Domaine::nouveau((0..TAILLE_DOMAINE).map(Atom).collect()))
}

/// Builds a fresh pool for the given schema.
fn nouveau_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms into the pool.
fn compiler_ensemble(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    atomes: &[u64],
) -> NodeId {
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
    let mut pool = nouveau_pool(&schema);
    let premier = compiler_ensemble(&mut pool, ANCRE_TEST, &schema, &ENSEMBLE_PARTAGE);
    let metriques = pool.metriques();
    let creations_apres_premier = metriques.creations;
    assert!(creations_apres_premier > 0, "creations counted");
    assert_eq!(metriques.partages, 0, "no share yet");

    // Recompile the same set: R-share, no new node created.
    let partage = compiler_ensemble(&mut pool, ANCRE_TEST, &schema, &ENSEMBLE_PARTAGE);
    assert_eq!(partage, premier, "same form = same handle");
    let metriques = pool.metriques();
    assert_eq!(
        metriques.creations, creations_apres_premier,
        "hit: no creation"
    );
    assert!(metriques.partages > 0, "shares counted");

    // Hit rate as a percentage: shares * 100 / (creations + shares).
    let taux = metriques.taux_hit();
    let attendu = metriques.partages * 100 / (metriques.creations + metriques.partages);
    assert_eq!(taux, attendu);
    assert!(taux > 0 && taux < 100, "hit rate: {taux}");
}

/// M4-1: the GC feeds the metrics (live, dead, gc, compactions) in a
/// way consistent with the profiled threshold, on a scenario where a root
/// shares sub-nodes with orphans.
#[test]
fn test_metriques_gc_et_compaction() {
    let schema = schema_test();
    let mut pool = nouveau_pool(&schema);
    let racine = compiler_ensemble(&mut pool, ANCRE_TEST, &schema, &ENSEMBLE_RACINE);
    // Orphans: many forms not reachable from the root.
    for i in 0..NB_ORPHELINS {
        compiler_ensemble(
            &mut pool,
            ANCRE_TEST,
            &schema,
            &[i % TAILLE_DOMAINE, ATOME_FIXE_ORPHELIN],
        );
    }
    let total = pool.node_count() as u64;
    let stats = pool.gc(&[racine]);
    assert!(stats.collectes > 0, "orphans are collected");
    assert_eq!(stats.vivants + stats.collectes, total);

    let metriques = pool.metriques();
    assert_eq!(metriques.gc, 1, "a single GC performed");
    assert_eq!(metriques.vivants, stats.vivants);
    assert_eq!(metriques.morts, stats.collectes);
    assert_eq!(
        metriques.compactions,
        if seuil_atteint(stats.collectes, total) {
            1
        } else {
            0
        },
        "profiled compaction consistent with the threshold"
    );
    assert_eq!(metriques.compactions, if stats.compaction { 1 } else { 0 });
    assert_eq!(pool.vivants(), stats.vivants as usize);
}
