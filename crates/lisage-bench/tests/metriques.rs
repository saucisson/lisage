//! Métriques du pool bi-plan (M4-1, énoncé C5) : compteurs de cache
//! (créations/partages/taux de hit) et de GC (vivants, morts, compactions)
//! exposés par `MetriquesPool`, mesurés sur un scénario déterministe
//! construit via les helpers de schéma.

use lisage_algebra::interface::{Anchor, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::biplan::seuil_atteint;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_lang::value::{Atom, Value};

/// Taille du domaine d'atomes du schéma de test.
const TAILLE_DOMAINE: u64 = 4;
/// Ancre de test.
const ANCRE_TEST: Anchor = Anchor(0);
/// Atomes du premier ensemble compilé (hit de cache ensuite).
const ENSEMBLE_PARTAGE: [u64; 2] = [0, 1];
/// Atomes de l'ensemble racine du scénario GC.
const ENSEMBLE_RACINE: [u64; 3] = [0, 1, 2];
/// Nombre d'ensembles orphelins du scénario GC.
const NB_ORPHELINS: u64 = 8;
/// Dernier atome du domaine, présent dans chaque orphelin.
const ATOME_FIXE_ORPHELIN: u64 = 3;

type PoolRef = Arena<ReferenceController>;

/// Schéma `Set` sur le domaine `{0, 1, 2, 3}`.
fn schema_test() -> Schema {
    Schema::Set(Domaine::nouveau((0..TAILLE_DOMAINE).map(Atom).collect()))
}

/// Construit un pool neuf pour le schéma donné.
fn nouveau_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compile un ensemble d'atomes dans le pool.
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

/// M4-1 : les métriques de cache du pool sont alimentées par le
/// hash-consing (créations à la première internisation, partages aux
/// internisations suivantes de la même forme) et le taux de hit est
/// cohérent avec les compteurs.
#[test]
fn test_metriques_cache_hash_consing() {
    let schema = schema_test();
    let mut pool = nouveau_pool(&schema);
    let premier = compiler_ensemble(&mut pool, ANCRE_TEST, &schema, &ENSEMBLE_PARTAGE);
    let metriques = pool.metriques();
    let creations_apres_premier = metriques.creations;
    assert!(creations_apres_premier > 0, "créations comptées");
    assert_eq!(metriques.partages, 0, "pas encore de partage");

    // Recompiler le même ensemble : R-share, aucun nouveau nœud créé.
    let partage = compiler_ensemble(&mut pool, ANCRE_TEST, &schema, &ENSEMBLE_PARTAGE);
    assert_eq!(partage, premier, "même forme = même handle");
    let metriques = pool.metriques();
    assert_eq!(
        metriques.creations, creations_apres_premier,
        "hit : pas de création"
    );
    assert!(metriques.partages > 0, "partages comptés");

    // Taux de hit en pourcentage : partages * 100 / (créations + partages).
    let taux = metriques.taux_hit();
    let attendu = metriques.partages * 100 / (metriques.creations + metriques.partages);
    assert_eq!(taux, attendu);
    assert!(taux > 0 && taux < 100, "taux de hit: {taux}");
}

/// M4-1 : le GC alimente les métriques (vivants, morts, gc, compactions) de
/// façon cohérente avec le seuil profilé, sur un scénario où une racine
/// partage des sous-nœuds avec des orphelins.
#[test]
fn test_metriques_gc_et_compaction() {
    let schema = schema_test();
    let mut pool = nouveau_pool(&schema);
    let racine = compiler_ensemble(&mut pool, ANCRE_TEST, &schema, &ENSEMBLE_RACINE);
    // Orphelins : beaucoup de formes non atteintes depuis la racine.
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
    assert!(stats.collectes > 0, "des orphelins sont collectés");
    assert_eq!(stats.vivants + stats.collectes, total);

    let metriques = pool.metriques();
    assert_eq!(metriques.gc, 1, "un seul GC effectué");
    assert_eq!(metriques.vivants, stats.vivants);
    assert_eq!(metriques.morts, stats.collectes);
    assert_eq!(
        metriques.compactions,
        if seuil_atteint(stats.collectes, total) {
            1
        } else {
            0
        },
        "compaction profilée cohérente avec le seuil"
    );
    assert_eq!(metriques.compactions, if stats.compaction { 1 } else { 0 });
    assert_eq!(pool.vivants(), stats.vivants as usize);
}
