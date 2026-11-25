//! MEM-16: narrowed key storage (u8/u16/u32 declared per level by
//! cardinality) - PBT round-trip on the narrowed keys, exact-class
//! routing with inline arcs, and the open-domain catch-all unchanged.

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, KeyWidth, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

/// Bounded domain |Val(l)| = 4, rank = identity on 0..4 (key width u8).
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
    fn unrank(&self, _level: Level, rank: u64) -> Option<Self::Key> {
        (rank < 4).then_some(rank)
    }
}

/// Open domain: cardinality beyond 32 bits, no rank - no lossless
/// narrowing, the key/value run path is unchanged.
struct MockKeysOpen;

impl KeyAlgebra for MockKeysOpen {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        u64::MAX
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
    type Payload = bool;
    type Keys = MockKeys;
    type Arcs = ArcRun<u64>;
    const FINGERPRINT: ControllerId = ControllerId(42);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

struct MockControllerOpen {
    keys: MockKeysOpen,
}

impl Controller for MockControllerOpen {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeysOpen;
    type Arcs = ArcRun<u64>;
    const FINGERPRINT: ControllerId = ControllerId(43);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

fn nouvelle_arena() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

fn nouvelle_arena_open() -> Arena<MockControllerOpen> {
    Arena::new(
        MockControllerOpen { keys: MockKeysOpen },
        Box::new(NoOpObserver),
    )
}

proptest! {
    /// PBT round-trip (criterion 1): on a bounded domain, every ranked
    /// key narrows then widens back to itself; a key outside the domain
    /// does not narrow.
    #[test]
    fn test_round_trip_keys_retrillies(
        key in 0u64..8,
        level in 0u32..3,
    ) {
        let keys = MockKeys;
        let level = Level(level);
        prop_assert_eq!(keys.key_width(level), Some(KeyWidth::U8));
        match keys.rank(level, &key) {
            Some(_) => {
                let narrowed = keys.narrow(level, &key).expect("ranked key narrows");
                prop_assert_eq!(narrowed, key as u32, "rank = identity on 0..4");
                prop_assert_eq!(keys.widen(level, narrowed), Some(key), "round-trip");
            }
            None => {
                prop_assert!(keys.narrow(level, &key).is_none(), "unranked key does not narrow");
            }
        }
    }

    /// PBT: a node with narrowable keys lands in the exact class of its
    /// degree and resolves back to the same semantic form (round-trip
    /// through the inline storage). Degenerate case (issue #346): when
    /// every arc points to the same target, R-collapse reduces the form
    /// to that target and no node is created - there is no exact class.
    #[test]
    fn test_class_exact_and_round_trip_form(
        pairs in prop::collection::vec((0u64..4, 0u32..8), 1..5),
        level in 0u32..3,
    ) {
        let mut pool = nouvelle_arena();
        let mut dedup: std::collections::BTreeMap<u64, u32> = std::collections::BTreeMap::new();
        for (key, target) in &pairs {
            dedup.insert(*key, *target);
        }
        let pairs: Vec<(u64, u32)> = dedup.into_iter().collect();
        let form = Form::Node {
            level: Level(level),
            arcs: pairs
                .iter()
                .map(|(key, target)| (*key, NodeId(*target)))
                .collect::<ArcRun<u64>>(),
        };
        let node = pool.intern(Anchor(0), form.clone());
        let degree = pairs.len() as u32;
        let distinct_targets: std::collections::BTreeSet<u32> =
            pairs.iter().map(|(_, target)| *target).collect();
        // R-collapse (issue #346): without a level default, collapse holds
        // only when ALL values are explicit (arcs.len() >= cardinality) and
        // every target is the same - then no node is created and the
        // interned handle IS the shared target (e.g. TERMINAL_CLASS 0).
        let covers_all_values = pairs.len() as u64 >= 4; // MockKeys::cardinality
        if distinct_targets.len() == 1 && covers_all_values {
            let target = NodeId(*distinct_targets.iter().next().expect("one target"));
            prop_assert_eq!(node, target, "R-collapse to the shared target");
            return Ok(());
        }
        // Exact class: the class of the node is its degree.
        let (class, _) = node.decode(pool.class_bits());
        prop_assert_eq!(class, degree, "narrowable node in the exact class");
        // Round-trip: resolve returns the same semantic content (the
        // family may be re-chosen canonically, MEM-12 - compare pairs).
        let resolved = pool.resolve(node).unwrap();
        let Form::Node { arcs, .. } = &resolved else {
            panic!("node expected");
        };
        let obtained: Vec<(u64, NodeId)> = arcs.iter().map(|(k, t)| (*k, *t)).collect();
        let expected: Vec<(u64, NodeId)> = pairs
            .iter()
            .map(|(key, target)| (*key, NodeId(*target)))
            .collect();
        prop_assert_eq!(obtained, expected, "inline storage round-trip");
        // C1: re-interning the resolved form shares the node.
        let again = pool.intern(Anchor(0), resolved);
        prop_assert_eq!(again, node, "C1 through the inline storage");
    }
}

/// Issue #346 regression (deterministic pin of the proptest seed): four
/// distinct keys all pointing to the same terminal, all values explicit
/// and no level default - R-collapse applies, no node is created, and the
/// handle is the terminal itself (TERMINAL_CLASS 0, not degree 4).
#[test]
fn test_collapse_all_keys_to_terminal() {
    let mut pool = nouvelle_arena();
    let form = Form::Node {
        level: Level(0),
        arcs: (0u64..4)
            .map(|key| (key, NodeId(0)))
            .collect::<ArcRun<u64>>(),
    };
    let node = pool.intern(Anchor(0), form);
    assert_eq!(node, NodeId(0), "R-collapse to the shared terminal");
    let (class, _) = node.decode(pool.class_bits());
    assert_eq!(class, 0, "collapsed handle lives in the terminal class");
}

/// Criterion 3: on an open domain (cardinality beyond 32 bits, no rank),
/// no narrowing is possible - the node goes to the catch-all class and
/// the key/value run path is unchanged (full-width keys, same form).
#[test]
fn test_domain_open_way_unchanged() {
    let mut pool = nouvelle_arena_open();
    let keys = MockKeysOpen;
    assert_eq!(
        keys.key_width(Level(0)),
        None,
        "open domain: no declared width"
    );
    assert!(keys.narrow(Level(0), &7).is_none());
    let form = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(7u64, NodeId(1)), (9u64, NodeId(2))]),
    };
    let node = pool.intern(Anchor(0), form.clone());
    let (class, _) = node.decode(pool.class_bits());
    assert_eq!(
        class,
        pool.class_map().catch_all,
        "open domain: catch-all class"
    );
    assert_eq!(pool.resolve(node).unwrap(), form, "key/value run unchanged");
}
