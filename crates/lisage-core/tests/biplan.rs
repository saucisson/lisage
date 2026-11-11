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

/// Ancre de test (valeur arbitraire mais nommée).
const ANCRE_TEST: Anchor = Anchor(0);
/// Niveau du nœud racine du petit diagramme.
const NIVEAU_RACINE: u32 = 1;
/// Niveau du nœud orphelin du petit diagramme.
const NIVEAU_ORPHELIN: u32 = 2;
/// Nombre d'orphelins ajoutés pour dépasser le seuil de compaction.
const NB_ORPHELINS: u64 = 20;
/// Niveau de base des orphelins de la compaction profilée.
const NIVEAU_BASE_ORPHELINS: u32 = 10;
/// Niveau du nœud interné dans le plan d'ombre.
const NIVEAU_OMBRE: u32 = 3;
/// Niveau absent de l'index (test de l'ensemble vide).
const NIVEAU_ABSENT: u32 = 9;

/// Graine du LCG des tests de propriété (même constante que les autres suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplicateur du LCG des tests de propriété.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// Incrément du LCG des tests de propriété.
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
    // Par défaut, tout est dans le plan principal.
    assert_eq!(pool.plan_de(racine), Some(Plan::Principal));
    // Un nœud interné dans le plan d'ombre y reste.
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
    // Index du plan d'ombre : niveau 1 contient la racine.
    assert!(pool.index_niveau(Level(NIVEAU_RACINE)).contains(&racine));
    assert!(pool.index_niveau(Level(NIVEAU_OMBRE)).contains(&ombre));
    assert!(pool.index_niveau(Level(NIVEAU_ABSENT)).is_empty());
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
    assert!(
        !pool
            .index_niveau(Level(NIVEAU_ORPHELIN))
            .contains(&orphelin)
    );
    // Ré-interner la forme de l'orphelin recrée un nœud (miss du cache).
    let vrai = pool.terminal(ANCRE_TEST, true);
    let faux = pool.terminal(ANCRE_TEST, false);
    let recree = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_ORPHELIN),
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
        "seuil de compaction atteint"
    );
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
    assert!(pool.index_niveau(Level(NIVEAU_RACINE)).contains(&nouveau));
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
    let partage = pool.intern(ANCRE_TEST, Form::Node { level, arcs });
    assert_eq!(partage, racine, "R-share : même handle");
    let metriques = pool.metriques();
    assert!(
        metriques.creations >= 4,
        "créations comptées: {}",
        metriques.creations
    );
    assert!(
        metriques.partages >= 1,
        "partages comptés: {}",
        metriques.partages
    );
    assert!(pool.metriques().taux_hit() > 0);
    let stats = pool.gc(&[racine]);
    let metriques = pool.metriques();
    assert_eq!(metriques.gc, 1);
    assert_eq!(metriques.vivants, stats.vivants);
    assert_eq!(metriques.morts, stats.collectes);
    assert_eq!(metriques.compactions, if stats.compaction { 1 } else { 0 });
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

/// M4-1 : `intern_plan` sur un hit du hash-consing re-planne le nœud
/// partagé (la forme existe déjà : le plan demandé écrase le plan
/// courant du handle retourné).
#[test]
fn test_intern_plan_hit_cache_replanifie() {
    let mut pool = arena();
    let (racine, _orphelin) = petit_diagramme(&mut pool);
    assert_eq!(pool.plan_de(racine), Some(Plan::Principal));
    let Form::Node { level, arcs } = pool.resolve(racine).unwrap().clone() else {
        panic!("la racine est un nœud");
    };
    // Hit du cache avec plan d'ombre : même handle, plan déplacé.
    let meme = pool.intern_plan(ANCRE_TEST, Form::Node { level, arcs }, Plan::Ombre);
    assert_eq!(meme, racine, "R-share : même handle");
    assert_eq!(pool.plan_de(racine), Some(Plan::Ombre));
}

/// M4-1 (tier propriété, LCG) : invariants du seuil de compaction, du taux
/// de hit et du GC — borne `morts × 4 == total` incluse, `taux_hit` borné,
/// `vivants + collectes == total` sur scénarios aléatoires.
#[test]
fn test_propriete_seuil_taux_hit_et_invariant_gc() {
    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    // Seuil : la borne est incluse (morts × DEN == total × NUM → atteint),
    // et un mort de moins ne l'atteint pas (pour total > 0).
    for _ in 0..1000 {
        let total = 1 + suivant() % 1000;
        let morts = suivant() % (total + 1);
        assert_eq!(
            seuil_atteint(morts, total),
            morts * SEUIL_COMPACTION_DEN >= total * SEUIL_COMPACTION_NUM
        );
        assert!(!seuil_atteint(0, 0), "total nul : pas de compaction");
    }
    // Borne exacte : le plus petit `morts` atteignant le seuil est
    // `ceil(total × NUM / DEN)` ; un mort de moins ne l'atteint pas.
    for total in 1..=64u64 {
        let seuil = (total * SEUIL_COMPACTION_NUM).div_ceil(SEUIL_COMPACTION_DEN);
        assert!(seuil_atteint(seuil, total), "borne atteinte: total={total}");
        assert!(
            !seuil_atteint(seuil - 1, total),
            "un mort de moins sous la borne: total={total}"
        );
    }

    // Taux de hit : 0 sans activité, sinon pourcentage borné dans [0, 100].
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

    // Invariant du GC : vivants + collectes == total, et les métriques
    // reflètent le dernier GC.
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

/// M4-1 (régression review) : un second GC sur le même pool sans
/// compaction intermédiaire compte les vivants sur les entrées vivantes,
/// pas sur le nombre de slots (qui inclut les tombstones du premier GC).
#[test]
fn test_gc_deux_fois_vivants_exacts() {
    let mut pool = arena();
    let (racine, _orphelin) = petit_diagramme(&mut pool);
    // Chaîne de nœuds tous atteints depuis la racine : le premier GC ne
    // collecte que l'orphelin et le terminal faux (2 morts sur un grand
    // total), le seuil n'est pas atteint, les tombstones restent.
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
    // Racines = racine + extrémité de chaîne : toute la chaîne est atteinte,
    // seuls l'orphelin et le terminal faux sont collectés.
    let premier = pool.gc(&[racine, courant]);
    assert!(!premier.compaction, "peu de morts : pas de compaction");
    assert!(pool.node_count() > pool.vivants(), "tombstones conservées");
    // Second GC : les vivants ne doivent pas compter les tombstones du
    // premier GC — vivants + collectes == entrées vivantes avant le GC.
    let vivantes_avant = pool.vivants() as u64;
    let stats = pool.gc(&[racine, courant]);
    assert_eq!(stats.vivants, pool.vivants() as u64);
    assert_eq!(stats.vivants + stats.collectes, vivantes_avant);
    let metriques = pool.metriques();
    assert_eq!(metriques.vivants, stats.vivants);
    assert_eq!(metriques.gc, 2);
}
