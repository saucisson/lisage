//! Two-plane pool tests (M4-1, statement C5): main/shadow planes, index
//! level → forms in the shadow plane, mark-and-sweep GC, profiled
//! compaction (dead threshold), cache metrics, `Gc` event.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_core::arena::Arena;
use lisage_core::biplan::{
    Plan, THRESHOLD_COMPACTION_DEN, THRESHOLD_COMPACTION_NUM, threshold_reached,
};
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};

/// Test anchor (arbitrary but named value).
const ANCRE_TEST: Anchor = Anchor(0);
/// Level of the root node of the small diagram.
const LEVEL_ROOT: u32 = 1;
/// Level of the orphan node of the small diagram.
const LEVEL_ORPHELIN: u32 = 2;
/// Number of orphans added to exceed the compaction threshold.
const NB_ORPHELINS: u64 = 20;
/// Base level of the orphans of the profiled compaction.
const LEVEL_BASE_ORPHELINS: u32 = 10;
/// Level of the node interned in the shadow plane.
const LEVEL_OMBRE: u32 = 3;
/// Level absent from the index (empty-set test).
const LEVEL_ABSENT: u32 = 9;

/// Seed of the LCG for property tests (same constant as the other suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplier of the LCG for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// Increment of the LCG for property tests.
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

/// Observer enregistreur.
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

/// Builds a small diagram: level-1 node with two edges to two
/// terminals, plus an orphan level-2 node.
fn small_diagram(pool: &mut Arena<MockController>) -> (NodeId, NodeId) {
    let false_node = pool.terminal(ANCRE_TEST, false);
    let true_node = pool.terminal(ANCRE_TEST, true);
    let root = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_ROOT),
            edges: BTreeMap::from([(0, false_node), (1, true_node)]),
        },
    );
    let orphelin = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_ORPHELIN),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    (root, orphelin)
}

/// M4-1: main/shadow planes and the shadow-plane index (level → forms).
#[test]
fn test_plans_and_index_shadow() {
    let mut pool = arena();
    let (root, orphelin) = small_diagram(&mut pool);
    // By default, everything is in the main plane.
    assert_eq!(pool.plan_of(root), Some(Plan::Main));
    // A node interned in the shadow plane stays there.
    let shadow = pool.intern_plan(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_OMBRE),
            edges: BTreeMap::from([(1, root)]),
        },
        Plan::Shadow,
    );
    assert_eq!(pool.plan_of(shadow), Some(Plan::Shadow));
    assert_eq!(pool.plan_of(orphelin), Some(Plan::Main));
    // Shadow-plane index: level 1 contains the root.
    assert!(pool.index_level(Level(LEVEL_ROOT)).contains(&root));
    assert!(pool.index_level(Level(LEVEL_OMBRE)).contains(&shadow));
    assert!(pool.index_level(Level(LEVEL_ABSENT)).is_empty());
}

/// M4-1: mark-and-sweep GC — only the nodes reached from the roots
/// survive; tombstones are removed from the unique table and from
/// the index, and `resolve` rejects them.
#[test]
fn test_gc_mark_sweep() {
    let mut pool = arena();
    let (root, orphelin) = small_diagram(&mut pool);
    let before = pool.node_count();
    let stats = pool.gc(&[root]);
    // R-default removed the default edge: reachable = root + true.
    assert_eq!(stats.live, 2, "root + true terminal");
    assert_eq!(stats.collected, (before as u64) - 2);
    // Compaction depends on the profiled threshold applied to the observed dead.
    assert_eq!(
        stats.compaction,
        threshold_reached(stats.collected, before as u64)
    );
    assert_eq!(pool.live(), 2);
    assert_eq!(
        pool.resolve(orphelin),
        Err(PoolError::UnknownNode(orphelin)),
        "collected node = tombstone"
    );
    assert!(!pool.index_level(Level(LEVEL_ORPHELIN)).contains(&orphelin));
    // Re-interning the orphan's form recreates a node (cache miss).
    let true_node = pool.terminal(ANCRE_TEST, true);
    let false_node = pool.terminal(ANCRE_TEST, false);
    let recree = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_ORPHELIN),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    // After compaction the ids are recycled: the assertion is about the
    // validity of the new node, not the value of its id.
    assert!(pool.resolve(recree).is_ok());
    assert_eq!(pool.plan_of(recree), Some(Plan::Main));
}

/// M4-1: profiled compaction — when dead × 4 ≥ total, the GC compacts and
/// remaps the `NodeId`s; the live handles remain valid.
#[test]
fn test_compaction_profilee() {
    let mut pool = arena();
    let (root, _orphelin) = small_diagram(&mut pool);
    // Many orphans to exceed the THRESHOLD_COMPACTION threshold.
    let mut orphans = Vec::new();
    for i in 0..NB_ORPHELINS {
        orphans.push(pool.intern(
            ANCRE_TEST,
            Form::Node {
                level: Level(LEVEL_BASE_ORPHELINS + i as u32),
                edges: BTreeMap::from([(1, root)]),
            },
        ));
    }
    let total = pool.node_count() as u64;
    assert!(
        threshold_reached(NB_ORPHELINS, total),
        "compaction threshold reached"
    );
    let stats = pool.gc(&[root]);
    assert!(stats.compaction, "profiled compaction triggered");
    assert_eq!(pool.live(), 2);
    assert_eq!(pool.node_count(), 2, "compacted tombstones");
    // Compaction invalidates the old NodeIds: the live root is
    // remapped to a new id, resolvable and indexed at level 1.
    let remap = pool.last_remap();
    let new = remap[root.0 as usize].expect("live root remapped");
    assert_ne!(new, root);
    assert!(pool.resolve(new).is_ok());
    assert!(pool.index_level(Level(LEVEL_ROOT)).contains(&new));
    // The threshold is not reached with few dead.
    assert!(!threshold_reached(1, 100));
    assert_eq!(THRESHOLD_COMPACTION_NUM, 1);
    assert_eq!(THRESHOLD_COMPACTION_DEN, 4);
}

/// M4-1: cache metrics — hash-consing creations/sharings, hit
/// rate, GC counters — and `Gc` event emitted to the observer.
#[test]
fn test_metrics_cache_and_event_gc() {
    let observer = Arc::new(RecordingObserver::default());
    struct Pont(Arc<RecordingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    let mut pool = Arena::new(
        MockController { keys: MockKeys },
        Box::new(Pont(observer.clone())),
    );
    let (root, orphelin) = small_diagram(&mut pool);
    // Re-interning the exact form of the root: hit (R-share, same handle).
    let Form::Node { level, edges } = pool.resolve(root).unwrap().clone() else {
        panic!("the root is a node");
    };
    let shared = pool.intern(ANCRE_TEST, Form::Node { level, edges });
    assert_eq!(shared, root, "R-share: same handle");
    let metrics = pool.metrics();
    assert!(
        metrics.creations >= 4,
        "counted creations: {}",
        metrics.creations
    );
    assert!(metrics.shared >= 1, "counted sharings: {}", metrics.shared);
    assert!(pool.metrics().rate_hit() > 0);
    let stats = pool.gc(&[root]);
    let metrics = pool.metrics();
    assert_eq!(metrics.gc, 1);
    assert_eq!(metrics.live, stats.live);
    assert_eq!(metrics.dead, stats.collected);
    assert_eq!(metrics.compactions, if stats.compaction { 1 } else { 0 });
    // Gc event emitted.
    let events = observer.events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Gc {
                live,
                collected,
                ..
            } if *live == stats.live && *collected == stats.collected
        )),
        "Gc event emitted"
    );
    let _ = orphelin;
}

/// M4-1: `intern_plan` on a hash-consing hit re-plans the shared node
/// (the form already exists: the requested plane overwrites the
/// current plane of the returned handle).
#[test]
fn test_intern_plan_hit_cache_replanned() {
    let mut pool = arena();
    let (root, _orphelin) = small_diagram(&mut pool);
    assert_eq!(pool.plan_of(root), Some(Plan::Main));
    let Form::Node { level, edges } = pool.resolve(root).unwrap().clone() else {
        panic!("the root is a node");
    };
    // Cache hit with shadow plane: same handle, plane moved.
    let same = pool.intern_plan(ANCRE_TEST, Form::Node { level, edges }, Plan::Shadow);
    assert_eq!(same, root, "R-share: same handle");
    assert_eq!(pool.plan_of(root), Some(Plan::Shadow));
}

/// M4-1 (property tier, LCG): invariants of the compaction threshold, the
/// hit rate and the GC — bound `dead × 4 == total` included, `rate_hit` bounded,
/// `live + collected == total` over random scenarios.
#[test]
fn test_property_threshold_rate_hit_and_invariant_gc() {
    let mut graine = GRAINE_LCG;
    let mut next = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    // Threshold: the bound is included (dead × DEN == total × NUM → reached),
    // and one dead fewer does not reach it (for total > 0).
    for _ in 0..1000 {
        let total = 1 + next() % 1000;
        let dead = next() % (total + 1);
        assert_eq!(
            threshold_reached(dead, total),
            dead * THRESHOLD_COMPACTION_DEN >= total * THRESHOLD_COMPACTION_NUM
        );
        assert!(!threshold_reached(0, 0), "zero total: no compaction");
    }
    // Exact bound: the smallest `dead` reaching the threshold is
    // `ceil(total × NUM / DEN)` ; one dead fewer does not reach it.
    for total in 1..=64u64 {
        let threshold = (total * THRESHOLD_COMPACTION_NUM).div_ceil(THRESHOLD_COMPACTION_DEN);
        assert!(
            threshold_reached(threshold, total),
            "bound reached: total={total}"
        );
        assert!(
            !threshold_reached(threshold - 1, total),
            "one dead fewer below the bound: total={total}"
        );
    }

    // Hit rate: 0 without activity, otherwise percentage bounded in [0, 100].
    for _ in 0..1000 {
        let creations = next() % 500;
        let shared = next() % 500;
        let metrics = lisage_core::biplan::MetricsPool {
            creations,
            shared,
            ..Default::default()
        };
        let rate = metrics.rate_hit();
        assert!(rate <= 100);
        let expected = shared
            .checked_mul(lisage_core::biplan::BASE_POURCENTAGE)
            .and_then(|p| p.checked_div(creations + shared))
            .unwrap_or(0);
        assert_eq!(rate, expected);
    }

    // GC invariant: live + collected == total, and the metrics
    // reflect the last GC.
    for _ in 0..100 {
        let mut pool = arena();
        let (root, _orphelin) = small_diagram(&mut pool);
        let count_orphans = next() % 30;
        for i in 0..count_orphans {
            pool.intern(
                ANCRE_TEST,
                Form::Node {
                    level: Level(LEVEL_BASE_ORPHELINS + i as u32),
                    edges: BTreeMap::from([(1, root)]),
                },
            );
        }
        let total = pool.node_count() as u64;
        let stats = pool.gc(&[root]);
        assert_eq!(stats.live + stats.collected, total);
        assert_eq!(stats.live, pool.live() as u64);
        let metrics = pool.metrics();
        assert_eq!(metrics.live, stats.live);
        assert_eq!(metrics.dead, stats.collected);
        assert_eq!(metrics.compactions, if stats.compaction { 1 } else { 0 });
    }
}

/// M4-1 (review regression): a second GC on the same pool without
/// intermediate compaction counts the live ones over the live entries,
/// not on the number of slots (which includes the tombstones of the first GC).
#[test]
fn test_gc_two_times_live_exact() {
    let mut pool = arena();
    let (root, _orphelin) = small_diagram(&mut pool);
    // Chain of nodes all reached from the root: the first GC does not
    // collects only the orphan and the false terminal (2 dead out of a large
    // total), the threshold is not reached, the tombstones remain.
    let mut current = root;
    for i in 0..NB_ORPHELINS {
        current = pool.intern(
            ANCRE_TEST,
            Form::Node {
                level: Level(LEVEL_BASE_ORPHELINS + i as u32 + 100),
                edges: BTreeMap::from([(1, current)]),
            },
        );
    }
    // Roots = root + end of chain: the whole chain is reached,
    // only the orphan and the false terminal are collected.
    let first = pool.gc(&[root, current]);
    assert!(!first.compaction, "few dead: no compaction");
    assert!(pool.node_count() > pool.live(), "tombstones kept");
    // Second GC: the live ones must not count the tombstones of the
    // first GC — live + collected == live entries before the GC.
    let live_before = pool.live() as u64;
    let stats = pool.gc(&[root, current]);
    assert_eq!(stats.live, pool.live() as u64);
    assert_eq!(stats.live + stats.collected, live_before);
    let metrics = pool.metrics();
    assert_eq!(metrics.live, stats.live);
    assert_eq!(metrics.gc, 2);
}
