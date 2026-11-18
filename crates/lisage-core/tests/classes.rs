//! Tests of the per-class arenas (MEM-9, ADR-0103): `NodeId` routed by
//! class (degree), frozen class map, C1 inter-class (same form, same
//! `NodeId`, whatever the tier), catch-all class beyond the window, and
//! per-class live/peak metrics.

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
