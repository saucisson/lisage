//! Two-plane pool (M4-1, statement C5): **main** and **shadow** planes, GC
//! mark-and-sweep, **profiled** compaction (dead threshold), and cache
//! metrics (hash-consing creations/shares, GC live/dead).
//!
//! The shadow plane carries the level → forms index (spec § 10 Q7: "the
//! level → forms index lives in the shadow plane"), the main plane carries
//! the diagram nodes. Storage stays shared (C1); only traversal
//! distinguishes the planes.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Level, NodeId};

/// Plane of a node in the two-plane pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Plan {
    /// Main plane: the diagram nodes.
    Principal,
    /// Shadow plane: the level → forms index and auxiliary nodes.
    Shadow,
}

/// Profiled compaction threshold: compact when the tombstone proportion
/// reaches 1/4 of the slots (dead × 4 ≥ total).
pub const THRESHOLD_COMPACTION_NUM: u64 = 1;
/// Denominator of the compaction threshold.
pub const THRESHOLD_COMPACTION_DEN: u64 = 4;

/// Percentage base for `rate_hit` (returned in percent, integer).
pub const BASE_PERCENTAGE: u64 = 100;

/// Pool metrics: hash-consing cache and GC counters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MetricsPool {
    /// Nodes created (hash-consing miss).
    pub creations: u64,
    /// Nodes shared (hash-consing hit, R-share).
    pub shared: u64,
    /// Live nodes after the last GC.
    pub live: u64,
    /// Nodes collected in total.
    pub dead: u64,
    /// Number of GCs executed.
    pub gc: u64,
    /// Number of compactions executed.
    pub compactions: u64,
    /// Terminal singletons of the pool (MEM-11): one per distinct payload.
    pub terminals: u64,
}

impl MetricsPool {
    /// Cache hit rate = shares / (creations + shares); 0 if empty.
    pub fn rate_hit(&self) -> u64 {
        let total = self.creations + self.shared;
        self.shared
            .checked_mul(BASE_PERCENTAGE)
            .and_then(|p| p.checked_div(total))
            .unwrap_or(0)
    }
}

/// GC statistics: live, collected, compaction performed, marking bitmap
/// size in bytes (MEM-10: 1 bit per node).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatsGc {
    /// Live nodes (reached from the roots).
    pub live: u64,
    /// Collected nodes (tombstones set).
    pub collected: u64,
    /// Compaction performed (dead threshold reached).
    pub compaction: bool,
    /// Marking bitmap size in bytes at the last GC (1 bit per slot).
    pub mark_bytes: u64,
}

/// Two-plane pool entry: plane + form (tombstone after GC).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct NodeEntry<Key, Payload> {
    pub plan: Plan,
    pub form: Option<lisage_algebra::interface::Form<Key, Payload>>,
}

/// Shadow-plane index: level → live nodes of that level. A `Vec` per level
/// (PERF-5, issue #288): `push` at creation is amortized and allocation-free
/// per node, against one tree-node allocation per created node for the
/// previous `BTreeSet`. No duplicates are possible (the unique table
/// guarantees a single creation per form); GC `retain` and compaction
/// rebuild keep the same semantics.
pub(crate) type IndexLevels = BTreeMap<Level, Vec<NodeId>>;

/// Decides whether the profiled compaction threshold is reached.
pub fn threshold_reached(dead: u64, total: u64) -> bool {
    dead * THRESHOLD_COMPACTION_DEN >= total * THRESHOLD_COMPACTION_NUM && total > 0
}
