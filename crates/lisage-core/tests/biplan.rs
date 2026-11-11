//! Tests du pool bi-plan (M4-1, énoncé C5) : plans principal/ombre, index
//! niveau → formes dans le plan d'ombre, GC mark-and-sweep, compaction
//! profilée (seuil de morts), métriques de cache, événement `Gc`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_core::arena::Arena;
use lisage_core::biplan::{Plan, SEUIL_COMPACTION_DEN, SEUIL_COMPACTION_NUM, seuil_atteint};
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};

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

/// Observateur enregistreur.
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

/// Construit un petit diagramme : nœud niveau 1 avec deux arcs vers deux
/// terminaux, plus un nœud orphelin niveau 2.
fn petit_diagramme(pool: &mut Arena<MockController>) -> (NodeId, NodeId) {
    let faux = pool.terminal(Anchor(0), false);
    let vrai = pool.terminal(Anchor(0), true);
    let racine = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, faux), (1, vrai)]),
        },
    );
    let orphelin = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(2),
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
    // Par défaut, tout est dans le plan principal.
    assert_eq!(pool.plan_de(racine), Some(Plan::Principal));
    // Un nœud interné dans le plan d'ombre y reste.
    let ombre = pool.intern_plan(
        Anchor(0),
        Form::Node {
            level: Level(3),
            arcs: BTreeMap::from([(1, racine)]),
        },
        Plan::Ombre,
    );
    assert_eq!(pool.plan_de(ombre), Some(Plan::Ombre));
    assert_eq!(pool.plan_de(orphelin), Some(Plan::Principal));
    // Index du plan d'ombre : niveau 1 contient la racine.
    assert!(pool.index_niveau(Level(1)).contains(&racine));
    assert!(pool.index_niveau(Level(3)).contains(&ombre));
    assert!(pool.index_niveau(Level(9)).is_empty());
}

/// M4-1 : GC mark-and-sweep — seuls les nœuds atteints depuis les racines
/// survivent ; les tombstones sont retirées de la table unique et de
/// l'index, et `resolve` les rejette.
#[test]
fn test_gc_mark_sweep() {
    let mut pool = arena();
    let (racine, orphelin) = petit_diagramme(&mut pool);
    let avant = pool.node_count();
    let stats = pool.gc(&[racine]);
    // R-default a retiré l'arc par défaut : atteignables = racine + vrai.
    assert_eq!(stats.vivants, 2, "racine + terminal vrai");
    assert_eq!(stats.collectes, (avant as u64) - 2);
    // La compaction dépend du seuil profilé appliqué aux morts constatés.
    assert_eq!(
        stats.compaction,
        seuil_atteint(stats.collectes, avant as u64)
    );
    assert_eq!(pool.vivants(), 2);
    assert_eq!(
        pool.resolve(orphelin),
        Err(PoolError::UnknownNode(orphelin)),
        "nœud collecté = tombstone"
    );
    assert!(!pool.index_niveau(Level(2)).contains(&orphelin));
    // Ré-interner la forme de l'orphelin recrée un nœud (miss du cache).
    let vrai = pool.terminal(Anchor(0), true);
    let faux = pool.terminal(Anchor(0), false);
    let recree = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(2),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    // Après compaction les ids sont recyclés : l'assertion porte sur la
    // validité du nouveau nœud, pas sur la valeur de son id.
    assert!(pool.resolve(recree).is_ok());
    assert_eq!(pool.plan_de(recree), Some(Plan::Principal));
}

/// M4-1 : compaction profilée — quand morts × 4 ≥ total, le GC compacte et
/// remappe les `NodeId` ; les handles vivants restent valides.
#[test]
fn test_compaction_profilee() {
    let mut pool = arena();
    let (racine, _orphelin) = petit_diagramme(&mut pool);
    // Beaucoup d'orphelins pour dépasser le seuil SEUIL_COMPACTION.
    let mut orphelins = Vec::new();
    for i in 0..20u64 {
        orphelins.push(pool.intern(
            Anchor(0),
            Form::Node {
                level: Level(10 + i as u32),
                arcs: BTreeMap::from([(1, racine)]),
            },
        ));
    }
    let total = pool.node_count() as u64;
    assert!(seuil_atteint(20, total), "seuil de compaction atteint");
    let stats = pool.gc(&[racine]);
    assert!(stats.compaction, "compaction profilée déclenchée");
    assert_eq!(pool.vivants(), 2);
    assert_eq!(pool.node_count(), 2, "tombstones compactées");
    // La compaction invalide les anciens NodeId : la racine vivante est
    // remappée vers un nouvel id, résolvable et indexé au niveau 1.
    let remap = pool.dernier_remap();
    let nouveau = remap[racine.0 as usize].expect("racine vivante remappée");
    assert_ne!(nouveau, racine);
    assert!(pool.resolve(nouveau).is_ok());
    assert!(pool.index_niveau(Level(1)).contains(&nouveau));
    // Le seuil n'est pas atteint avec peu de morts.
    assert!(!seuil_atteint(1, 100));
    assert_eq!(SEUIL_COMPACTION_NUM, 1);
    assert_eq!(SEUIL_COMPACTION_DEN, 4);
}

/// M4-1 : métriques de cache — créations/partages du hash-consing, taux de
/// hit, compteurs de GC — et événement `Gc` émis vers l'observateur.
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
    // Re-interner la forme exacte de la racine : hit (R-share, même handle).
    let Form::Node { level, arcs } = pool.resolve(racine).unwrap().clone() else {
        panic!("la racine est un nœud");
    };
    let partage = pool.intern(Anchor(0), Form::Node { level, arcs });
    assert_eq!(partage, racine, "R-share : même handle");
    let m = pool.metriques();
    assert!(m.creations >= 4, "créations comptées: {}", m.creations);
    assert!(m.partages >= 1, "partages comptés: {}", m.partages);
    assert!(pool.metriques().taux_hit() > 0);
    let stats = pool.gc(&[racine]);
    let m = pool.metriques();
    assert_eq!(m.gc, 1);
    assert_eq!(m.vivants, stats.vivants);
    assert_eq!(m.morts, stats.collectes);
    assert_eq!(m.compactions, if stats.compaction { 1 } else { 0 });
    // Événement Gc émis.
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
        "événement Gc émis"
    );
    let _ = orphelin;
}
