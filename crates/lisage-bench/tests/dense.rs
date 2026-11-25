//! PBT of statement C1 with the dense ordinal family (MEM-12): whatever the
//! construction, the same semantic form interns to the same `NodeId`, and the
//! dense family is chosen exactly when the fill is at least one third and
//! every key has a rank.

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

/// |Val(ℓ)| = 4, rank = identity on 0..4.
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

struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = u64;
    type Keys = MockKeys;
    type Arcs = ArcRun<Self::Key>;
    const FINGERPRINT: ControllerId = ControllerId(13);
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

fn form_random(pairs: Vec<(u8, u32)>, level: u8) -> Form<u64, u64> {
    Form::Node {
        level: Level(u32::from(level)),
        arcs: pairs
            .into_iter()
            .map(|(key, target)| (u64::from(key), NodeId(target)))
            .collect::<ArcRun<u64>>(),
    }
}

proptest! {
    /// C1 with the dense family: two constructions of the same form (any
    /// pair order) share the same NodeId; the stored family matches the
    /// documented threshold (fill ≥ 1/3 of |Val| = 4, all keys ranked).
    #[test]
    fn test_c1_family_dense(
        pairs in prop::collection::vec((0u8..4, 0u32..8), 0..5),
        level in 0u8..3,
    ) {
        let mut pool = nouvelle_arena();
        // Deduplicate by key (last write wins, MEM-5) so that the reversed
        // construction has the SAME semantic content.
        let mut dedup: std::collections::BTreeMap<u8, u32> = std::collections::BTreeMap::new();
        for (key, target) in &pairs {
            dedup.insert(*key, *target);
        }
        let pairs: Vec<(u8, u32)> = dedup.into_iter().collect();
        let form = form_random(pairs.clone(), level);
        // Shuffled construction of the same content.
        let mut melange = pairs.clone();
        melange.reverse();
        let form_melangee = form_random(melange, level);
        let first = pool.intern(Anchor(0), form.clone());
        let second = pool.intern(Anchor(0), form);
        let third = pool.intern(Anchor(0), form_melangee);
        prop_assert_eq!(first, second, "same form: same NodeId");
        prop_assert_eq!(first, third, "pair order does not matter (C1)");
        // The stored family follows the threshold: dense iff ≥ 2 arcs
        // (4·4 ≤ 12·arcs ⟺ arcs ≥ 2), all keys < 4 ranked.
        let Form::Node { arcs, .. } = pool.resolve(first).unwrap() else {
            panic!("node expected");
        };
        let expected_dense = arcs.len() >= 2;
        prop_assert_eq!(arcs.is_dense(), expected_dense, "threshold 4·|Val| ≤ 12·arcs");
        // The dense family preserves the content: same pairs, rank order.
        let expected: Vec<(u64, NodeId)> = pairs
            .iter()
            .map(|(key, target)| (u64::from(*key), NodeId(*target)))
            .collect();
        let obtained: Vec<(u64, NodeId)> = arcs.iter().map(|(k, t)| (*k, *t)).collect();
        prop_assert_eq!(obtained, expected);
    }
}
