//! Tests of the `ArcStore` port (MEM-3, ADR-0103): the default instance
//! is the current `BTreeMap`, and any layout implementing the port is
//! seen by the reductions only through its ordered iteration.

use std::collections::BTreeMap;

use lisage_algebra::interface::{ArcStore, ArcStoreBuilder, NodeId};
use lisage_algebra::reduction::collapse_arcs;

/// Alternative layout proving the port is not tied to `BTreeMap`: a
/// sorted `Vec` of `(key, target)` pairs, built through the builder.
#[derive(Clone, Debug, PartialEq, Eq)]
struct VecArcStore {
    pairs: Vec<(u64, NodeId)>,
}

impl ArcStore for VecArcStore {
    type Key = u64;
    type Builder = VecArcStoreBuilder;

    fn iter(&self) -> impl Iterator<Item = (&Self::Key, &NodeId)> {
        self.pairs.iter().map(|(key, target)| (key, target))
    }

    fn len(&self) -> usize {
        self.pairs.len()
    }
}

#[derive(Default)]
struct VecArcStoreBuilder {
    pairs: Vec<(u64, NodeId)>,
}

impl ArcStoreBuilder for VecArcStoreBuilder {
    type Key = u64;
    type Store = VecArcStore;

    fn insert(&mut self, key: Self::Key, target: NodeId) {
        self.pairs.push((key, target));
    }

    fn build(mut self) -> Self::Store {
        self.pairs.sort_by_key(|(key, _)| *key);
        VecArcStore { pairs: self.pairs }
    }
}

#[test]
fn test_default_instance_is_btreemap() {
    let mut arcs: BTreeMap<u64, NodeId> = BTreeMap::new();
    ArcStoreBuilder::insert(&mut arcs, 2, NodeId(20));
    ArcStoreBuilder::insert(&mut arcs, 1, NodeId(10));
    let store: BTreeMap<u64, NodeId> = ArcStoreBuilder::build(arcs);
    let collected: Vec<(u64, NodeId)> = store.iter().map(|(k, t)| (*k, *t)).collect();
    assert_eq!(collected, vec![(1, NodeId(10)), (2, NodeId(20))]);
    assert_eq!(store.len(), 2);
    assert!(!store.is_empty());
    assert!(BTreeMap::<u64, NodeId>::new().is_empty());
}

#[test]
fn test_collapse_sees_only_the_port_iteration() {
    let mut builder = VecArcStoreBuilder::default();
    builder.insert(2, NodeId(7));
    builder.insert(0, NodeId(7));
    builder.insert(1, NodeId(7));
    let store = builder.build();
    assert_eq!(collapse_arcs(&store), Some(NodeId(7)));

    let mut builder = VecArcStoreBuilder::default();
    builder.insert(0, NodeId(3));
    builder.insert(1, NodeId(4));
    let store = builder.build();
    assert_eq!(collapse_arcs(&store), None);
    assert_eq!(collapse_arcs(&BTreeMap::<u64, NodeId>::new()), None);
}
