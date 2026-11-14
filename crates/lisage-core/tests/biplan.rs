//! Biplane pool tests (M4-1, statement C5): main/shadow planes, level → forms
//! index in the shadow plane, mark-and-sweep GC, profiled compaction (dead
//! threshold), cache metrics, `Gc` event.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_core::arena::Arena;
use lisage_core::biplan::{Plan, SEUIL_COMPACTION_DEN, SEUIL_COMPACTION_NUM, seuil_atteint};
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};

/// Test anchor (arbitrary but named value).
const ANCRE_TEST: Anchor = Anchor(0);
/// Level of the root node of the small diagram.
const NIVEAU_RACINE: u32 = 1;
/// Level of the orphan node of the small diagram.
const NIVEAU_ORPHELIN: u32 = 2;
/// Number of orphans added to exceed the compaction threshold.
const NB_ORPHELINS: u64 = 20;
/// Base level of the orphans in the profiled compaction.
const NIVEAU_BASE_ORPHELINS: u32 = 10;
/// Level of the node interned in the shadow plane.
const NIVEAU_OMBRE: u32 = 3;
/// Level absent from the index (empty-set test).
const NIVEAU_ABSENT: u32 = 9;

/// LCG seed for property tests (same constant as the other suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// LCG increment for property tests.
const INCREMENT_LCG: u64 = 1442695040888963407;

struct MockKeys;

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        2
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(0)
    }
}

struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(7);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

/// Recording observer.
#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<TraceEvent>>,
}

impl Observer for RecordingObserver {
    fn on_event(&self, event: &TraceEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

fn arena() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

/// Builds a small diagram: a level-1 node with two arcs to two terminals,
/// plus an orphan level-2 node.
fn petit_diagramme(pool: &mut Arena<MockController>) -> (NodeId, NodeId) {
    let faux = pool.terminal(ANCRE_TEST, false);
    let vrai = pool.terminal(ANCRE_TEST, true);
    let racine = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_RACINE),
            arcs: BTreeMap::from([(0, faux), (1, vrai)]),
        },
    );
    let orphelin = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_ORPHELIN),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    (racine, orphelin)
}

/// M4-1 : plans principal/ombre et index du plan d'ombre (niveau → formes).
#[test]
fn test_plans_et_index_ombre() {
    let mut pool = arena();
    let (racine, orphelin) = petit_diagramme(&mut pool);
    // By default, everything is in the main plane.
    assert_eq!(pool.plan_de(racine), Some(Plan::Principal));
    // A node interned in the shadow plane stays there.
    let ombre = pool.intern_plan(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_OMBRE),
            arcs: BTreeMap::from([(1, racine)]),
        },
        Plan::Ombre,
    );
    assert_eq!(pool.plan_de(ombre), Some(Plan::Ombre));
    assert_eq!(pool.plan_de(orphelin), Some(Plan::Principal));
    // Shadow plane index: level 1 contains the root.
    assert!(pool.index_niveau(Level(NIVEAU_RACINE)).contains(&racine));
    assert!(pool.index_niveau(Level(NIVEAU_OMBRE)).contains(&ombre));
    assert!(pool.index_niveau(Level(NIVEAU_ABSENT)).is_empty());
}

/// M4-1: mark-and-sweep GC — only nodes reachable from the roots survive;
/// tombstones are removed from the unique table and the index, and `resolve`
/// rejects them.
#[test]
fn test_gc_mark_sweep() {
    let mut pool = arena();
    let (racine, orphelin) = petit_diagramme(&mut pool);
    let avant = pool.node_count();
    let stats = pool.gc(&[racine]);
    // R-default removed the default arc: reachable = root + true.
    assert_eq!(stats.vivants, 2, "root + true terminal");
    assert_eq!(stats.collectes, (avant as u64) - 2);
    // Compaction depends on the profiled threshold applied to the observed dead.
    assert_eq!(
        stats.compaction,
        seuil_atteint(stats.collectes, avant as u64)
    );
    assert_eq!(pool.vivants(), 2);
    assert_eq!(
        pool.resolve(orphelin),
        Err(PoolError::UnknownNode(orphelin)),
        "collected node = tombstone"
    );
    assert!(
        !pool
            .index_niveau(Level(NIVEAU_ORPHELIN))
            .contains(&orphelin)
    );
    // Re-interning the orphan's form recreates a node (cache miss).
    let vrai = pool.terminal(ANCRE_TEST, true);
    let faux = pool.terminal(ANCRE_TEST, false);
    let recree = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_ORPHELIN),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    // After compaction the ids are recycled: the assertion is about the
    // validity of the new node, not the value of its id.
    assert!(pool.resolve(recree).is_ok());
    assert_eq!(pool.plan_de(recree), Some(Plan::Principal));
}

/// M4-1: profiled compaction — when dead × 4 ≥ total, the GC compacts and
/// remaps the `NodeId`s; live handles stay valid.
#[test]
fn test_compaction_profilee() {
    let mut pool = arena();
    let (racine, _orphelin) = petit_diagramme(&mut pool);
    // Many orphans to exceed the SEUIL_COMPACTION threshold.
    let mut orphelins = Vec::new();
    for i in 0..NB_ORPHELINS {
        orphelins.push(pool.intern(
            ANCRE_TEST,
            Form::Node {
                level: Level(NIVEAU_BASE_ORPHELINS + i as u32),
                arcs: BTreeMap::from([(1, racine)]),
            },
        ));
    }
    let total = pool.node_count() as u64;
    assert!(
        seuil_atteint(NB_ORPHELINS, total),
        "compaction threshold reached"
    );
    let stats = pool.gc(&[racine]);
    assert!(stats.compaction, "profiled compaction triggered");
    assert_eq!(pool.vivants(), 2);
    assert_eq!(pool.node_count(), 2, "tombstones compacted");
    // Compaction invalidates the old NodeIds: the live root is remapped to a
    // new id, resolvable and indexed at level 1.
    let remap = pool.dernier_remap();
    let nouveau = remap[racine.0 as usize].expect("live root remapped");
    assert_ne!(nouveau, racine);
    assert!(pool.resolve(nouveau).is_ok());
    assert!(pool.index_niveau(Level(NIVEAU_RACINE)).contains(&nouveau));
    // The threshold is not reached with few dead.
    assert!(!seuil_atteint(1, 100));
    assert_eq!(SEUIL_COMPACTION_NUM, 1);
    assert_eq!(SEUIL_COMPACTION_DEN, 4);
}

/// M4-1: cache metrics — hash-consing creations/shares, hit rate, GC
/// counters — and the `Gc` event emitted to the observer.
#[test]
fn test_metriques_cache_et_evenement_gc() {
    let observateur = Arc::new(RecordingObserver::default());
    struct Pont(Arc<RecordingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    let mut pool = Arena::new(
        MockController { keys: MockKeys },
        Box::new(Pont(observateur.clone())),
    );
    let (racine, orphelin) = petit_diagramme(&mut pool);
    // Re-intern the exact form of the root: hit (R-share, same handle).
    let Form::Node { level, arcs } = pool.resolve(racine).unwrap().clone() else {
        panic!("the root is a node");
    };
    let partage = pool.intern(ANCRE_TEST, Form::Node { level, arcs });
    assert_eq!(partage, racine, "R-share: same handle");
    let metriques = pool.metriques();
    assert!(
        metriques.creations >= 4,
        "creations counted: {}",
        metriques.creations
    );
    assert!(
        metriques.partages >= 1,
        "shares counted: {}",
        metriques.partages
    );
    assert!(pool.metriques().taux_hit() > 0);
    let stats = pool.gc(&[racine]);
    let metriques = pool.metriques();
    assert_eq!(metriques.gc, 1);
    assert_eq!(metriques.vivants, stats.vivants);
    assert_eq!(metriques.morts, stats.collectes);
    assert_eq!(metriques.compactions, if stats.compaction { 1 } else { 0 });
    // Gc event emitted.
    let events = observateur.events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Gc {
                vivants,
                collectes,
                ..
            } if *vivants == stats.vivants && *collectes == stats.collectes
        )),
        "Gc event emitted"
    );
    let _ = orphelin;
}

/// M4-1: `intern_plan` on a hash-consing hit re-plans the shared node (the
/// form already exists: the requested plane overwrites the current plane of
/// the returned handle).
#[test]
fn test_intern_plan_hit_cache_replanifie() {
    let mut pool = arena();
    let (racine, _orphelin) = petit_diagramme(&mut pool);
    assert_eq!(pool.plan_de(racine), Some(Plan::Principal));
    let Form::Node { level, arcs } = pool.resolve(racine).unwrap().clone() else {
        panic!("the root is a node");
    };
    // Cache hit with shadow plane: same handle, plane moved.
    let meme = pool.intern_plan(ANCRE_TEST, Form::Node { level, arcs }, Plan::Ombre);
    assert_eq!(meme, racine, "R-share: same handle");
    assert_eq!(pool.plan_de(racine), Some(Plan::Ombre));
}

/// M4-1 (property tier, LCG): invariants of the compaction threshold, the hit
/// rate and the GC — bound `dead × 4 == total` included, `taux_hit` bounded,
/// `vivants + collectes == total` over random scenarios.
#[test]
fn test_propriete_seuil_taux_hit_et_invariant_gc() {
    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    // Threshold: the bound is included (dead × DEN == total × NUM → reached),
    // and one dead less does not reach it (for total > 0).
    for _ in 0..1000 {
        let total = 1 + suivant() % 1000;
        let morts = suivant() % (total + 1);
        assert_eq!(
            seuil_atteint(morts, total),
            morts * SEUIL_COMPACTION_DEN >= total * SEUIL_COMPACTION_NUM
        );
        assert!(!seuil_atteint(0, 0), "zero total: no compaction");
    }
    // Exact bound: the smallest `morts` reaching the threshold is
    // `ceil(total × NUM / DEN)`; one dead less does not reach it.
    for total in 1..=64u64 {
        let seuil = (total * SEUIL_COMPACTION_NUM).div_ceil(SEUIL_COMPACTION_DEN);
        assert!(seuil_atteint(seuil, total), "bound reached: total={total}");
        assert!(
            !seuil_atteint(seuil - 1, total),
            "one dead below the bound: total={total}"
        );
    }

    // Hit rate: 0 without activity, otherwise a percentage bounded in [0, 100].
    for _ in 0..1000 {
        let creations = suivant() % 500;
        let partages = suivant() % 500;
        let metriques = lisage_core::biplan::MetriquesPool {
            creations,
            partages,
            ..Default::default()
        };
        let taux = metriques.taux_hit();
        assert!(taux <= 100);
        let attendu = partages
            .checked_mul(lisage_core::biplan::BASE_POURCENTAGE)
            .and_then(|p| p.checked_div(creations + partages))
            .unwrap_or(0);
        assert_eq!(taux, attendu);
    }

    // GC invariant: vivants + collectes == total, and the metrics reflect
    // the last GC.
    for _ in 0..100 {
        let mut pool = arena();
        let (racine, _orphelin) = petit_diagramme(&mut pool);
        let nb_orphelins = suivant() % 30;
        for i in 0..nb_orphelins {
            pool.intern(
                ANCRE_TEST,
                Form::Node {
                    level: Level(NIVEAU_BASE_ORPHELINS + i as u32),
                    arcs: BTreeMap::from([(1, racine)]),
                },
            );
        }
        let total = pool.node_count() as u64;
        let stats = pool.gc(&[racine]);
        assert_eq!(stats.vivants + stats.collectes, total);
        assert_eq!(stats.vivants, pool.vivants() as u64);
        let metriques = pool.metriques();
        assert_eq!(metriques.vivants, stats.vivants);
        assert_eq!(metriques.morts, stats.collectes);
        assert_eq!(metriques.compactions, if stats.compaction { 1 } else { 0 });
    }
}

/// M4-1 (review regression): a second GC on the same pool without intermediate
/// compaction counts the live ones over the live entries, not over the number
/// of slots (which includes the tombstones of the first GC).
#[test]
fn test_gc_deux_fois_vivants_exacts() {
    let mut pool = arena();
    let (racine, _orphelin) = petit_diagramme(&mut pool);
    // Chain of nodes all reachable from the root: the first GC only collects
    // the orphan and the false terminal (2 dead out of a large total), the
    // threshold is not reached, the tombstones remain.
    let mut courant = racine;
    for i in 0..NB_ORPHELINS {
        courant = pool.intern(
            ANCRE_TEST,
            Form::Node {
                level: Level(NIVEAU_BASE_ORPHELINS + i as u32 + 100),
                arcs: BTreeMap::from([(1, courant)]),
            },
        );
    }
    // Roots = root + chain end: the whole chain is reached, only the orphan
    // and the false terminal are collected.
    let premier = pool.gc(&[racine, courant]);
    assert!(!premier.compaction, "few dead: no compaction");
    assert!(pool.node_count() > pool.vivants(), "tombstones kept");
    // Second GC: the live count must not include the tombstones of the first
    // GC — vivants + collectes == live entries before the GC.
    let vivantes_avant = pool.vivants() as u64;
    let stats = pool.gc(&[racine, courant]);
    assert_eq!(stats.vivants, pool.vivants() as u64);
    assert_eq!(stats.vivants + stats.collectes, vivantes_avant);
    let metriques = pool.metriques();
    assert_eq!(metriques.vivants, stats.vivants);
    assert_eq!(metriques.gc, 2);
}
