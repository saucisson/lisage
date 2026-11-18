//! Tests of the per-class arenas (MEM-9, ADR-0103): `NodeId` routed by
//! class (degree), frozen class map, C1 inter-class (same form, same
//! `NodeId`, whatever the tier), catch-all class beyond the window, and
//! per-class live/peak metrics.

use lisage_algebra::class_bits::TERMINAL_CLASS;
use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// Mock algebra: cardinality 2 (→ b = 2), no default (no R-default).
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
        None
    }
}

/// Mock controller with a `u64` payload: many distinct terminals, so a
/// node whose arcs lead to distinct targets never R-collapses.
struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = u64;
    type Keys = MockKeys;
    type Arcs = ArcRun<u64>;

    const FINGERPRINT: ControllerId = ControllerId(1);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

type PoolMock = Arena<MockController>;

fn pool() -> PoolMock {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

/// Intern a node of the given degree (number of arcs) at level 0, arc
/// `key` leading to terminal `key` — distinct targets, no R-collapse.
fn intern_degree(pool: &mut PoolMock, degree: u32) -> NodeId {
    let targets: Vec<NodeId> = (0..degree as u64)
        .map(|key| pool.terminal(Anchor(0), key))
        .collect();
    let arcs = ArcRun::from_pairs((0..degree as u64).zip(targets));
    pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(0),
            arcs,
        },
    )
}

#[test]
fn test_class_map_frozen_at_construction() {
    // Cardinality 2 → b = 2, classes 0..=3, catch-all = 3, C = 2.
    let pool = pool();
    let map = pool.class_map();
    assert_eq!(map.class_bits, 2);
    assert_eq!(map.number_classes(), 4);
    assert_eq!(map.exact_max, 2);
    assert_eq!(map.catch_all, 3);
    assert_eq!(map.class_of_degree(0), 0);
    assert_eq!(map.class_of_degree(1), 1);
    assert_eq!(map.class_of_degree(2), 2);
    assert_eq!(map.class_of_degree(3), 3, "beyond the window: catch-all");
    assert_eq!(map.class_of_degree(100), 3);
}

#[test]
fn test_c1_inter_class_same_form_same_node_id() {
    // C1 inter-class: the same form interns to the same NodeId whatever
    // the tier (exact classes 1..=2 and catch-all 3 at b = 2).
    let mut pool = pool();
    for degree in [1u32, 2, 3, 10] {
        let first = intern_degree(&mut pool, degree);
        let second = intern_degree(&mut pool, degree);
        assert_eq!(first, second, "same form, same NodeId (degree {degree})");
        let (class, _index) = first.decode(pool.class_map().class_bits);
        assert_eq!(
            class,
            pool.class_map().class_of_degree(degree),
            "NodeId carries the class of the degree {degree}"
        );
        // A different form of the same degree: different NodeId, same class.
        let targets: Vec<NodeId> = (0..degree as u64)
            .map(|key| pool.terminal(Anchor(0), key + 1000))
            .collect();
        let other = pool.intern(
            Anchor(0),
            Form::Node {
                level: Level(1),
                arcs: ArcRun::from_pairs((0..degree as u64).zip(targets)),
            },
        );
        assert_ne!(first, other);
        assert_eq!(other.decode(pool.class_map().class_bits).0, class);
    }
}

#[test]
fn test_catch_all_class_holds_sorted_run_entries() {
    // Degree beyond the window (b = 2 → C = 2): the node lands in the
    // catch-all class and stays resolvable — its form carries the sorted
    // run. No loss of correctness.
    let mut pool = pool();
    let node = intern_degree(&mut pool, 5);
    let (class, _index) = node.decode(pool.class_map().class_bits);
    assert_eq!(class, pool.class_map().catch_all);
    match pool.resolve(node) {
        Ok(Form::Node { arcs, .. }) => {
            assert_eq!(arcs.len(), 5);
            let keys: Vec<u64> = arcs.keys().copied().collect();
            assert_eq!(keys, vec![0, 1, 2, 3, 4], "sorted run");
        }
        other => panic!("expected a node, got {other:?}"),
    }
}

/// Intern a degree-2 node at the given level (arcs to two distinct
/// terminals — no R-collapse), class 2 at b = 2.
fn intern_degree2_at(pool: &mut PoolMock, level: u32) -> NodeId {
    let t0 = pool.terminal(Anchor(0), 0);
    let t1 = pool.terminal(Anchor(0), 1);
    pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(level),
            arcs: ArcRun::from_pairs([(0, t0), (1, t1)]),
        },
    )
}

/// Intern a degree-3 node at the given level, catch-all class 3 at b = 2.
fn intern_degree3_at(pool: &mut PoolMock, level: u32) -> NodeId {
    let t0 = pool.terminal(Anchor(0), 0);
    let t1 = pool.terminal(Anchor(0), 1);
    let t2 = pool.terminal(Anchor(0), 2);
    pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(level),
            arcs: ArcRun::from_pairs([(0, t0), (1, t1), (2, t2)]),
        },
    )
}

#[test]
fn test_marking_bitmap_memory_under_old_btree_set() {
    // 2000 nodes: the marking bitmap is 1 bit per slot, far under the old
    // `BTreeSet<NodeId>` (4-byte ids plus tree overhead per node).
    let mut pool = pool();
    let roots: Vec<NodeId> = (0..2000u32)
        .map(|i| intern_degree2_at(&mut pool, i))
        .collect();
    let total = pool.node_count() as u64;
    let stats = pool.gc(&roots);
    assert_eq!(stats.live, total, "all roots live");
    // Exact bitmap size: one word per 64 slots, per class arena.
    let classes = pool.class_map().number_classes() as u64;
    assert!(
        stats.mark_bytes * 8 <= total + 64 * classes,
        "1 bit per node: {} bytes for {} slots",
        stats.mark_bytes,
        total
    );
    // Measured threshold: under the raw ids alone of the old BTreeSet.
    assert!(
        stats.mark_bytes < total * 4,
        "marking bitmap ({} B) under the old set's raw ids ({} B)",
        stats.mark_bytes,
        total * 4
    );
}

#[test]
fn test_no_index_reused_outside_remap_protocol() {
    // Below the compaction threshold: tombstones keep their slots, a
    // re-interned dead form gets a FRESH index, never the stale one.
    let mut pool = pool();
    let nodes: Vec<NodeId> = (0..10u32)
        .map(|i| intern_degree2_at(&mut pool, i))
        .collect();
    let stats = pool.gc(&nodes[..9]);
    assert!(!stats.compaction, "below the threshold: no compaction");
    let stale = nodes[9];
    assert!(pool.resolve(stale).is_err(), "stale id is unknown");
    let recreated = intern_degree2_at(&mut pool, 9);
    assert_ne!(recreated, stale, "index not reused outside the protocol");
    assert!(pool.resolve(recreated).is_ok());
    // With compaction, reuse happens only through the recorded remap.
    let mut orphans = Vec::new();
    for i in 100..140u32 {
        orphans.push(intern_degree2_at(&mut pool, i));
    }
    let stats = pool.gc(&nodes[..9]);
    assert!(stats.compaction, "threshold reached: compaction");
    let remap = pool.last_remap();
    for orphan in &orphans {
        assert_eq!(remap.get(&orphan.0), Some(&None), "orphan remapped to None");
    }
    assert!(
        remap.get(&nodes[0].0).is_some_and(|n| n.is_some()),
        "live root remapped"
    );
}

#[test]
fn test_emptied_class_releases_pages_without_recycling_indices() {
    // 100 live catch-all nodes (class 3) + 3 dead exact-class nodes
    // (class 2): below the threshold, class 2 empties and releases its
    // pages, keeping the slot high-water mark.
    let mut pool = pool();
    let live: Vec<NodeId> = (0..100u32)
        .map(|i| intern_degree3_at(&mut pool, i))
        .collect();
    let dead: Vec<NodeId> = (0..3u32)
        .map(|i| intern_degree2_at(&mut pool, 1000 + i))
        .collect();
    let slots_before = pool.node_count();
    let stats = pool.gc(&live);
    assert!(!stats.compaction);
    assert_eq!(stats.live, live.len() as u64 + 3, "live + 3 terminals");
    let by_class = |class: u32| {
        pool.class_metrics()
            .into_iter()
            .find(|m| m.class == class)
            .unwrap_or_default()
    };
    assert_eq!(by_class(2).live, 0, "class 2 emptied");
    assert_eq!(
        pool.node_count(),
        slots_before,
        "pages released but slots kept as high-water mark"
    );
    for node in &dead {
        assert!(pool.resolve(*node).is_err());
    }
    // A new node of the emptied class gets a fresh index beyond the
    // high-water mark — no stale id aliased.
    let fresh = intern_degree2_at(&mut pool, 2000);
    let (class, index) = fresh.decode(pool.class_map().class_bits);
    assert_eq!(class, 2);
    assert_eq!(index, 3, "index beyond the 3 old slots of class 2");
    assert!(pool.resolve(fresh).is_ok());
}

#[test]
fn test_per_class_live_and_peak_traced() {
    let mut pool = pool();
    let n1 = intern_degree(&mut pool, 1);
    let n2 = intern_degree(&mut pool, 2);
    let n3 = intern_degree(&mut pool, 3);
    let by_class = |pool: &PoolMock, class: u32| {
        pool.class_metrics()
            .into_iter()
            .find(|m| m.class == class)
            .unwrap_or_default()
    };
    // Terminals in class 0, degrees 1/2 in their exact classes, degree 3
    // in the catch-all.
    assert_eq!(by_class(&pool, 0).live, 3, "3 shared terminals (0, 1, 2)");
    assert_eq!(by_class(&pool, 1).live, 1);
    assert_eq!(by_class(&pool, 1).peak, 1);
    assert_eq!(by_class(&pool, 2).live, 1);
    assert_eq!(by_class(&pool, 3).live, 1);
    assert_eq!(by_class(&pool, 3).peak, 1);
    // GC keeping only the degree-1 node (and its terminal): live drops
    // per class, peaks stay.
    let stats = pool.gc(&[n1]);
    assert_eq!(stats.live, 2, "n1 + its terminal");
    assert_eq!(by_class(&pool, 1).live, 1);
    assert_eq!(by_class(&pool, 2).live, 0);
    assert_eq!(by_class(&pool, 3).live, 0);
    assert_eq!(by_class(&pool, 2).peak, 1, "peak survives the GC");
    assert!(pool.resolve(n2).is_err(), "collected node is unknown");
    assert!(pool.resolve(n3).is_err());
    assert!(pool.resolve(n1).is_ok());
}

/// Mock algebra with a designated default key δ(ℓ) = 0, cardinality 3.
struct MockKeysDefault;

impl KeyAlgebra for MockKeysDefault {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        3
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(0)
    }
}

/// Mock controller designating D_δ = terminal 99 (R-default active).
struct MockControllerDefault {
    keys: MockKeysDefault,
}

impl Controller for MockControllerDefault {
    type Key = u64;
    type Payload = u64;
    type Keys = MockKeysDefault;
    type Arcs = ArcRun<u64>;

    const FINGERPRINT: ControllerId = ControllerId(3);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(99)
    }
}

#[test]
fn test_terminal_singleton_per_pool_across_anchors() {
    // MEM-11: a terminal is a pool-wide singleton keyed by payload only —
    // the anchor does not enter its canonicity.
    let mut pool = pool();
    let t_a = pool.terminal(Anchor(0), 42);
    let t_b = pool.terminal(Anchor(7), 42);
    assert_eq!(t_a, t_b, "same payload, same terminal NodeId pool-wide");
    let (class, _index) = t_a.decode(pool.class_map().class_bits);
    assert_eq!(
        class, TERMINAL_CLASS,
        "terminals live in the dedicated class 0"
    );
    let t_c = pool.terminal(Anchor(0), 43);
    assert_ne!(t_a, t_c, "distinct payloads: distinct terminals");
    assert_eq!(pool.metriques().terminaux, 2, "one singleton per payload");
    // Re-interning shares the singleton instead of creating a new one.
    let t_d = pool.terminal(Anchor(3), 42);
    assert_eq!(t_d, t_a);
    assert_eq!(pool.metriques().terminaux, 2);
    assert!(pool.metriques().partages >= 2, "hits counted as shares");
    // GC keeps live terminal singletons; re-interning still shares.
    let stats = pool.gc(&[t_a]);
    assert!(stats.live >= 1);
    assert!(pool.resolve(t_a).is_ok());
    let t_e = pool.terminal(Anchor(2), 42);
    assert_eq!(t_e, t_a, "singleton survives the GC");
}

#[test]
fn test_r_collapse_inter_anchor_shares_terminal() {
    // MEM-11: R-collapse (all arcs to the same target) yields the terminal
    // singleton whatever the anchor — canonicity by handle only concerns
    // internal nodes.
    let mut pool = pool();
    let t = pool.terminal(Anchor(0), 7);
    let form = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(0, t), (1, t)]),
    };
    let n_a = pool.intern(Anchor(0), form.clone());
    let n_b = pool.intern(Anchor(5), form);
    assert_eq!(n_a, t, "R-collapse to the terminal");
    assert_eq!(n_b, t, "same collapse under another anchor");
}

#[test]
fn test_r_default_sees_pool_wide_default_terminal_across_anchors() {
    // MEM-11: R-default absorbs the arc δ(ℓ) → D_δ; D_δ is interned once
    // per pool, so the reduced form is identical under every anchor.
    let mut pool = Arena::new(
        MockControllerDefault {
            keys: MockKeysDefault,
        },
        Box::new(NoOpObserver),
    );
    let d = pool.terminal(Anchor(0), 99); // D_δ
    let t5 = pool.terminal(Anchor(0), 5);
    let t6 = pool.terminal(Anchor(0), 6);
    let form = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(0, d), (1, t5), (2, t6)]),
    };
    let n_a = pool.intern(Anchor(0), form.clone());
    let n_a2 = pool.intern(Anchor(0), form.clone());
    assert_eq!(n_a, n_a2, "same reduced form, same anchor: same NodeId");
    // Under another anchor the internal node gets its own handle
    // (canonicity by handle), but its arcs reference the SAME terminal
    // singletons — est_defaut saw the pool-wide D_δ.
    let n_b = pool.intern(Anchor(9), form);
    assert_ne!(n_a, n_b, "internal nodes stay per-anchor handles");
    for node in [n_a, n_b] {
        match pool.resolve(node) {
            Ok(Form::Node { arcs, .. }) => {
                assert_eq!(arcs.len(), 2, "the δ(0) arc toward D_δ was absorbed");
                assert_eq!(arcs.get(&1), Some(&t5), "same terminal across anchors");
                assert_eq!(arcs.get(&2), Some(&t6), "same terminal across anchors");
            }
            other => panic!("expected a node, got {other:?}"),
        }
    }
    assert_eq!(
        pool.metriques().terminaux,
        3,
        "D_δ interned once per pool, not once per anchor"
    );
}

/// Mock controller with a large payload (4 KiB): the terminal class holds
/// payloads of any size without touching internal nodes.
struct BigController {
    keys: MockKeys,
}

impl Controller for BigController {
    type Key = u64;
    type Payload = [u8; 4096];
    type Keys = MockKeys;
    type Arcs = ArcRun<u64>;

    const FINGERPRINT: ControllerId = ControllerId(4);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

#[test]
fn test_large_payload_terminal_in_dedicated_class() {
    let mut pool = Arena::new(BigController { keys: MockKeys }, Box::new(NoOpObserver));
    let t_big = pool.terminal(Anchor(0), [7u8; 4096]);
    let t_big_again = pool.terminal(Anchor(1), [7u8; 4096]);
    assert_eq!(t_big, t_big_again, "large payload: pool-wide singleton too");
    let (class, _index) = t_big.decode(pool.class_map().class_bits);
    assert_eq!(class, TERMINAL_CLASS);
    // Internal nodes are unaffected: a degree-2 node over two distinct
    // large terminals lands in its exact class, not in class 0.
    let t_other = pool.terminal(Anchor(0), [8u8; 4096]);
    let n = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(0),
            arcs: ArcRun::from_pairs([(0, t_big), (1, t_other)]),
        },
    );
    let (n_class, _n_index) = n.decode(pool.class_map().class_bits);
    assert_eq!(n_class, 2, "degree 2 → exact class 2 at b = 2");
    assert!(pool.resolve(n).is_ok());
    assert_eq!(pool.metriques().terminaux, 2);
}
