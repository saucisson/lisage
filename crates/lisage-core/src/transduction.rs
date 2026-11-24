//! Attributed transduction engine (M1-4, spec § 3.1, statement L3):
//! the two channels — **inherited** attribute `H` (downward, `φ_disp`) and
//! **synthesized** `S` (upward, `φ_aggr`) — and the memo keyed by fingerprint
//! `(op, node, h)` (lemma L1). Rules T-TERM/T-NODE.
//!
//! Since M1-5 (#12), the transduction reads its input from a pool with
//! payload `bool` and writes into a pool with any payload `Out`
//! (`Δ_out` may differ from `Δ_in`, spec § 3.2): `psi` and `φ_out` write
//! themselves (spec: "φ_out is the only rule that writes the diagram"),
//! always by interning (A3: canonical result by construction).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use lisage_algebra::interface::{Anchor, Form, Level, MemoryPolicy, NodeId, Pool};

/// Structural summary of a form (D1-2): level and arcs `(key, target)`.
fn extract_structure<P>(form: &Form<u64, P>) -> (Option<Level>, Vec<(u64, NodeId)>) {
    match form {
        Form::Terminal(_) => (None, Vec::new()),
        Form::Node { level, arcs } => (
            Some(*level),
            arcs.iter().map(|(key, target)| (*key, *target)).collect(),
        ),
    }
}

/// Classifies the structure change between the input and output forms of
/// one node visit (D1-2): identity, relabelling, collapse, expansion or
/// restructuring.
fn classifier_structure(
    before: (Option<Level>, Vec<(u64, NodeId)>),
    after: (Option<Level>, Vec<(u64, NodeId)>),
) -> StructureChangeKind {
    match (before.0, after.0) {
        (None, None) => StructureChangeKind::Identity,
        (Some(level_before), Some(level_after))
            if before.1 == after.1 && level_before == level_after =>
        {
            StructureChangeKind::Identity
        }
        (Some(_), Some(_)) if before.1 == after.1 => StructureChangeKind::Relabeling,
        (Some(_), None) => StructureChangeKind::Collapse,
        (None, Some(_)) => StructureChangeKind::Expansion,
        (Some(_), Some(_)) => StructureChangeKind::Restructuration,
    }
}

use crate::error::CoreError;
use crate::memory::{EffectMemory, MemoryLevel};
use crate::observer::{
    ChannelAttr, Observer, SpanKind, StructureChangeKind, TraceEvent, Traceable, contextualize,
    next_span_id, pop_span, push_span,
};

/// Memo key: (operation fingerprint, node, inherited attribute,
/// per-fragment level-memory versions) — lemma L1, scoped per fragment
/// since M2-5 (the #388 core remainder): the fourth component lists the
/// versions of the fragments the visit DECLARES to read
/// (`read_fragments`), not the global version of the memory — the
/// entries of the untouched fragments survive the writes of the
/// others.
type KeyMemo<H> = (u64, NodeId, H, Vec<u64>);

/// Context of `φ_out` (T-NODE): groups the restructuring parameters
/// (spec § 3.1: `φ_out(ℓ, h, {v ↦ (D', s)}, M)`).
pub struct PhiOutContext<'a, H, S, M: Clone + Eq + Hash + Debug> {
    /// Anchor of the root.
    pub anchor: Anchor,
    /// Level of the node being restructured.
    pub level: Level,
    /// Inherited attribute at the node.
    pub h: &'a H,
    /// Already-transduced children: `{v ↦ (D', s)}`.
    pub arcs: BTreeMap<u64, (NodeId, S)>,
    /// Level memory in read (M2-3).
    pub memory: &'a MemoryLevel<M>,
    /// Input node being visited (key of the level memory).
    pub node_input: NodeId,
}

/// Attributed transduction (spec § 3.1): three partial functions of the
/// controller + initial element of the inherited channel + level memory `M`
/// (M2-3, T3). Input: pool with payload `bool` (set-theoretic reading);
/// output: pool with payload `Out`.
pub trait Transduction: Send + Sync {
    /// Inherited attribute (downward).
    type H: Clone + PartialEq + Eq + Hash + Debug + Send + Sync + Traceable;
    /// Synthesized attribute (upward).
    type S: Clone + Debug + Send + Sync + Traceable;
    /// Payload of the terminals of the output diagram (`Δ_out`, spec § 3.2).
    type Out: Clone + Eq + Hash + Debug + Send + Sync;
    /// Value stored in the level memory `M : (id(node), ℓ) ⇀ M`
    /// (M2-3). `()` for transductions without memory (M1).
    type M: Clone + Eq + Hash + Debug + Send + Sync + Traceable;

    /// Fingerprint of the operation (parameters included): enters the memo
    /// key `(op, node, h, versions of the declared fragments)` (L1/L3;
    /// the versions are per-fragment since M2-5, the #388 core
    /// remainder — see [`Transduction::read_fragments`]).
    fn fingerprint(&self) -> u64;

    /// Initial element of the inherited channel.
    fn h0(&self) -> Self::H;

    /// (T-NODE) dispatch: inherited attribute along arc `v` of level `ℓ`.
    fn phi_disp(&self, level: Level, h: &Self::H, arc: u64) -> Self::H;

    /// (T-NODE) synthesis: aggregation of the children's attributes. Each arc
    /// carries the **level of the child** (`None` if the child is a terminal):
    /// needed by the A4 expansion-counting (the levels skipped between `ℓ`
    /// and the child were collapsed, their cardinality enters the fold).
    fn phi_aggr(
        &self,
        level: Level,
        h: &Self::H,
        arcs: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S;

    /// (T-NODE) restructuring: result node interned in the output pool from
    /// the already-transduced children `{v ↦ (D', s)}` (A3), with
    /// level-memory read and production of a memory effect
    /// `μ` (M2-3, spec § 3.1: `φ_out(ℓ, h, {v ↦ (D', s)}, M) ↦ (n', μ)`).
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<Self::M>);

    /// (T-TERM): interned result terminal node + synthesized attribute.
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S);

    /// Fragments of the level memory the visit of a node of `level`
    /// READS (M2-5, the #388 core remainder): the per-fragment
    /// versions entering the memo key enter for these fragments only.
    /// Default: the fragment of the visited level itself — level-local
    /// scoping, the same contract discipline as the fingerprint (the
    /// op declares what makes its answers stale); a transduction
    /// reading across fragments declares them explicitly.
    fn read_fragments(&self, level: Level) -> Vec<u32> {
        vec![level.0]
    }
}

/// Runner of a transduction with its computed table (memo L1/L3) and its
/// level memory (M2-3).
pub struct Transducer<T: Transduction> {
    transduction: T,
    memo: HashMap<KeyMemo<T::H>, (NodeId, T::S)>,
    memory: MemoryLevel<T::M>,
    policy: MemoryPolicy,
    observer: Option<Arc<dyn Observer>>,
}

impl<T: Transduction> Transducer<T> {
    /// New runner for the given transduction, empty memory (version 0),
    /// policy `NoMemory` (M1: no level memory), no observer.
    pub fn new(transduction: T) -> Self {
        Self::with_policy(transduction, MemoryPolicy::NoMemory)
    }

    /// Runner with a level-memory storage policy
    /// declared by the controller (M2-3, T3).
    pub fn with_policy(transduction: T, policy: MemoryPolicy) -> Self {
        Transducer {
            transduction,
            memo: HashMap::new(),
            memory: MemoryLevel::new(),
            policy,
            observer: None,
        }
    }

    /// Runner with trace observer (M2-6): emits `MemoHit`/`MemoMiss`
    /// and the `Attr` of the three channels during execution.
    pub fn with_observer(
        transduction: T,
        policy: MemoryPolicy,
        observer: Arc<dyn Observer>,
    ) -> Self {
        Transducer {
            transduction,
            memo: HashMap::new(),
            memory: MemoryLevel::new(),
            policy,
            observer: Some(observer),
        }
    }

    /// Emits an event on the observer, if there is one. D1-1: the event
    /// is contextualized by the innermost open span.
    fn notifier(&self, event: TraceEvent) {
        if let Some(observer) = &self.observer {
            observer.on_event(&contextualize(event));
        }
    }

    /// Opens a span of the given kind and emits its `SpanOpen` event (D1-1).
    fn open_span(&self, kind: SpanKind, lane: u32) -> crate::observer::SpanId {
        let span = next_span_id();
        push_span(span);
        self.notifier(TraceEvent::SpanOpen { span, kind, lane });
        span
    }

    /// Closes a span and emits its `SpanClose` event (D1-1).
    fn close_span(&self, span: crate::observer::SpanId) {
        self.notifier(TraceEvent::SpanClose { span });
        pop_span();
    }

    /// Access to the embedded transduction (to inspect the state of a
    /// wrapper, e.g. levels modified by `RestrictedToLevels`).
    pub fn transduction(&self) -> &T {
        &self.transduction
    }

    /// Replaces the embedded transduction, KEEPING the memo and the
    /// level memory (M2-5, the #388 core remainder): the saturate loop
    /// swaps its per-round wrapper for the next round while the
    /// computed table survives — the validity of the surviving entries
    /// is carried by the fingerprint and the per-fragment versions of
    /// the memo key, not by the identity of the transduction value.
    pub fn set_transduction(&mut self, transduction: T) {
        self.transduction = transduction;
    }

    /// Number of memo entries (for tests: proof of hits).
    pub fn size_memo(&self) -> usize {
        self.memo.len()
    }

    /// Current version of the level memory (L1).
    pub fn version_memory(&self) -> u64 {
        self.memory.version()
    }

    /// Access to the level memory (for tests: produced entries).
    pub fn memory(&self) -> &MemoryLevel<T::M> {
        &self.memory
    }

    /// Runs the transduction at the root, with initial attribute `h0`.
    /// The input is read from `pool_input` (payload `bool`), the result is
    /// written into `pool_output` (payload `Out`).
    pub fn run<PIn, POut>(
        &mut self,
        pool_input: &PIn,
        pool_output: &mut POut,
        anchor: Anchor,
        node: NodeId,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        PIn: Pool<Key = u64, Payload = bool>,
        POut: Pool<Key = u64, Payload = T::Out>,
    {
        let h = self.transduction.h0();
        let span = self.open_span(SpanKind::Op, 0);
        self.notifier(TraceEvent::OpStart {
            fingerprint: self.transduction.fingerprint(),
            span: None,
        });
        let result = self.run_with(pool_input, pool_output, anchor, node, h);
        self.notifier(TraceEvent::OpEnd { span: None });
        self.close_span(span);
        result
    }

    /// Single-pool variant: reads and writes in the same pool (payload `Out`
    /// = `bool`). Used by `saturate` (M2-4) where the current iteration lives
    /// in the same pool as the result.
    pub fn run_same_pool<P>(
        &mut self,
        pool: &mut P,
        anchor: Anchor,
        node: NodeId,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        P: Pool<Key = u64, Payload = bool>,
        T: Transduction<Out = bool>,
    {
        let h = self.transduction.h0();
        let span = self.open_span(SpanKind::Op, 0);
        self.notifier(TraceEvent::OpStart {
            fingerprint: self.transduction.fingerprint(),
            span: None,
        });
        let result = self.run_with_same_pool(pool, anchor, node, h);
        self.notifier(TraceEvent::OpEnd { span: None });
        self.close_span(span);
        result
    }

    fn run_with<PIn, POut>(
        &mut self,
        pool_input: &PIn,
        pool_output: &mut POut,
        anchor: Anchor,
        node: NodeId,
        h: T::H,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        PIn: Pool<Key = u64, Payload = bool>,
        POut: Pool<Key = u64, Payload = T::Out>,
    {
        // D1-1/D1-2: one Visit span per node visit, closed on every exit path.
        let span = self.open_span(SpanKind::Visit, 0);
        self.notifier(TraceEvent::VisitEntry { node, span: None });
        let result = self.run_with_inner(pool_input, pool_output, anchor, node, h);
        if let Ok((result_node, _)) = &result {
            self.notifier(TraceEvent::VisitExit {
                node,
                result: *result_node,
                span: None,
            });
        }
        self.close_span(span);
        result
    }

    fn run_with_inner<PIn, POut>(
        &mut self,
        pool_input: &PIn,
        pool_output: &mut POut,
        anchor: Anchor,
        node: NodeId,
        h: T::H,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        PIn: Pool<Key = u64, Payload = bool>,
        POut: Pool<Key = u64, Payload = T::Out>,
    {
        // MEM-15: the co-located memory decodes NodeIds with the pool's
        // class width.
        self.memory.set_class_bits(pool_input.class_bits());
        // M2-5 (the #388 core remainder): the level of the visit probes
        // the input BEFORE the memo check — the fragments the visit
        // declares to read give the versions of the memo key (a
        // terminal visit is pure in (payload, h), no fragment read:
        // empty versions).
        let level_visit = match pool_input.resolve(node) {
            Ok(Form::Node { level, .. }) => Some(level),
            Ok(Form::Terminal(_)) => None,
            Err(error) => return Err(error.into()),
        };
        let versions: Vec<u64> = match level_visit {
            Some(level) => BTreeSet::from_iter(self.transduction.read_fragments(level))
                .into_iter()
                .map(|fragment| self.memory.version_of(fragment))
                .collect(),
            None => Vec::new(),
        };
        let key: KeyMemo<T::H> = (self.transduction.fingerprint(), node, h.clone(), versions);
        let fingerprint = self.transduction.fingerprint();
        let version = self.memory.version();
        // D1-3: the attribute is a lazy TraceVal — no format! on the hot path.
        let h_val = h.trace_val();
        if let Some((result, s)) = self.memo.get(&key) {
            self.notifier(TraceEvent::MemoConsult {
                fingerprint,
                node,
                h: h_val.clone(),
                version,
                hit: true,
                span: None,
            });
            self.notifier(TraceEvent::MemoHit { node, span: None });
            return Ok((*result, s.clone()));
        }
        self.notifier(TraceEvent::MemoConsult {
            fingerprint,
            node,
            h: h_val.clone(),
            version,
            hit: false,
            span: None,
        });
        self.notifier(TraceEvent::MemoMiss { node, span: None });
        let form = pool_input.resolve(node)?.clone();
        // D1-2: structural summary of the input form, for the StructureChange.
        let structure_before = extract_structure(&form);
        let (result, s) = match form {
            Form::Terminal(payload) => self.transduction.psi(pool_output, anchor, payload, &h),
            Form::Node { level, arcs } => {
                let mut children = BTreeMap::new();
                let mut s_arcs = BTreeMap::new();
                for (arc, target) in arcs {
                    let h_arc = self.transduction.phi_disp(level, &h, arc);
                    self.notifier(TraceEvent::Attr {
                        channel: ChannelAttr::Inherited,
                        node,
                        value: h_arc.trace_val(),
                        span: None,
                    });
                    let (target_result, s_arc) =
                        self.run_with(pool_input, pool_output, anchor, target, h_arc)?;
                    let level_target = match pool_input.resolve(target) {
                        Ok(Form::Node { level, .. }) => Some(level),
                        Ok(Form::Terminal(_)) => None,
                        Err(error) => return Err(error.into()),
                    };
                    children.insert(arc, (target_result, s_arc.clone()));
                    s_arcs.insert(arc, (level_target, s_arc));
                }
                let s = self.transduction.phi_aggr(level, &h, &s_arcs);
                self.notifier(TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    node,
                    value: s.trace_val(),
                    span: None,
                });
                let ctx = PhiOutContext {
                    anchor,
                    level,
                    h: &h,
                    arcs: children,
                    memory: &self.memory,
                    node_input: node,
                };
                let (node_result, effect) = self.transduction.phi_out(pool_output, ctx);
                if self.policy == MemoryPolicy::NoMemory && !effect.writes.is_empty() {
                    return Err(CoreError::MemoryDisallowed);
                }
                if !effect.writes.is_empty() {
                    self.notifier(TraceEvent::Attr {
                        channel: ChannelAttr::Memory,
                        node,
                        value: effect.writes.trace_val(),
                        span: None,
                    });
                    // D1-2: the memory write bumps the version.
                    let version_before = self.memory.version();
                    effect.apply(&mut self.memory);
                    self.notifier(TraceEvent::MemoryWrite {
                        version_before,
                        version_after: self.memory.version(),
                        span: None,
                    });
                } else {
                    effect.apply(&mut self.memory);
                }
                (node_result, s)
            }
        };
        // D1-2: generic structure change, by comparison of the forms.
        self.notifier_structure_change(pool_output, node, &structure_before, result);
        self.notifier(TraceEvent::MemoInsert {
            fingerprint,
            node,
            h: h_val,
            version,
            result,
            span: None,
        });
        self.memo.insert(key, (result, s.clone()));
        Ok((result, s))
    }

    /// Emits the `StructureChange` of one visit (D1-2): the output form is
    /// resolved in the output pool and compared with the input form.
    fn notifier_structure_change<POut>(
        &self,
        pool_output: &POut,
        node: NodeId,
        structure_before: &(Option<Level>, Vec<(u64, NodeId)>),
        result: NodeId,
    ) where
        POut: Pool<Key = u64>,
    {
        let Ok(form_after) = pool_output.resolve(result) else {
            return;
        };
        let structure_after = extract_structure(&form_after);
        let classification =
            classifier_structure(structure_before.clone(), structure_after.clone());
        self.notifier(TraceEvent::StructureChange {
            node,
            classification,
            arcs_before: structure_before.1.clone(),
            arcs_after: structure_after.1,
            span: None,
        });
    }

    fn run_with_same_pool<P>(
        &mut self,
        pool: &mut P,
        anchor: Anchor,
        node: NodeId,
        h: T::H,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        P: Pool<Key = u64, Payload = bool>,
        T: Transduction<Out = bool>,
    {
        // D1-1/D1-2: one Visit span per node visit, closed on every exit path.
        let span = self.open_span(SpanKind::Visit, 0);
        self.notifier(TraceEvent::VisitEntry { node, span: None });
        let result = self.run_with_same_pool_inner(pool, anchor, node, h);
        if let Ok((result_node, _)) = &result {
            self.notifier(TraceEvent::VisitExit {
                node,
                result: *result_node,
                span: None,
            });
        }
        self.close_span(span);
        result
    }

    fn run_with_same_pool_inner<P>(
        &mut self,
        pool: &mut P,
        anchor: Anchor,
        node: NodeId,
        h: T::H,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        P: Pool<Key = u64, Payload = bool>,
        T: Transduction<Out = bool>,
    {
        // MEM-15: the co-located memory decodes NodeIds with the pool's
        // class width.
        self.memory.set_class_bits(pool.class_bits());
        // M2-5 (the #388 core remainder): the level of the visit probes
        // the input BEFORE the memo check — the fragments the visit
        // declares to read give the versions of the memo key (a
        // terminal visit is pure in (payload, h), no fragment read:
        // empty versions).
        let level_visit = match pool.resolve(node) {
            Ok(Form::Node { level, .. }) => Some(level),
            Ok(Form::Terminal(_)) => None,
            Err(error) => return Err(error.into()),
        };
        let versions: Vec<u64> = match level_visit {
            Some(level) => BTreeSet::from_iter(self.transduction.read_fragments(level))
                .into_iter()
                .map(|fragment| self.memory.version_of(fragment))
                .collect(),
            None => Vec::new(),
        };
        let key: KeyMemo<T::H> = (self.transduction.fingerprint(), node, h.clone(), versions);
        let fingerprint = self.transduction.fingerprint();
        let version = self.memory.version();
        // D1-3: the attribute is a lazy TraceVal — no format! on the hot path.
        let h_val = h.trace_val();
        if let Some((result, s)) = self.memo.get(&key) {
            self.notifier(TraceEvent::MemoConsult {
                fingerprint,
                node,
                h: h_val.clone(),
                version,
                hit: true,
                span: None,
            });
            self.notifier(TraceEvent::MemoHit { node, span: None });
            return Ok((*result, s.clone()));
        }
        self.notifier(TraceEvent::MemoConsult {
            fingerprint,
            node,
            h: h_val.clone(),
            version,
            hit: false,
            span: None,
        });
        self.notifier(TraceEvent::MemoMiss { node, span: None });
        let form = pool.resolve(node)?.clone();
        // D1-2: structural summary of the input form, for the StructureChange.
        let structure_before = extract_structure(&form);
        let (result, s) = match form {
            Form::Terminal(payload) => self.transduction.psi(pool, anchor, payload, &h),
            Form::Node { level, arcs } => {
                let mut children = BTreeMap::new();
                let mut s_arcs = BTreeMap::new();
                for (arc, target) in arcs {
                    let h_arc = self.transduction.phi_disp(level, &h, arc);
                    self.notifier(TraceEvent::Attr {
                        channel: ChannelAttr::Inherited,
                        node,
                        value: h_arc.trace_val(),
                        span: None,
                    });
                    let (target_result, s_arc) =
                        self.run_with_same_pool(pool, anchor, target, h_arc)?;
                    let level_target = match pool.resolve(target) {
                        Ok(Form::Node { level, .. }) => Some(level),
                        Ok(Form::Terminal(_)) => None,
                        Err(error) => return Err(error.into()),
                    };
                    children.insert(arc, (target_result, s_arc.clone()));
                    s_arcs.insert(arc, (level_target, s_arc));
                }
                let s = self.transduction.phi_aggr(level, &h, &s_arcs);
                self.notifier(TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    node,
                    value: s.trace_val(),
                    span: None,
                });
                let ctx = PhiOutContext {
                    anchor,
                    level,
                    h: &h,
                    arcs: children,
                    memory: &self.memory,
                    node_input: node,
                };
                let (node_result, effect) = self.transduction.phi_out(pool, ctx);
                if self.policy == MemoryPolicy::NoMemory && !effect.writes.is_empty() {
                    return Err(CoreError::MemoryDisallowed);
                }
                if !effect.writes.is_empty() {
                    self.notifier(TraceEvent::Attr {
                        channel: ChannelAttr::Memory,
                        node,
                        value: effect.writes.trace_val(),
                        span: None,
                    });
                    // D1-2: the memory write bumps the version.
                    let version_before = self.memory.version();
                    effect.apply(&mut self.memory);
                    self.notifier(TraceEvent::MemoryWrite {
                        version_before,
                        version_after: self.memory.version(),
                        span: None,
                    });
                } else {
                    effect.apply(&mut self.memory);
                }
                (node_result, s)
            }
        };
        // D1-2: generic structure change, by comparison of the forms.
        self.notifier_structure_change(pool, node, &structure_before, result);
        self.notifier(TraceEvent::MemoInsert {
            fingerprint,
            node,
            h: h_val,
            version,
            result,
            span: None,
        });
        self.memo.insert(key, (result, s.clone()));
        Ok((result, s))
    }
}
