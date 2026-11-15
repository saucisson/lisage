//! PBT of R-collapse (M1-0): `Controller::collapse` is pure; any
//! wrong implementation (always `None`, always the first target) is
//! refuted by the property.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level, NodeId};
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
    const FINGERPRINT: ControllerId = ControllerId(1);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

fn arcs_of(pairs: Vec<(u8, u32)>) -> BTreeMap<u64, NodeId> {
    pairs
        .into_iter()
        .map(|(key, target)| (u64::from(key), NodeId(target)))
        .collect()
}

#[test]
fn test_collapse_cas_limites() {
    assert_eq!(
        MockController::collapse(&BTreeMap::<u64, NodeId>::new()),
        None
    );
    assert_eq!(
        MockController::collapse(&arcs_of(vec![(1, 7)])),
        Some(NodeId(7))
    );
    assert_eq!(
        MockController::collapse(&arcs_of(vec![(0, 3), (1, 3), (2, 3)])),
        Some(NodeId(3))
    );
    assert_eq!(
        MockController::collapse(&arcs_of(vec![(0, 3), (1, 4)])),
        None
    );
}

proptest! {
    /// R-collapse: `Some(target)` iff the table is non-empty and all the
    /// targets are equal; `None` otherwise.
    #[test]
    fn test_collapse_pbt(pairs in prop::collection::vec((0u8..4, 0u32..10), 0..8)) {
        let arcs = arcs_of(pairs);
        let result = MockController::collapse(&arcs);
        let first = arcs.values().next().copied();
        let all_equal = first.is_some_and(|target| arcs.values().all(|t| *t == target));
        if all_equal {
            prop_assert_eq!(result, first);
        } else {
            prop_assert_eq!(result, None);
        }
    }
}
