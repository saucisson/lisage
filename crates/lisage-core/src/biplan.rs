//! Two-plane pool (M4-1, statement C5): **main** and **shadow** planes, GC
//! mark-and-sweep, **profiled** compaction (dead threshold), and
//! cache metrics (hash-consing creations/sharings, GC live/dead).
//!
//! The shadow plane carries the level → forms index (spec § 10 Q7: "the
//! level → forms index lives in the shadow plane"), the main plane carries the
//! nodes of the diagram. Storage stays shared (C1); only the traversal
//! distinguishes the planes.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Level, NodeId};

/// Plane of a node of the two-plane pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Plan {
    /// Main plane: the nodes of the diagram.
    Main,
    /// Shadow plane: the level → forms index and the auxiliary nodes.
    Shadow,
}

/// Profiled compaction threshold: we compact when the proportion of
/// tombstones reach 1/4 of the slots (dead × 4 ≥ total).
pub const THRESHOLD_COMPACTION_NUM: u64 = 1;
/// Denominator of the compaction threshold.
pub const THRESHOLD_COMPACTION_DEN: u64 = 4;

/// Percentage base for `rate_hit` (returned in percent, integer).
pub const BASE_POURCENTAGE: u64 = 100;

/// Pool metrics: hash-consing cache and GC counters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MetricsPool {
    /// Nodes created (hash-consing miss).
    pub creations: u64,
    /// Nodes shared (hash-consing hit, R-share).
    pub shared: u64,
    /// Nodes alive after the last GC.
    pub live: u64,
    /// Nodes collected in total.
    pub dead: u64,
    /// Number of GCs executed.
    pub gc: u64,
    /// Number of compactions executed.
    pub compactions: u64,
}

impl MetricsPool {
    /// Cache hit rate = sharings / (creations + sharings); 0 if empty.
    pub fn rate_hit(&self) -> u64 {
        let total = self.creations + self.shared;
        self.shared
            .checked_mul(BASE_POURCENTAGE)
            .and_then(|p| p.checked_div(total))
            .unwrap_or(0)
    }
}

/// Statistics of a GC: live, collected, compaction performed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatsGc {
    /// Live nodes (reached from the roots).
    pub live: u64,
    /// Collected nodes (tombstones set).
    pub collected: u64,
    /// Compaction performed (dead threshold reached).
    pub compaction: bool,
}

/// Entry of the two-plane pool: plane + form (tombstone after GC).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct InputNode<Key, Payload> {
    pub plan: Plan,
    pub form: Option<lisage_algebra::interface::Form<Key, Payload>>,
}

/// Index of the shadow plane: level → live nodes of this level.
pub(crate) type IndexLevels = BTreeMap<Level, BTreeSet<NodeId>>;

/// Decides whether the profiled compaction threshold is reached.
pub fn threshold_reached(dead: u64, total: u64) -> bool {
    dead * THRESHOLD_COMPACTION_DEN >= total * THRESHOLD_COMPACTION_NUM && total > 0
}
