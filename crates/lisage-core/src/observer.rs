//! Instrumentation port (M1-0, M1-9): `Observer` in `core/`, no-op by
//! default; JSONL serializer in `lisage-liseuse`.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Handle, Level, NodeId};

/// Identifier of a trace span (D1-1, trace v4). Process-global counter:
/// spans opened by different components (runner, saturation, pool) never
/// collide.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpanId(pub u64);

/// Kind of a trace span (D1-1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpanKind {
    /// Whole operation: transduction run, diagram build, measure.
    Op,
    /// One saturation round.
    Round,
    /// Visit of one node.
    Visite,
}

static NEXT_SPAN_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Stack of the currently open spans (D1-1). Thread-local so that
    /// nested components (saturation round -> runner op -> node visit)
    /// share one span context without plumbing it through every call.
    static SPAN_STACK: RefCell<Vec<SpanId>> = const { RefCell::new(Vec::new()) };
}

/// Allocates a fresh span identifier.
pub fn next_span_id() -> SpanId {
    SpanId(NEXT_SPAN_ID.fetch_add(1, Ordering::Relaxed))
}

/// Pushes an open span on the current stack.
pub fn push_span(span: SpanId) {
    SPAN_STACK.with(|stack| stack.borrow_mut().push(span));
}

/// Pops the innermost open span, returning it.
pub fn pop_span() -> Option<SpanId> {
    SPAN_STACK.with(|stack| stack.borrow_mut().pop())
}

/// Innermost open span, `None` outside any span.
pub fn current_span() -> Option<SpanId> {
    SPAN_STACK.with(|stack| stack.borrow().last().copied())
}

/// Attaches the current span to an event (D1-1): every event carries its
/// span; `SpanOpen`/`SpanClose` define spans and stay unspanned.
pub fn contextualiser(mut evenement: TraceEvent) -> TraceEvent {
    evenement.set_span(current_span());
    evenement
}

/// Reduction rule applied at handle creation (spec § 2.3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReductionRule {
    /// R-share: the form already existed, the node is shared.
    Share,
    /// R-collapse: all arcs lead to the same target.
    Collapse,
    /// R-default: the δ(ℓ) arc led to the default diagram, it is removed.
    Default,
}

/// Channel of an attribute (view 3 "step by step", colored channels — M2-6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChannelAttr {
    /// **Inherited** attribute (downward, `φ_disp`) — rendered in blue.
    Inherited,
    /// **Synthesized** attribute (upward, `φ_aggr`) — rendered in orange.
    Synthesized,
    /// **Level memory** (written by `φ_out`) — rendered in purple.
    Memory,
}

/// Per-class degree histogram entry (MEM-17): live, peak and slack
/// (tombstone slots) of one class.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ClassHistogram {
    /// Class identifier.
    pub class: u32,
    /// Live nodes in the class.
    pub live: u64,
    /// Peak of live nodes in the class.
    pub peak: u64,
    /// Slack: tombstone slots kept in the class arena.
    pub slack: u64,
}

/// Typed events emitted by the pool during diagram construction and by the
/// transduction runner during execution (M2-6). This is the trace that
/// Liseuse serializes and renders.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TraceEvent {
    /// A span opens (D1-1, trace v4): `kind` and `lane` (lane number,
    /// reserved for future parallelism). Defines a span, carries none.
    SpanOpen {
        /// Identifier of the opened span.
        span: SpanId,
        /// Kind of the span.
        kind: SpanKind,
        /// Lane number (0 for sequential execution).
        lane: u32,
    },
    /// A span closes (D1-1, trace v4). Defines a span, carries none.
    SpanClose {
        /// Identifier of the closed span.
        span: SpanId,
    },
    /// Node created in the pool (after reductions).
    NodeCreated {
        /// Node created.
        node: NodeId,
        /// Level of the node, `None` for terminals.
        level: Option<Level>,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Shared node: the form was already interned (R-share).
    NodeShared {
        /// Shared node.
        node: NodeId,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Reduction applied at handle creation.
    ReductionApplied {
        /// Rule applied.
        rule: ReductionRule,
        /// Level concerned.
        level: Level,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Diagram complete: root handle and pool size.
    DiagramBuilt {
        /// Root handle.
        handle: Handle,
        /// Pool size in nodes.
        nodes: usize,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Attribute flowing on a channel (inherited/synthesized/memory), M2-6.
    Attr {
        /// Channel of the attribute.
        channel: ChannelAttr,
        /// Node concerned.
        node: NodeId,
        /// Value (`Debug` format), displayed as-is.
        value: String,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Memo hit: the (op, node, h, version) tuple was in the computed table.
    MemoHit {
        /// Node whose visit hit.
        node: NodeId,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Memo miss: full visit of the subtree.
    MemoMiss {
        /// Node whose visit missed.
        node: NodeId,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// GC of the two-plane pool (M4-1): live, collected, profiled compaction.
    Gc {
        /// Live nodes after collection.
        live: u64,
        /// Collected nodes.
        collected: u64,
        /// Compaction performed (dead threshold reached).
        compaction: bool,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Start of a saturation round (M2-6): iteration number.
    Round {
        /// Round number (1-based).
        numero: u32,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Degree histogram per class (MEM-17): emitted at each GC — live,
    /// peak and slack per class; fixes the C boundary of MEM-9 and
    /// monitors the drift.
    Histogram {
        /// Per-class entries.
        classes: Vec<ClassHistogram>,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
}

impl TraceEvent {
    /// Attaches a span to the event (D1-1). `SpanOpen`/`SpanClose` define
    /// spans and are left untouched.
    pub fn set_span(&mut self, span: Option<SpanId>) {
        match self {
            TraceEvent::SpanOpen { .. } | TraceEvent::SpanClose { .. } => {}
            TraceEvent::NodeCreated { span: s, .. }
            | TraceEvent::NodeShared { span: s, .. }
            | TraceEvent::ReductionApplied { span: s, .. }
            | TraceEvent::DiagramBuilt { span: s, .. }
            | TraceEvent::Attr { span: s, .. }
            | TraceEvent::MemoHit { span: s, .. }
            | TraceEvent::MemoMiss { span: s, .. }
            | TraceEvent::Gc { span: s, .. }
            | TraceEvent::Round { span: s, .. }
            | TraceEvent::Histogram { span: s, .. } => *s = span,
        }
    }

    /// Span carried by the event, `None` for span boundaries and events
    /// emitted outside any span.
    pub fn span(&self) -> Option<SpanId> {
        match self {
            TraceEvent::SpanOpen { .. } | TraceEvent::SpanClose { .. } => None,
            TraceEvent::NodeCreated { span, .. }
            | TraceEvent::NodeShared { span, .. }
            | TraceEvent::ReductionApplied { span, .. }
            | TraceEvent::DiagramBuilt { span, .. }
            | TraceEvent::Attr { span, .. }
            | TraceEvent::MemoHit { span, .. }
            | TraceEvent::MemoMiss { span, .. }
            | TraceEvent::Gc { span, .. }
            | TraceEvent::Round { span, .. }
            | TraceEvent::Histogram { span, .. } => *span,
        }
    }
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
