//! `Send`/`Sync` bounds of the interface traits checked at compiled time
//! (M1-0) — complement of the lisage-bench source test.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};

const fn assert_send_sync<T: Send + Sync + ?Sized>() {}

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

struct MockPool;

impl Pool for MockPool {
    type Key = u64;
    type Payload = bool;
    const CONTROLLER: ControllerId = MockController::FINGERPRINT;
    fn intern(&mut self, _anchor: Anchor, _form: Form<Self::Key, Self::Payload>) -> NodeId {
        NodeId(0)
    }
    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError> {
        Err(PoolError::UnknownNode(node))
    }
    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle {
        Handle {
            controller: Self::CONTROLLER,
            anchor,
            node,
        }
    }
    fn node_count(&self) -> usize {
        0
    }
}

#[test]
fn test_interface_send_sync() {
    assert_send_sync::<MockPool>();
    assert_send_sync::<MockController>();
    assert_send_sync::<NoOpObserver>();
    assert_send_sync::<dyn Observer>();
    assert_send_sync::<TraceEvent>();
    assert_send_sync::<Handle>();
    assert_send_sync::<BTreeMap<u64, NodeId>>();
}
