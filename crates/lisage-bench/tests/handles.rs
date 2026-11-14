//! PBT of statement C1 (M1-1): handle uniqueness — two constructions of
//! the same form share the same node (R-share), two distinct forms
//! have distinct handles. Canonicity is tested by handle
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

fn nouvelle_arena() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

fn forme_aleatoire(paires: Vec<(u8, u32)>, niveau: u8) -> Form<u64, bool> {
    Form::Node {
        level: Level(u32::from(niveau)),
        arcs: paires
            .into_iter()
            .map(|(cle, cible)| (u64::from(cle), NodeId(cible)))
            .collect::<BTreeMap<u64, NodeId>>(),
    }
}

proptest! {
    /// C1: deterministic interning — the same form interned twice
    /// (same arena or distinct arenas) gives the same handle.
    #[test]
    fn test_c1_unicite_handles(paires in prop::collection::vec((0u8..3, 0u32..4), 0..5), niveau in 0u8..4) {
        let forme = forme_aleatoire(paires, niveau);
        let anchor = Anchor(0);
        let mut arena_a = nouvelle_arena();
        let mut arena_b = nouvelle_arena();
        let node_a1 = arena_a.intern(anchor, forme.clone());
        let node_a2 = arena_a.intern(anchor, forme.clone());
        let node_b = arena_b.intern(anchor, forme);
        prop_assert_eq!(node_a1, node_a2, "same form ⇒ same node in the same arena");
        prop_assert_eq!(node_a1, node_b, "same form ⇒ same node across arenas");
        prop_assert_eq!(arena_a.handle(node_a1, anchor), arena_b.handle(node_b, anchor));
    }

    /// C1 (canonicity by handle, spec § 2.4.3): two forms with the same
    /// reduced form (after R-collapse/R-default) share the same handle; two
    /// distinct reduced forms have distinct handles.
    #[test]
    fn test_c1_canonicite_par_handle(paires_a in prop::collection::vec((0u8..3, 0u32..2), 0..5), paires_b in prop::collection::vec((0u8..3, 0u32..2), 0..5), niveau in 0u8..4) {
        use lisage_algebra::reduction::{reduire, Reduction};
        let est_defaut = |node: NodeId| node == NodeId(0);
        let reduite = |paires: Vec<(u8, u32)>| -> Reduction<u64> {
            let arcs: std::collections::BTreeMap<u64, NodeId> = paires
                .into_iter()
                .map(|(cle, cible)| (u64::from(cle), NodeId(cible)))
                .collect();
            reduire(arcs, Some(0), &est_defaut, 2)
        };
        prop_assume!(!paires_a.is_empty() && !paires_b.is_empty());
        let forme_a = forme_aleatoire(paires_a.clone(), niveau);
        let forme_b = forme_aleatoire(paires_b.clone(), niveau);
        let reduite_a = reduite(paires_a);
        let reduite_b = reduite(paires_b);
        let mut arena = nouvelle_arena();
        let anchor = Anchor(0);
        // arc targets must be real nodes: intern the false terminal
        // (D_δ = NodeId(0)) then the true terminal (NodeId(1)),
        // which makes the test's est_defaut predicate identical to the arena's.
        let defaut = arena.terminal(anchor, false);
        prop_assert_eq!(defaut, NodeId(0));
        let autre = arena.terminal(anchor, true);
        prop_assert_eq!(autre, NodeId(1));
        let node_a = arena.intern(anchor, forme_a);
        let node_b = arena.intern(anchor, forme_b);
        // C1, uniqueness: same reduced form ⇒ same handle (shared node).
        if reduite_a == reduite_b {
            prop_assert_eq!(node_a, node_b, "same reduced form ⇒ same handle (C1)");
        } else {
            prop_assert_ne!(node_a, node_b, "distinct reduced forms ⇒ distinct nodes (C1)");
        }
    }
}
