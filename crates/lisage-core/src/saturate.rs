//! `saturate` — increasing fixpoint of a step transduction (M2-4,
//! spec § 7, statements T2/T3) : `saturate(τ, D) = νX. D ⊕ τ(X, h₀)`.
//!
//! Two saturation policies, two instances of the same fixpoint semantics
//! (same final result, different iteration strategy):
//! - [`PolicySaturation::Static`] (Ciardo-style): `τ` is only applied
//!   at the declared levels;
//! - [`PolicySaturation::AutoDetected`] (Hamez-style): all levels are
//!   active at first; a level whose current iteration changed nothing is
//!   removed from the active set.
//!
//! The construction merge `⊕` is the idempotent union (Bool, A5).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::fold::union;
use crate::observer::{
    Observer, SpanKind, TraceEvent, contextualize, next_span_id, pop_span, push_span,
};
use crate::schema::SchemaKeys;
use crate::transduction::{PhiOutContext, Transducer, Transduction};

/// Safety bound of the fixpoint loop (T2 guarantees termination on a
/// bounded schema; the bound is only there to detect a regression).
pub const BOUND_ITERATIONS: u32 = 1_000;

/// Fingerprint salt of the [`RestrictedToLevels`] wrapper: domain-separates
/// the wrapper's fingerprint from that of the inner transduction.
const SEL_FINGERPRINT_WRAPPER: u64 = 0x5A7A_0000_0000_0000;

/// Saturation policy (M2-4): two instances of the same fixpoint semantics.
/// Both policies produce the same fixpoint when `Static` declares all
/// levels; only the iteration strategy differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicySaturation {
    /// Static by level (Ciardo-style): only the listed levels are saturated
    /// at each iteration.
    Static(Vec<Level>),
    /// Auto-detected (Hamez-style): all levels are active at first; a level
    /// is removed as soon as an iteration changes nothing at that level.
    AutoDetected,
}

/// Parameters of a saturation (record, ADR-0003).
pub struct SaturationParameters<'a, T, P> {
    /// Step transduction `τ` (cloneable for the iterations).
    pub transduction: &'a T,
    /// Saturation policy.
    pub policy: &'a PolicySaturation,
    /// Single pool (read + write, payload `bool`): the current iteration and
    /// the result live in the same pool.
    pub pool: &'a mut P,
    /// Key algebra of the schema (for the union).
    pub keys: &'a SchemaKeys,
    /// Anchor of the root.
    pub anchor: Anchor,
    /// Initial node `D`.
    pub initial: NodeId,
    /// Number of levels of the schema (for AutoDetected: all levels).
    pub number_levels: u32,
    /// Trace observer (M2-6): receives one `Round` per iteration and the
    /// events of the step runner. `None` = no trace.
    pub observer: Option<Arc<dyn Observer>>,
}

/// Result of a saturation: fixpoint node + iteration count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultSaturation {
    /// Fixpoint node reached.
    pub node: NodeId,
    /// Number of iterations of the fixpoint loop.
    pub iterations: u32,
}

/// Wrapper transduction that restricts a step transduction to a set of
/// active levels: at active levels, it delegates to the inner transduction
/// and records the modified levels; at the others, identity copy.
pub struct RestrictedToLevels<T> {
    inner: T,
    active_levels: Vec<Level>,
    modified_levels: Mutex<BTreeSet<Level>>,
}

impl<T: Clone> Clone for RestrictedToLevels<T> {
    fn clone(&self) -> Self {
        // The clone starts from a fresh tracking: `saturate` creates a new
        // wrapper at each iteration anyway.
        RestrictedToLevels {
            inner: self.inner.clone(),
            active_levels: self.active_levels.clone(),
            modified_levels: Mutex::new(BTreeSet::new()),
        }
    }
}

impl<T> RestrictedToLevels<T> {
    /// New wrapper: `inner` restricted to the `active_levels`.
    pub fn new(inner: T, active_levels: Vec<Level>) -> Self {
        RestrictedToLevels {
            inner,
            active_levels,
            modified_levels: Mutex::new(BTreeSet::new()),
        }
    }

    /// Levels at which `φ_out` produced a node different from the input
    /// since the wrapper was created (AutoDetected policy).
    pub fn modified_levels(&self) -> BTreeSet<Level> {
        self.modified_levels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl<T: Transduction> Transduction for RestrictedToLevels<T> {
    type H = T::H;
    type S = T::S;
    type Out = T::Out;
    type M = T::M;

    fn fingerprint(&self) -> u64 {
        self.inner.fingerprint() ^ SEL_FINGERPRINT_WRAPPER
    }
    fn h0(&self) -> Self::H {
        self.inner.h0()
    }
    fn phi_disp(&self, level: Level, h: &Self::H, arc: u64) -> Self::H {
        self.inner.phi_disp(level, h, arc)
    }
    fn phi_aggr(
        &self,
        level: Level,
        h: &Self::H,
        arcs: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S {
        self.inner.phi_aggr(level, h, arcs)
    }
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
        if self.active_levels.contains(&ctx.level) {
            let node_input = ctx.node_input;
            let level = ctx.level;
            let (node, effect) = self.inner.phi_out(pool, ctx);
            if node != node_input {
                self.modified_levels
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(level);
            }
            (node, effect)
        } else {
            let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
            (
                pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: ctx.level,
                        arcs,
                    },
                ),
                crate::memory::EffectMemory::new(),
            )
        }
    }
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S) {
        self.inner.psi(pool, anchor, payload, h)
    }
}

/// Increasing fixpoint `saturate(τ, D)` on the pool (spec § 7, T2):
/// iterates `X ← X ⊕ τ(X)` until stabilization, applying `τ` according to
/// the policy (restricted to the active levels; AutoDetected removes the
/// stabilized levels from the active set).
///
/// Termination (T2) is guaranteed on a bounded schema: the set of canonical
/// forms is finite and the chain is increasing for denotation inclusion.
/// [`BOUND_ITERATIONS`] is only a safety net against a regression.
pub fn saturate<T, P>(
    params: &mut SaturationParameters<'_, T, P>,
) -> Result<ResultSaturation, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    // Active levels according to the policy.
    let mut active_levels: Vec<Level> = match params.policy {
        PolicySaturation::Static(levels) => levels.clone(),
        PolicySaturation::AutoDetected => (0..params.number_levels).map(Level).collect(),
    };

    let mut courant = params.initial;
    let mut iterations = 0u32;
    loop {
        iterations += 1;
        // D1-1: one Round span per iteration, closed on every exit path.
        let span = next_span_id();
        push_span(span);
        if iterations > BOUND_ITERATIONS {
            close_span(&params.observer, span);
            return Err(CoreError::NonTermination {
                bound: BOUND_ITERATIONS,
            });
        }
        if let Some(observer) = &params.observer {
            observer.on_event(&TraceEvent::SpanOpen {
                span,
                kind: SpanKind::Round,
                lane: 0,
            });
        }
        let pool_before = params.pool.node_count() as i64;
        let step = RestrictedToLevels::new(params.transduction.clone(), active_levels.clone());
        // Spec § 7: saturate steps can produce a memory effect
        // (T3, M2-5) — PerLevel policy. The memory lives for one pass:
        // the runner (and its memory) is recreated at each iteration.
        let mut runner = match &params.observer {
            Some(observer) => Transducer::with_observer(
                step,
                lisage_algebra::interface::MemoryPolicy::PerLevel,
                Arc::clone(observer),
            ),
            None => {
                Transducer::with_policy(step, lisage_algebra::interface::MemoryPolicy::PerLevel)
            }
        };
        let result = runner.run_same_pool(params.pool, params.anchor, courant);
        let result = match result {
            Ok(value) => value,
            Err(error) => {
                close_span(&params.observer, span);
                return Err(error);
            }
        };
        let (image, _) = result;
        let next = match union(params.pool, params.keys, params.anchor, courant, image) {
            Ok(value) => value,
            Err(error) => {
                close_span(&params.observer, span);
                return Err(error);
            }
        };
        // D1-2: the Round event carries the active/modified levels, the
        // before/image/after handles and the growth of the pool.
        if let Some(observer) = &params.observer {
            let modified_levels: Vec<u32> = runner
                .transduction()
                .modified_levels()
                .iter()
                .map(|level| level.0)
                .collect();
            observer.on_event(&contextualize(TraceEvent::Round {
                number: iterations,
                active_levels: active_levels.iter().map(|level| level.0).collect(),
                modified_levels,
                before: courant,
                image,
                after: next,
                growth: params.pool.node_count() as i64 - pool_before,
                span: None,
            }));
        }
        if next == courant {
            close_span(&params.observer, span);
            return Ok(ResultSaturation {
                node: courant,
                iterations,
            });
        }
        if matches!(params.policy, PolicySaturation::AutoDetected) {
            // Hamez policy: a level not modified by the iteration is removed
            // from the active set.
            let modified = runner.transduction().modified_levels();
            active_levels.retain(|level| modified.contains(level));
            if active_levels.is_empty() {
                close_span(&params.observer, span);
                return Ok(ResultSaturation {
                    node: next,
                    iterations,
                });
            }
        }
        close_span(&params.observer, span);
        courant = next;
    }
}

/// Closes a span and emits its `SpanClose` event on the optional observer (D1-1).
fn close_span(observer: &Option<Arc<dyn Observer>>, span: crate::observer::SpanId) {
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::SpanClose { span });
    }
    pop_span();
}
