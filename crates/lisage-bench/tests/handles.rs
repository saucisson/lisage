//! PBT de l'énoncé C1 (M1-1) : unicité des handles — deux constructions de
//! la même forme partagent le même nœud (R-share), deux formes distinctes
//! ont des handles distincts. La canonicité est testée par handle
//! (contrôleur, ancre, forme), pas par nœud nu (spec § 2.4.3).

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
    fn normal_form(key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(_level: Level) -> u64 {
        2
    }
    fn default(_level: Level) -> Option<Self::Key> {
        Some(0)
    }
}

struct MockController;

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(7);
    fn default_payload(_level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

fn nouvelle_arena() -> Arena<MockController> {
    Arena::new(Box::new(NoOpObserver))
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
    /// C1 : internement déterministe — la même forme internée deux fois
    /// (même arena ou arenas distinctes) donne le même handle.
    #[test]
    fn test_c1_unicite_handles(paires in prop::collection::vec((0u8..3, 0u32..4), 0..5), niveau in 0u8..4) {
        let forme = forme_aleatoire(paires, niveau);
        let anchor = Anchor(0);
        let mut arena_a = nouvelle_arena();
        let mut arena_b = nouvelle_arena();
        let node_a1 = arena_a.intern(anchor, forme.clone());
        let node_a2 = arena_a.intern(anchor, forme.clone());
        let node_b = arena_b.intern(anchor, forme);
        prop_assert_eq!(node_a1, node_a2, "même forme ⇒ même nœud dans la même arena");
        prop_assert_eq!(node_a1, node_b, "même forme ⇒ même nœud entre arenas");
        prop_assert_eq!(arena_a.handle(node_a1, anchor), arena_b.handle(node_b, anchor));
    }

    /// C1 : deux formes distinctes ont des handles distincts (pas de fusion
    /// abusive par le pool).
    #[test]
    fn test_c1_formes_distinctes(paires_a in prop::collection::vec((0u8..3, 0u32..4), 0..5), paires_b in prop::collection::vec((0u8..3, 0u32..4), 0..5), niveau in 0u8..4) {
        let forme_a = forme_aleatoire(paires_a, niveau);
        let forme_b = forme_aleatoire(paires_b, niveau);
        prop_assume!(forme_a != forme_b);
        let mut arena = nouvelle_arena();
        let anchor = Anchor(0);
        let node_a = arena.intern(anchor, forme_a);
        let node_b = arena.intern(anchor, forme_b);
        prop_assert_ne!(arena.handle(node_a, anchor), arena.handle(node_b, anchor));
    }
}
