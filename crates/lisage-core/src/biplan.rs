//! Bi-plan pool (M4-1, statement C5): **main** and **shadow** plans,
//! mark-and-sweep GC, **profiled** compaction (dead threshold), and cache
//! metrics (hash-consing creations/shares, GC live/dead).
//!
//! The shadow plan carries the level → forms index (spec § 10 Q7: "the
//! level → forms index lives in the shadow plan"), the main plan carries the
//! diagram nodes. Storage remains shared (C1); only the traversal
//! distinguishes the plans.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Level, NodeId};

/// Plan of a node of the bi-plan pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Plan {
    /// Main plan: the diagram nodes.
    Principal,
    /// Shadow plan: the level → forms index and auxiliary nodes.
    Ombre,
}

/// Profiled compaction threshold: compact when the proportion of tombstones
/// reaches 1/4 of the slots (dead × 4 ≥ total).
pub const SEUIL_COMPACTION_NUM: u64 = 1;
/// Denominator of the compaction threshold.
pub const SEUIL_COMPACTION_DEN: u64 = 4;

/// Percentage base for `taux_hit` (returned in percent, integer).
pub const BASE_POURCENTAGE: u64 = 100;

/// Pool metrics: hash-consing cache and GC counters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MetriquesPool {
    /// Nodes created (hash-consing miss).
    pub creations: u64,
    /// Nodes shared (hash-consing hit, R-share).
    pub partages: u64,
    /// Live nodes after the last GC.
    pub vivants: u64,
    /// Nodes collected in total.
    pub morts: u64,
    /// Number of GCs executed.
    pub gc: u64,
    /// Number of compactions executed.
    pub compactions: u64,
}

impl MetriquesPool {
    /// Cache hit rate = shares / (creations + shares); 0 if empty.
    pub fn taux_hit(&self) -> u64 {
        let total = self.creations + self.partages;
        self.partages
            .checked_mul(BASE_POURCENTAGE)
            .and_then(|p| p.checked_div(total))
            .unwrap_or(0)
    }
}

/// GC statistics: live, collected, compaction performed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatsGc {
    /// Live nodes (reached from the roots).
    pub vivants: u64,
    /// Collected nodes (tombstones placed).
    pub collectes: u64,
    /// Compaction performed (dead threshold reached).
    pub compaction: bool,
}

/// Entry of the bi-plan pool: plan + form (tombstone after GC).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct EntreeNoeud<Key, Payload> {
    pub plan: Plan,
    pub forme: Option<lisage_algebra::interface::Form<Key, Payload>>,
}

/// Index of the shadow plan: level → live nodes of this level.
pub(crate) type IndexNiveaux = BTreeMap<Level, BTreeSet<NodeId>>;

/// Decides whether the profiled compaction threshold is reached.
pub fn seuil_atteint(morts: u64, total: u64) -> bool {
    morts * SEUIL_COMPACTION_DEN >= total * SEUIL_COMPACTION_NUM && total > 0
}
