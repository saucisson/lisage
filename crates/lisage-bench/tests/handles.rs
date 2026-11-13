//! PBT of statement C1 (M1-1): handle uniqueness — two constructions of
//! the same form share the same node (R-share), two distinct forms have
//! distinct handles. Canonicity is tested by handle
//! (controller, anchor, form), not by bare node (spec § 2.4.3).

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

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

fn new_arena() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

fn form_random(pairs: Vec<(u8, u32)>, level: u8) -> Form<u64, bool> {
    Form::Node {
        level: Level(u32::from(level)),
        edges: pairs
            .into_iter()
            .map(|(key, target)| (u64::from(key), NodeId(target)))
            .collect::<BTreeMap<u64, NodeId>>(),
    }
}

proptest! {
    /// C1: deterministic interning — the same form interned twice
    /// (same arena or distinct arenas) gives the same handle.
    #[test]
    fn test_c1_uniqueness_handles(pairs in prop::collection::vec((0u8..3, 0u32..4), 0..5), level in 0u8..4) {
        let form = form_random(pairs, level);
        let anchor = Anchor(0);
        let mut arena_a = new_arena();
        let mut arena_b = new_arena();
        let node_a1 = arena_a.intern(anchor, form.clone());
        let node_a2 = arena_a.intern(anchor, form.clone());
        let node_b = arena_b.intern(anchor, form);
        prop_assert_eq!(node_a1, node_a2, "same form ⇒ same node in the same arena");
        prop_assert_eq!(node_a1, node_b, "same form ⇒ same node across arenas");
        prop_assert_eq!(arena_a.handle(node_a1, anchor), arena_b.handle(node_b, anchor));
    }

    /// C1 (canonicity by handle, spec § 2.4.3): two forms with the same
    /// reduced form (after R-collapse/R-default) share the same handle; two
    /// distinct reduced forms have distinct handles.
    #[test]
    fn test_c1_canonicity_by_handle(pairs_a in prop::collection::vec((0u8..3, 0u32..2), 0..5), pairs_b in prop::collection::vec((0u8..3, 0u32..2), 0..5), level in 0u8..4) {
        use lisage_algebra::reduction::{reduce, Reduction};
        let is_default = |node: NodeId| node == NodeId(0);
        let reduced = |pairs: Vec<(u8, u32)>| -> Reduction<u64> {
            let edges: std::collections::BTreeMap<u64, NodeId> = pairs
                .into_iter()
                .map(|(key, target)| (u64::from(key), NodeId(target)))
                .collect();
            reduce(edges, Some(0), &is_default, 2)
        };
        prop_assume!(!pairs_a.is_empty() && !pairs_b.is_empty());
        let form_a = form_random(pairs_a.clone(), level);
        let form_b = form_random(pairs_b.clone(), level);
        let reduced_a = reduced(pairs_a);
        let reduced_b = reduced(pairs_b);
        let mut arena = new_arena();
        let anchor = Anchor(0);
        // edge targets must be real nodes: we intern the false terminal
        // (D_δ = NodeId(0)) then the true terminal (NodeId(1)), which makes
        // the test's is_default predicate identical to the arena's.
        let default = arena.terminal(anchor, false);
        prop_assert_eq!(default, NodeId(0));
        let other = arena.terminal(anchor, true);
        prop_assert_eq!(other, NodeId(1));
        let node_a = arena.intern(anchor, form_a);
        let node_b = arena.intern(anchor, form_b);
        // C1, uniqueness: same reduced form ⇒ same handle (shared node).
        if reduced_a == reduced_b {
            prop_assert_eq!(node_a, node_b, "same reduced form ⇒ same handle (C1)");
        } else {
            prop_assert_ne!(node_a, node_b, "distinct reduced forms ⇒ distinct nodes (C1)");
        }
    }
}
