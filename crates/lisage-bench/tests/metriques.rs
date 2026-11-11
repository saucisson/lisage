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
    let anchor = Anchor(0);

    let premier = compiler_ensemble(&mut pool, anchor, &schema, &[0, 1]);
    let m = pool.metriques();
    let creations_apres_premier = m.creations;
    assert!(creations_apres_premier > 0, "créations comptées");
    assert_eq!(m.partages, 0, "pas encore de partage");

    // Recompiler le même ensemble : R-share, aucun nouveau nœud créé.
    let partage = compiler_ensemble(&mut pool, anchor, &schema, &[0, 1]);
    assert_eq!(partage, premier, "même forme = même handle");
    let m = pool.metriques();
    assert_eq!(
        m.creations, creations_apres_premier,
        "hit : pas de création"
    );
    assert!(m.partages > 0, "partages comptés");

    // Taux de hit en pourcentage : partages * 100 / (créations + partages).
    let taux = m.taux_hit();
    let attendu = m.partages * 100 / (m.creations + m.partages);
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
    let anchor = Anchor(0);

    let racine = compiler_ensemble(&mut pool, anchor, &schema, &[0, 1, 2]);
    // Orphelins : beaucoup de formes non atteintes depuis la racine.
    for i in 0..8u64 {
        compiler_ensemble(&mut pool, anchor, &schema, &[i % TAILLE_DOMAINE, 3]);
    }
    let total = pool.node_count() as u64;
    let stats = pool.gc(&[racine]);
    assert!(stats.collectes > 0, "des orphelins sont collectés");
    assert_eq!(stats.vivants + stats.collectes, total);

    let m = pool.metriques();
    assert_eq!(m.gc, 1, "un seul GC effectué");
    assert_eq!(m.vivants, stats.vivants);
    assert_eq!(m.morts, stats.collectes);
    assert_eq!(
        m.compactions,
        if seuil_atteint(stats.collectes, total) {
            1
        } else {
            0
        },
        "compaction profilée cohérente avec le seuil"
    );
    assert_eq!(m.compactions, if stats.compaction { 1 } else { 0 });
    assert_eq!(pool.vivants(), stats.vivants as usize);
}
