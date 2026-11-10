//! PBT de la règle R-collapse (M1-0) : `Controller::collapse` est une
//! fonction pure, testée par propriété — toute implémentation fausse
//! (toujours `None`, toujours la première cible) est réfutée.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level, NodeId};
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

    const FINGERPRINT: ControllerId = ControllerId(1);

    fn default_payload(_level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

/// Construit une table d'arcs arbitraire à partir de paires (clé, cible).
fn arcs_of(pairs: Vec<(u8, u32)>) -> BTreeMap<u64, NodeId> {
    pairs
        .into_iter()
        .map(|(key, target)| (u64::from(key), NodeId(target)))
        .collect()
}

#[test]
fn test_collapse_forme_vide() {
    assert_eq!(
        MockController::collapse(&BTreeMap::<u64, NodeId>::new()),
        None
    );
}

#[test]
fn test_collapse_un_seul_arc() {
    let arcs = arcs_of(vec![(1, 7)]);
    assert_eq!(MockController::collapse(&arcs), Some(NodeId(7)));
}

#[test]
fn test_collapse_tous_egaux() {
    let arcs = arcs_of(vec![(0, 3), (1, 3), (2, 3)]);
    assert_eq!(MockController::collapse(&arcs), Some(NodeId(3)));
}

#[test]
fn test_collapse_deux_cibles() {
    let arcs = arcs_of(vec![(0, 3), (1, 4)]);
    assert_eq!(MockController::collapse(&arcs), None);
}

proptest! {
    /// R-collapse : `Some(cible)` ssi la table est non vide et toutes les
    /// cibles sont égales ; `None` sinon. Toute implémentation constante est
    /// réfutée par cette propriété.
    #[test]
    fn test_collapse_pbt(pairs in prop::collection::vec((0u8..4, 0u32..10), 0..8)) {
        let arcs = arcs_of(pairs);
        let result = MockController::collapse(&arcs);
        let all_equal = !arcs.is_empty()
            && arcs.values().all(|target| *target == *arcs.values().next().unwrap());
        if all_equal {
            prop_assert_eq!(result, Some(*arcs.values().next().unwrap()));
        } else {
            prop_assert_eq!(result, None);
        }
    }
}
