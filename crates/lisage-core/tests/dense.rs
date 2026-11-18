//! Tests of the dense ordinal arc family (MEM-12, ADR-0103): arcs indexed
//! by rank, holes = default not stored; chosen in normalization when every
//! key has a rank and the fill is at least one third (threshold
//! `4·|Val(ℓ)| ≤ 12·arcs`); composes with the class tier by arc count.

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// Mock algebra: cardinality 4, every key < 4 has its rank (identity),
/// no default (no R-default).
struct MockKeys;

impl KeyAlgebra for MockKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        4
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }

    fn rank(&self, _level: Level, key: &Self::Key) -> Option<u64> {
        (*key < 4).then_some(*key)
    }

    fn unrank(&self, _level: Level, rang: u64) -> Option<Self::Key> {
        (rang < 4).then_some(rang)
    }
}

/// Mock algebra without ranks: the dense family never applies.
struct MockKeysSansRang;

impl KeyAlgebra for MockKeysSansRang {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        4
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = u64;
    type Keys = MockKeys;
    type Arcs = ArcRun<u64>;

    const FINGERPRINT: ControllerId = ControllerId(11);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

fn pool() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

#[test]
fn test_seuil_un_tiers_exact() {
    // Threshold `4·|Val(ℓ)| ≤ 12·arcs`, i.e. fill ≥ 1/3, boundary included.
    let keys = MockKeys; // |Val| = 4
    let level = Level(0);
    // 1 arc out of 4: 4·4 = 16 > 12·1 = 12 → sorted.
    let arcs = ArcRun::from_pairs([(0, NodeId(1))]);
    let chosen = ArcRun::choose_dense_family(level, &keys, arcs);
    assert!(!chosen.is_dense(), "fill 1/4 < 1/3: sorted family");
    // 2 arcs out of 4: 16 ≤ 24 → dense.
    let arcs = ArcRun::from_pairs([(0, NodeId(1)), (3, NodeId(2))]);
    let chosen = ArcRun::choose_dense_family(level, &keys, arcs);
    assert!(chosen.is_dense(), "fill 1/2 ≥ 1/3: dense family");
    // Boundary with |Val| = 3: 1 arc → 12 ≤ 12 → dense.
    struct Trois;
    impl KeyAlgebra for Trois {
        type Key = u64;
        fn normal_form(&self, key: &Self::Key) -> Self::Key {
            *key
        }
        fn cardinality(&self, _level: Level) -> u64 {
            3
        }
        fn default(&self, _level: Level) -> Option<Self::Key> {
            None
        }
        fn rank(&self, _level: Level, key: &Self::Key) -> Option<u64> {
            (*key < 3).then_some(*key)
        }
    }
    let arcs = ArcRun::from_pairs([(2, NodeId(9))]);
    let chosen = ArcRun::choose_dense_family(Level(0), &Trois, arcs);
    assert!(
        chosen.is_dense(),
        "fill exactly 1/3: dense (boundary included)"
    );
}

#[test]
fn test_pas_de_rang_pas_de_dense() {
    // A key without rank (here: every key, the algebra has no rank) keeps
    // the sorted family even with a full fill.
    let keys = MockKeysSansRang;
    let arcs = ArcRun::from_pairs([
        (0, NodeId(1)),
        (1, NodeId(2)),
        (2, NodeId(3)),
        (3, NodeId(4)),
    ]);
    let chosen = ArcRun::choose_dense_family(Level(0), &keys, arcs);
    assert!(
        !chosen.is_dense(),
        "no rank: sorted family whatever the fill"
    );
}

#[test]
fn test_dense_holes_are_the_unstored_default() {
    // Dense family: one slot per rank, holes = default not stored.
    let keys = MockKeys;
    let arcs = ArcRun::from_pairs([(0, NodeId(10)), (3, NodeId(30))]);
    let dense = ArcRun::choose_dense_family(Level(0), &keys, arcs);
    assert!(dense.is_dense());
    assert_eq!(dense.len(), 2, "holes are not arcs");
    assert_eq!(dense.get(&0), Some(&NodeId(10)));
    assert_eq!(dense.get(&3), Some(&NodeId(30)));
    assert_eq!(dense.get(&1), None, "hole = default, not stored");
    assert_eq!(dense.get(&2), None);
    assert!(!dense.contains_key(&1));
    let collected: Vec<(u64, NodeId)> = dense.iter().map(|(k, t)| (*k, *t)).collect();
    assert_eq!(
        collected,
        vec![(0, NodeId(10)), (3, NodeId(30))],
        "rank order"
    );
    let values: Vec<NodeId> = dense.values().copied().collect();
    assert_eq!(values, vec![NodeId(10), NodeId(30)]);
}

#[test]
fn test_dense_choisie_dans_le_pool_et_compose_avec_la_classe() {
    // Through the pool: a fill ≥ 1/3 form is stored dense, and the node
    // still lands in the class of its degree (tier by arc count).
    let mut pool = pool();
    let t0 = pool.terminal(Anchor(0), 100);
    let t3 = pool.terminal(Anchor(0), 300);
    let node = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(0),
            arcs: ArcRun::from_pairs([(0, t0), (3, t3)]),
        },
    );
    let Form::Node { arcs, .. } = pool.resolve(node).unwrap() else {
        panic!("node expected")
    };
    assert!(arcs.is_dense(), "fill 2/4 ≥ 1/3: dense family in the pool");
    assert_eq!(arcs.len(), 2);
    assert_eq!(arcs.get(&3), Some(&t3));
    let (class, _index) = node.decode(pool.class_map().class_bits);
    assert_eq!(
        class,
        pool.class_map().class_of_degree(2),
        "dense node in the class of its degree"
    );
    // Below the threshold the same pool keeps the sorted family.
    let t1 = pool.terminal(Anchor(0), 111);
    let sparse = pool.intern(
        Anchor(0),
        Form::Node {
            level: Level(1),
            arcs: ArcRun::from_pairs([(1, t1)]),
        },
    );
    let Form::Node { arcs, .. } = pool.resolve(sparse).unwrap() else {
        panic!("node expected")
    };
    assert!(!arcs.is_dense(), "fill 1/4 < 1/3: sorted family");
}

#[test]
fn test_c1_avec_la_famille_dense() {
    // C1 with the dense family: the same semantic form interns to the same
    // NodeId whatever the construction order of the pairs.
    let mut pool = pool();
    let t0 = pool.terminal(Anchor(0), 100);
    let t1 = pool.terminal(Anchor(0), 101);
    let t2 = pool.terminal(Anchor(0), 102);
    let t3 = pool.terminal(Anchor(0), 103);
    let form = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(0, t0), (1, t1), (2, t2), (3, t3)]),
    };
    let shuffled = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(3, t3), (0, t0), (2, t2), (1, t1)]),
    };
    let first = pool.intern(Anchor(0), form.clone());
    let second = pool.intern(Anchor(0), form);
    let third = pool.intern(Anchor(0), shuffled);
    assert_eq!(first, second, "same form: same NodeId");
    assert_eq!(first, third, "pair order does not matter (C1)");
    let Form::Node { arcs, .. } = pool.resolve(first).unwrap() else {
        panic!("node expected")
    };
    assert!(arcs.is_dense(), "full fill: dense family");
    assert_eq!(arcs.len(), 4);
}
