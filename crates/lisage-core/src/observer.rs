//! Instrumentation port (M1-0, M1-9): `Observer` in `core/`, no-op by
//! default; JSONL serializer in `lisage-liseuse`.

use lisage_algebra::interface::{Handle, Level, NodeId};

/// Reduction rule applied at handle creation (spec § 2.3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReductionRule {
    /// R-share: the form already existed, the node is shared.
    Share,
    /// R-collapse: all arcs lead to the same target.
    Collapse,
    /// R-default: the arc δ(ℓ) led to the default diagram, it is removed.
    Default,
}

/// Channel of an attribute (view 3 "step by step", colored channels — M2-6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CanalAttr {
    /// **Inherited** attribute (descending, `φ_disp`) — rendered in blue.
    Herite,
    /// **Synthesized** attribute (ascending, `φ_aggr`) — rendered in orange.
    Synthétise,
    /// **Level memory** (written by `φ_out`) — rendered in purple.
    Memoire,
}

/// Typed events emitted by the pool during diagram construction and by the
/// transduction runner during execution (M2-6). This is the trace that
/// Liseuse serializes and renders.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TraceEvent {
    /// Node created in the pool (after reductions).
    NodeCreated { node: NodeId, level: Option<Level> },
    /// Shared node: the form was already interned (R-share).
    NodeShared { node: NodeId },
    /// Reduction applied at handle creation.
    ReductionApplied { rule: ReductionRule, level: Level },
    /// Diagram completed: root handle and pool size.
    DiagramBuilt { handle: Handle, nodes: usize },
    /// Attribute flowing on a channel (inherited/synthesized/memory), M2-6.
    Attr {
        /// Channel of the attribute.
        canal: CanalAttr,
        /// Node concerned.
        noeud: NodeId,
        /// Value (`Debug` format), displayed as-is.
        valeur: String,
    },
    /// Memo hit: the tuple (op, node, h, version) was in the computed table.
    MemoHit {
        /// Node whose visit hit.
        noeud: NodeId,
    },
    /// Memo miss: full visit of the subtree.
    MemoMiss {
        /// Node whose visit missed.
        noeud: NodeId,
    },
    /// GC of the bi-plan pool (M4-1): live, collected, profiled compaction.
    Gc {
        /// Live nodes after collection.
        vivants: u64,
        /// Collected nodes.
        collectes: u64,
        /// Compaction performed (dead threshold reached).
        compaction: bool,
    },
    /// Start of a saturation round (M2-6): iteration number.
    Round {
        /// Round number (1-based).
        numero: u32,
    },
}

/// Trace observer, no-op by default (Liseuse provides its own). `Send`/`Sync`
/// bounds frozen (M1-0).
pub trait Observer: Send + Sync {
    /// Reception of a trace event.
    fn on_event(&self, event: &TraceEvent);
}

/// No-op observer (default).
pub struct NoOpObserver;

impl Observer for NoOpObserver {
    fn on_event(&self, _event: &TraceEvent) {}
}
