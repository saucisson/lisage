//! Instrumentation port (M1-0, M1-9): `Observer` in `core/`, no-op by
//! default; JSONL serializer in `lisage-liseuse`.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
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

/// Classification of the structure change between the input form and the
/// output form of one node visit (D1-2), computed generically by the
/// runner — never by the operations themselves.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StructureChangeKind {
    /// Output form identical to the input form.
    Identite,
    /// Same arcs, different level (relabelling).
    Reetiquetage,
    /// Node collapsed to a terminal.
    Collapse,
    /// Terminal expanded to a node (T-EXPAND).
    Expansion,
    /// Arcs restructured (keys or targets changed).
    Restructuration,
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

/// Maximum number of elements in an `Ensemble`/`Carte` trace value
/// (D1-3): the value is bounded, the excess is elided.
pub const TRACE_VAL_MAX_JETONS: usize = 64;

/// Maximum length (in chars) of a `Texte` trace value (D1-3).
pub const TRACE_VAL_MAX_TEXTE: usize = 256;

/// Maximum depth of `Fragment` resolution when forcing a trace value
/// (D1-3): beyond it, the value elides.
pub const TRACE_VAL_MAX_PROFONDEUR: usize = 16;

/// Bounded mini-ADT of a trace value (D1-3). Small values are eager
/// (`Entier`/`Texte`/`Ensemble`/`Carte`); expensive values are lazy
/// (`Fragment`: computed only when forced, e.g. by the serializer or by
/// an observer whose filter passes); `Elide` marks an elided value. The
/// ADT is bounded: collections are capped at `TRACE_VAL_MAX_JETONS`
/// elements and texts at `TRACE_VAL_MAX_TEXTE` chars (enforced by the
/// constructors).
#[derive(Clone)]
pub enum TraceVal {
    /// Integer value.
    Entier(i64),
    /// Text value (bounded by the constructor).
    Texte(String),
    /// Set of values (bounded by the constructor).
    Ensemble(Vec<TraceVal>),
    /// Map of key → value pairs (bounded by the constructor).
    Carte(Vec<(TraceVal, TraceVal)>),
    /// Lazy fragment: the value is computed only when forced.
    Fragment(Arc<dyn Fn() -> TraceVal + Send + Sync>),
    /// Elided value (bound reached or filter rejected).
    Elide,
}

impl TraceVal {
    /// Integer value.
    pub fn entier(valeur: i64) -> Self {
        TraceVal::Entier(valeur)
    }

    /// Text value, truncated to `TRACE_VAL_MAX_TEXTE` chars.
    pub fn texte(texte: impl Into<String>) -> Self {
        let texte = texte.into();
        let tronque: String = texte.chars().take(TRACE_VAL_MAX_TEXTE).collect();
        if tronque.chars().count() < texte.chars().count() {
            TraceVal::Texte(format!("{tronque}…"))
        } else {
            TraceVal::Texte(tronque)
        }
    }

    /// Set value, capped at `TRACE_VAL_MAX_JETONS` elements (the cap
    /// appends an `Elide` marker).
    pub fn ensemble(valeurs: Vec<TraceVal>) -> Self {
        let mut valeurs = valeurs;
        if valeurs.len() > TRACE_VAL_MAX_JETONS {
            valeurs.truncate(TRACE_VAL_MAX_JETONS);
            valeurs.push(TraceVal::Elide);
        }
        TraceVal::Ensemble(valeurs)
    }

    /// Map value, capped at `TRACE_VAL_MAX_JETONS` pairs (the cap
    /// appends an `Elide` marker).
    pub fn carte(paires: Vec<(TraceVal, TraceVal)>) -> Self {
        let mut paires = paires;
        if paires.len() > TRACE_VAL_MAX_JETONS {
            paires.truncate(TRACE_VAL_MAX_JETONS);
            paires.push((TraceVal::Elide, TraceVal::Elide));
        }
        TraceVal::Carte(paires)
    }

    /// Lazy fragment: `valeur` is formatted with `Debug` **only when the
    /// fragment is forced** — never on the emission hot path.
    pub fn fragment<T: fmt::Debug + Send + Sync + 'static>(valeur: T) -> Self {
        TraceVal::Fragment(Arc::new(move || TraceVal::texte(format!("{valeur:?}"))))
    }

    /// Lazy formatted text: the `format!` runs **only when the fragment
    /// is forced** — never on the emission hot path.
    pub fn fragment_format(f: impl Fn() -> String + Send + Sync + 'static) -> Self {
        TraceVal::Fragment(Arc::new(move || TraceVal::texte(f())))
    }

    /// Forces the value: resolves the `Fragment` chain (bounded by
    /// `TRACE_VAL_MAX_PROFONDEUR`) and returns an eager value.
    pub fn force(&self) -> TraceVal {
        self.force_profondeur(0)
    }

    fn force_profondeur(&self, profondeur: usize) -> TraceVal {
        match self {
            TraceVal::Fragment(fragment) if profondeur < TRACE_VAL_MAX_PROFONDEUR => {
                fragment().force_profondeur(profondeur + 1)
            }
            TraceVal::Fragment(_) => TraceVal::Elide,
            TraceVal::Ensemble(valeurs) => TraceVal::Ensemble(
                valeurs
                    .iter()
                    .map(|v| v.force_profondeur(profondeur))
                    .collect(),
            ),
            TraceVal::Carte(paires) => TraceVal::Carte(
                paires
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.force_profondeur(profondeur),
                            v.force_profondeur(profondeur),
                        )
                    })
                    .collect(),
            ),
            autre => autre.clone(),
        }
    }

    /// Renders the value as a display string (forcing lazy fragments):
    /// `Entier` → `42`, `Texte` → as-is, `Ensemble` → `{a, b}`,
    /// `Carte` → `{k: v}`, `Elide` → `…`.
    pub fn rendre(&self) -> String {
        match self.force() {
            TraceVal::Entier(valeur) => format!("{valeur}"),
            TraceVal::Texte(texte) => texte,
            TraceVal::Ensemble(valeurs) => format!(
                "{{{}}}",
                valeurs
                    .iter()
                    .map(|v| v.rendre())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TraceVal::Carte(paires) => format!(
                "{{{}}}",
                paires
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k.rendre(), v.rendre()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TraceVal::Elide => "…".to_string(),
            TraceVal::Fragment(_) => "…".to_string(),
        }
    }
}

impl PartialEq for TraceVal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TraceVal::Entier(a), TraceVal::Entier(b)) => a == b,
            (TraceVal::Texte(a), TraceVal::Texte(b)) => a == b,
            (TraceVal::Ensemble(a), TraceVal::Ensemble(b)) => a == b,
            (TraceVal::Carte(a), TraceVal::Carte(b)) => a == b,
            (TraceVal::Elide, TraceVal::Elide) => true,
            // Two lazy fragments are equal iff they share the same
            // allocation (the payload is not forced to compare).
            (TraceVal::Fragment(a), TraceVal::Fragment(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for TraceVal {}

impl fmt::Debug for TraceVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceVal::Entier(valeur) => write!(f, "Entier({valeur})"),
            TraceVal::Texte(texte) => write!(f, "Texte({texte:?})"),
            TraceVal::Ensemble(valeurs) => write!(f, "Ensemble({valeurs:?})"),
            TraceVal::Carte(paires) => write!(f, "Carte({paires:?})"),
            TraceVal::Fragment(_) => write!(f, "Fragment(<lazy>)"),
            TraceVal::Elide => write!(f, "Elide"),
        }
    }
}

/// Conversion of a value into a trace value (D1-3). The default
/// implementation is the **Debug fallback**: a lazy fragment, formatted
/// only if forced. Types with a meaningful structure override it with
/// eager `Entier`/`Texte`/`Ensemble`/`Carte` values.
pub trait Traceable: fmt::Debug + Clone + Send + Sync + 'static {
    /// Trace value of `self`.
    fn trace_val(&self) -> TraceVal {
        TraceVal::fragment(self.clone())
    }
}

impl Traceable for () {
    fn trace_val(&self) -> TraceVal {
        TraceVal::Elide
    }
}

impl Traceable for bool {
    fn trace_val(&self) -> TraceVal {
        TraceVal::Entier(i64::from(*self))
    }
}

impl Traceable for u64 {
    fn trace_val(&self) -> TraceVal {
        TraceVal::Entier(*self as i64)
    }
}

impl Traceable for NodeId {
    fn trace_val(&self) -> TraceVal {
        TraceVal::Entier(self.0 as i64)
    }
}

impl Traceable for Level {
    fn trace_val(&self) -> TraceVal {
        TraceVal::Entier(i64::from(self.0))
    }
}

impl Traceable for BTreeSet<u64> {
    fn trace_val(&self) -> TraceVal {
        TraceVal::ensemble(self.iter().map(|cle| cle.trace_val()).collect())
    }
}

impl Traceable for BTreeMap<u64, NodeId> {
    fn trace_val(&self) -> TraceVal {
        TraceVal::carte(
            self.iter()
                .map(|(cle, cible)| (cle.trace_val(), cible.trace_val()))
                .collect(),
        )
    }
}

impl<M: Traceable> Traceable for Vec<(NodeId, Level, M)> {
    fn trace_val(&self) -> TraceVal {
        TraceVal::ensemble(
            self.iter()
                .map(|(node, level, valeur)| {
                    TraceVal::carte(vec![
                        (node.trace_val(), level.trace_val()),
                        (TraceVal::Elide, valeur.trace_val()),
                    ])
                })
                .collect(),
        )
    }
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
    /// Node created in the pool (after reductions). D1-2: carries the
    /// full form of the created node.
    NodeCreated {
        /// Node created.
        node: NodeId,
        /// Level of the node, `None` for terminals.
        level: Option<Level>,
        /// Full form of the node (lazy `TraceVal`, D1-3).
        forme: TraceVal,
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
        /// Value (lazy `TraceVal`, D1-3), displayed as-is.
        value: TraceVal,
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
    /// D1-2: carries the list of the collected nodes.
    Gc {
        /// Live nodes after collection.
        live: u64,
        /// Collected nodes.
        collected: u64,
        /// Compaction performed (dead threshold reached).
        compaction: bool,
        /// Identifiers of the collected nodes (D1-2).
        liste_collectes: Vec<NodeId>,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Saturation round (M2-6, extended D1-2): iteration number, active
    /// and modified levels, handles before/image/after, growth.
    Round {
        /// Round number (1-based).
        numero: u32,
        /// Active levels of the round.
        niveaux_actifs: Vec<u32>,
        /// Levels modified by the round.
        niveaux_modifies: Vec<u32>,
        /// Handle before the round.
        avant: NodeId,
        /// Handle of the image computed by the round.
        image: NodeId,
        /// Handle after the union.
        apres: NodeId,
        /// Growth of the pool in nodes during the round.
        croissance: i64,
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
    /// Start of an operation (D1-2): emitted right after the `Op` span opens.
    OpStart {
        /// Fingerprint of the transduction run.
        fingerprint: u64,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// End of an operation (D1-2): emitted right before the `Op` span closes.
    OpEnd {
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Entry into a node visit (D1-2), right after the `Visite` span opens.
    VisiteEntree {
        /// Node visited.
        node: NodeId,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Exit of a node visit (D1-2), right before the `Visite` span closes.
    VisiteSortie {
        /// Node visited.
        node: NodeId,
        /// Resulting node of the visit.
        resultat: NodeId,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Consultation of the memo table (D1-2) with the full key:
    /// (op fingerprint, node, h, version).
    MemoConsult {
        /// Fingerprint of the operation.
        fingerprint: u64,
        /// Node of the key.
        node: NodeId,
        /// Inherited attribute of the key (lazy `TraceVal`, D1-3).
        h: TraceVal,
        /// Memory version of the key.
        version: u64,
        /// Whether the consultation hit.
        hit: bool,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Insertion into the memo table (D1-2) with the full key.
    MemoInsert {
        /// Fingerprint of the operation.
        fingerprint: u64,
        /// Node of the key.
        node: NodeId,
        /// Inherited attribute of the key (lazy `TraceVal`, D1-3).
        h: TraceVal,
        /// Memory version of the key.
        version: u64,
        /// Result stored under the key.
        resultat: NodeId,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Memory write (D1-2): level-memory version before -> after.
    EcritureMemoire {
        /// Version before the write.
        version_avant: u64,
        /// Version after the write.
        version_apres: u64,
        /// Span in which the event occurs (D1-1).
        span: Option<SpanId>,
    },
    /// Structure change of one node visit (D1-2): classification and arc
    /// correspondence between the input form and the output form.
    StructureChange {
        /// Node visited.
        node: NodeId,
        /// Classification of the change.
        classification: StructureChangeKind,
        /// Arcs of the input form `(key, target)`.
        arcs_avant: Vec<(u64, NodeId)>,
        /// Arcs of the output form `(key, target)`.
        arcs_apres: Vec<(u64, NodeId)>,
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
            | TraceEvent::Histogram { span: s, .. }
            | TraceEvent::OpStart { span: s, .. }
            | TraceEvent::OpEnd { span: s, .. }
            | TraceEvent::VisiteEntree { span: s, .. }
            | TraceEvent::VisiteSortie { span: s, .. }
            | TraceEvent::MemoConsult { span: s, .. }
            | TraceEvent::MemoInsert { span: s, .. }
            | TraceEvent::EcritureMemoire { span: s, .. }
            | TraceEvent::StructureChange { span: s, .. } => *s = span,
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
            | TraceEvent::Histogram { span, .. }
            | TraceEvent::OpStart { span, .. }
            | TraceEvent::OpEnd { span, .. }
            | TraceEvent::VisiteEntree { span, .. }
            | TraceEvent::VisiteSortie { span, .. }
            | TraceEvent::MemoConsult { span, .. }
            | TraceEvent::MemoInsert { span, .. }
            | TraceEvent::EcritureMemoire { span, .. }
            | TraceEvent::StructureChange { span, .. } => *span,
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
