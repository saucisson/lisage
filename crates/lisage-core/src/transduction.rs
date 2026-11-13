//! Attributed transduction engine (M1-4, spec § 3.1, statement L3):
//! the two channels — **inherited** attribute `H` (top-down, `φ_disp`) and
//! **synthesized** `S` (bottom-up, `φ_aggr`) — and the fingerprint memo
//! `(op, node, h)` (lemma L1). Rules T-TERM/T-NODE.
//!
//! Since M1-5 (#12), the transduction reads its input from a pool with
//! payload `bool` and writes to a pool with any payload `Out`
//! (`Δ_out` may differ from `Δ_in`, spec § 3.2): `psi` and `φ_out` write
//! themselves (spec: "φ_out is the only rule that writes the diagram"),
//! always by interning (A3: canonical result by construction).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, PolicyMemory, Pool};

use crate::error::CoreError;
use crate::memory::{EffectMemory, MemoryLevel};
use crate::observer::{ChannelAttribute, Observer, TraceEvent};

/// Memo key: (fingerprint of the operation, node, inherited attribute,
/// level memory version) — lemma L1.
type KeyMemo<H> = (u64, NodeId, H, u64);

/// Context of `φ_out` (T-NODE): groups the parameters of the
/// restructuration (spec § 3.1 : `φ_out(ℓ, h, {v ↦ (D', s)}, M)`).
pub struct ContextPhiOut<'a, H, S, M: Clone + Eq + Hash + Debug> {
    /// Anchor of the root.
    pub anchor: Anchor,
    /// Level of the node being restructured.
    pub level: Level,
    /// Inherited attribute at the node.
    pub h: &'a H,
    /// Already transduced children: `{v ↦ (D', s)}`.
    pub edges: BTreeMap<u64, (NodeId, S)>,
    /// Level memory in read (M2-3).
    pub memory: &'a MemoryLevel<M>,
    /// Input node being visitd (key of the level memory).
    pub node_input: NodeId,
}

/// Attributed transduction (spec § 3.1): three partial functions of the
/// controller + initial element of the inherited channel + level memory `M`
/// (M2-3, T3). Input: pool with payload `bool` (set reading);
/// output: pool with payload `Out`.
pub trait Transduction: Send + Sync {
    /// Inherited attribute (top-down).
    type H: Clone + PartialEq + Eq + Hash + Debug + Send + Sync;
    /// Synthesized attribute (bottom-up).
    type S: Clone + Debug + Send + Sync;
    /// Payload of the terminals of the output diagram (`Δ_out`, spec § 3.2).
    type Out: Clone + Eq + Hash + Debug + Send + Sync;
    /// Value stored in the level memory `M : (id(node), ℓ) ⇀ M`
    /// (M2-3). `()` for transductions without memory (M1).
    type M: Clone + Eq + Hash + Debug + Send + Sync;

    /// Fingerprint of the operation (parameters included): enters the
    /// memo key `(op, node, h, version(M))` (L1/L3).
    fn fingerprint(&self) -> u64;

    /// Initial element of the inherited channel.
    fn h0(&self) -> Self::H;

    /// (T-NODE) dispatch: inherited attribute along edge `v` of level `ℓ`.
    fn phi_dispatch(&self, level: Level, h: &Self::H, edge: u64) -> Self::H;

    /// (T-NODE) synthesis: aggregation of the children attributes. Each edge
    /// carries the **level of the child** (`None` if the child is a terminal):
    /// needed for the A4 counting expansion (the levels skipped between `ℓ`
    /// and the child have been collapsed, their cardinality enters the fold).
    fn phi_aggregate(
        &self,
        level: Level,
        h: &Self::H,
        edges: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S;

    /// (T-NODE) restructuring: result node interned in the output pool
    /// from the already transduced children `{v ↦ (D', s)}` (A3), with
    /// reading of the level memory and production of a memory effect
    /// `μ` (M2-3, spec § 3.1 : `φ_out(ℓ, h, {v ↦ (D', s)}, M) ↦ (n', μ)`).
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<Self::M>);

    /// (T-TERM): interned result terminal node + synthesized attribute.
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S);
}

/// Runner of a transduction with its computed table (memo L1/L3) and its
/// level memory (M2-3).
pub struct Transducer<T: Transduction> {
    transduction: T,
    memo: HashMap<KeyMemo<T::H>, (NodeId, T::S)>,
    memory: MemoryLevel<T::M>,
    policy: PolicyMemory,
    observer: Option<Arc<dyn Observer>>,
}

impl<T: Transduction> Transducer<T> {
    /// New runner for the given transduction, empty memory (version 0),
    /// policy `No` (M1: no level memory), without observer.
    pub fn new(transduction: T) -> Self {
        Self::with_policy(transduction, PolicyMemory::No)
    }

    /// Runner with a level memory storage policy
    /// declared by the controller (M2-3, T3).
    pub fn with_policy(transduction: T, policy: PolicyMemory) -> Self {
        Transducer {
            transduction,
            memo: HashMap::new(),
            memory: MemoryLevel::new(),
            policy,
            observer: None,
        }
    }

    /// Runner with trace observer (M2-6): emits `MemoHit`/`MemoMiss`
    /// and the `Attribute`s of the three channels during execution.
    pub fn with_observer(
        transduction: T,
        policy: PolicyMemory,
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

    /// Emits an event on the observer, if there is one.
    fn notifier(&self, event: TraceEvent) {
        if let Some(observer) = &self.observer {
            observer.on_event(&event);
        }
    }

    /// Access to the embedded transduction (to inspect the state of a
    /// wrapper, e.g. levels modified by `RestrictedAuxLevels`).
    pub fn transduction(&self) -> &T {
        &self.transduction
    }

    /// Number of memo entries (for tests: proof of hits).
    pub fn size_memo(&self) -> usize {
        self.memo.len()
    }

    /// Current version of the level memory (L1).
    pub fn version_memory(&self) -> u64 {
        self.memory.version()
    }

    /// Access to the level memory (for tests: entries produced).
    pub fn memory(&self) -> &MemoryLevel<T::M> {
        &self.memory
    }

    /// Runs the transduction at the root, with initial attribute `h0`.
    /// The input is read from `pool_input` (payload `bool`), the result is
    /// written to `pool_output` (payload `Out`).
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
        self.run_with(pool_input, pool_output, anchor, node, h)
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
        self.run_with_same_pool(pool, anchor, node, h)
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
        let key = (
            self.transduction.fingerprint(),
            node,
            h.clone(),
            self.memory.version(),
        );
        if let Some((result, s)) = self.memo.get(&key) {
            self.notifier(TraceEvent::MemoHit { node });
            return Ok((*result, s.clone()));
        }
        self.notifier(TraceEvent::MemoMiss { node });
        let form = pool_input.resolve(node)?.clone();
        let (result, s) = match form {
            Form::Terminal(payload) => self.transduction.psi(pool_output, anchor, payload, &h),
            Form::Node { level, edges } => {
                let mut children = BTreeMap::new();
                let mut s_edges = BTreeMap::new();
                for (edge, target) in edges {
                    let h_edge = self.transduction.phi_dispatch(level, &h, edge);
                    self.notifier(TraceEvent::Attribute {
                        channel: ChannelAttribute::Inherited,
                        node,
                        value: format!("{h_edge:?}"),
                    });
                    let (target_result, s_edge) =
                        self.run_with(pool_input, pool_output, anchor, target, h_edge)?;
                    let level_target = match pool_input.resolve(target) {
                        Ok(Form::Node { level, .. }) => Some(*level),
                        Ok(Form::Terminal(_)) => None,
                        Err(error) => return Err(error.into()),
                    };
                    children.insert(edge, (target_result, s_edge.clone()));
                    s_edges.insert(edge, (level_target, s_edge));
                }
                let s = self.transduction.phi_aggregate(level, &h, &s_edges);
                self.notifier(TraceEvent::Attribute {
                    channel: ChannelAttribute::Synthesize,
                    node,
                    value: format!("{s:?}"),
                });
                let ctx = ContextPhiOut {
                    anchor,
                    level,
                    h: &h,
                    edges: children,
                    memory: &self.memory,
                    node_input: node,
                };
                let (node_result, effect) = self.transduction.phi_out(pool_output, ctx);
                if self.policy == PolicyMemory::No && !effect.ecritures.is_empty() {
                    return Err(CoreError::MemoryNotAutorisee);
                }
                if !effect.ecritures.is_empty() {
                    self.notifier(TraceEvent::Attribute {
                        channel: ChannelAttribute::Memory,
                        node,
                        value: format!("{:?}", effect.ecritures),
                    });
                }
                effect.apply(&mut self.memory);
                (node_result, s)
            }
        };
        self.memo.insert(key, (result, s.clone()));
        Ok((result, s))
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
        let key = (
            self.transduction.fingerprint(),
            node,
            h.clone(),
            self.memory.version(),
        );
        if let Some((result, s)) = self.memo.get(&key) {
            self.notifier(TraceEvent::MemoHit { node });
            return Ok((*result, s.clone()));
        }
        self.notifier(TraceEvent::MemoMiss { node });
        let form = pool.resolve(node)?.clone();
        let (result, s) = match form {
            Form::Terminal(payload) => self.transduction.psi(pool, anchor, payload, &h),
            Form::Node { level, edges } => {
                let mut children = BTreeMap::new();
                let mut s_edges = BTreeMap::new();
                for (edge, target) in edges {
                    let h_edge = self.transduction.phi_dispatch(level, &h, edge);
                    self.notifier(TraceEvent::Attribute {
                        channel: ChannelAttribute::Inherited,
                        node,
                        value: format!("{h_edge:?}"),
                    });
                    let (target_result, s_edge) =
                        self.run_with_same_pool(pool, anchor, target, h_edge)?;
                    let level_target = match pool.resolve(target) {
                        Ok(Form::Node { level, .. }) => Some(*level),
                        Ok(Form::Terminal(_)) => None,
                        Err(error) => return Err(error.into()),
                    };
                    children.insert(edge, (target_result, s_edge.clone()));
                    s_edges.insert(edge, (level_target, s_edge));
                }
                let s = self.transduction.phi_aggregate(level, &h, &s_edges);
                self.notifier(TraceEvent::Attribute {
                    channel: ChannelAttribute::Synthesize,
                    node,
                    value: format!("{s:?}"),
                });
                let ctx = ContextPhiOut {
                    anchor,
                    level,
                    h: &h,
                    edges: children,
                    memory: &self.memory,
                    node_input: node,
                };
                let (node_result, effect) = self.transduction.phi_out(pool, ctx);
                if self.policy == PolicyMemory::No && !effect.ecritures.is_empty() {
                    return Err(CoreError::MemoryNotAutorisee);
                }
                if !effect.ecritures.is_empty() {
                    self.notifier(TraceEvent::Attribute {
                        channel: ChannelAttribute::Memory,
                        node,
                        value: format!("{:?}", effect.ecritures),
                    });
                }
                effect.apply(&mut self.memory);
                (node_result, s)
            }
        };
        self.memo.insert(key, (result, s.clone()));
        Ok((result, s))
    }
}
