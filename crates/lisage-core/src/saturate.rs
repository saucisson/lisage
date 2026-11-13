//! `saturate` — increasing fixpoint of a step transduction (M2-4,
//! spec § 7, statements T2/T3) : `saturate(τ, D) = νX. D ⊕ τ(X, h₀)`.
//!
//! Two saturation policies, two instances of the same fixpoint semantics
//! (same final result, different iteration strategy):
//! - [`PolicySaturation::Static`] (Ciardo-style): `τ` is only applied
//!   at the declared levels;
//! - [`PolicySaturation::AutoDetectee`] (Hamez-style): all levels
//!   are active at first; a level whose current iteration changed nothing
//!   is removed from the active set.
//!
//! The construction merge `⊕` is the idempotent union (Bool, A5).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::fold::union;
use crate::observer::{Observer, TraceEvent};
use crate::schema::SchemaKeys;
use crate::transduction::{ContextPhiOut, Transducer, Transduction};

/// Safety bound of the fixpoint loop (T2 guarantees termination on a
/// bounded schema; the bound is only there to detect a regression).
pub const BOUND_ITERATIONS: u32 = 1_000;

/// Salt of the [`RestrictedAuxLevels`] wrapper fingerprint: separates
/// the wrapper fingerprint from that of the inner transduction.
const SEL_FINGERPRINT_WRAPPER: u64 = 0x5A7A_0000_0000_0000;

/// Saturation policy (M2-4): two instances of the same fixpoint semantics.
/// The two policies produce the same fixpoint when `Static` declares all
/// levels; only the iteration strategy differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicySaturation {
    /// Static by level (Ciardo-style): only the listed levels are
    /// saturated at each iteration.
    Static(Vec<Level>),
    /// Auto-detected (Hamez-style): all levels are active at first;
    /// a level is removed as soon as an iteration changes nothing at that level.
    AutoDetectee,
}

/// Parameters of a saturation (record, ADR-0003).
pub struct ParametersSaturation<'a, T, P> {
    /// Step transduction `τ` (cloneable for the iterations).
    pub transduction: &'a T,
    /// Saturation policy.
    pub policy: &'a PolicySaturation,
    /// Single pool (read + write, payload `bool`): the current iteration
    /// and the result live in the same pool.
    pub pool: &'a mut P,
    /// Key algebra of the schema (for the union).
    pub keys: &'a SchemaKeys,
    /// Anchor of the root.
    pub anchor: Anchor,
    /// Initial node `D`.
    pub initial: NodeId,
    /// Number of levels of the schema (for AutoDetectee: all levels).
    pub number_levels: u32,
    /// Trace observer (M2-6): receives one `Round` per iteration and the
    /// step runner events. `None` = no trace.
    pub observer: Option<Arc<dyn Observer>>,
}

/// Result of a saturation: fixpoint node + number of iterations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultSaturation {
    /// Fixpoint node reached.
    pub node: NodeId,
    /// Number of iterations of the fixpoint loop.
    pub iterations: u32,
}

/// Transduction wrapper that restricts a step transduction to a set of
/// active levels: at active levels, it delegates to the inner transduction
/// and records the modified levels; at the others, identity copy.
pub struct RestrictedAuxLevels<T> {
    inner: T,
    levels_active: Vec<Level>,
    levels_modified: Mutex<BTreeSet<Level>>,
}

impl<T: Clone> Clone for RestrictedAuxLevels<T> {
    fn clone(&self) -> Self {
        // The clone starts from a fresh tracking: `saturer` creates a new
        // wrapper at each iteration anyway.
        RestrictedAuxLevels {
            inner: self.inner.clone(),
            levels_active: self.levels_active.clone(),
            levels_modified: Mutex::new(BTreeSet::new()),
        }
    }
}

impl<T> RestrictedAuxLevels<T> {
    /// New wrapper: `inner` restricted to `levels_active`.
    pub fn new(inner: T, levels_active: Vec<Level>) -> Self {
        RestrictedAuxLevels {
            inner,
            levels_active,
            levels_modified: Mutex::new(BTreeSet::new()),
        }
    }

    /// Levels at which `φ_out` produced a node different from the input
    /// since the wrapper was created (AutoDetectee policy).
    pub fn levels_modified(&self) -> BTreeSet<Level> {
        self.levels_modified
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl<T: Transduction> Transduction for RestrictedAuxLevels<T> {
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
    fn phi_dispatch(&self, level: Level, h: &Self::H, edge: u64) -> Self::H {
        self.inner.phi_dispatch(level, h, edge)
    }
    fn phi_aggregate(
        &self,
        level: Level,
        h: &Self::H,
        edges: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S {
        self.inner.phi_aggregate(level, h, edges)
    }
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
        if self.levels_active.contains(&ctx.level) {
            let node_input = ctx.node_input;
            let level = ctx.level;
            let (node, effect) = self.inner.phi_out(pool, ctx);
            if node != node_input {
                self.levels_modified
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(level);
            }
            (node, effect)
        } else {
            let edges = ctx.edges.into_iter().map(|(v, (n, _))| (v, n)).collect();
            (
                pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: ctx.level,
                        edges,
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
/// the policy (restricted to the active levels; AutoDetectee removes the
/// stabilized levels from the active set).
///
/// Termination (T2) is guaranteed on a bounded schema: the set of
/// canonical forms is finite and the chain is increasing for the inclusion
/// of denotations. [`BOUND_ITERATIONS`] is only a safety net against a
/// regression.
pub fn saturer<T, P>(
    params: &mut ParametersSaturation<'_, T, P>,
) -> Result<ResultSaturation, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    // Active levels according to the policy.
    let mut levels_active: Vec<Level> = match params.policy {
        PolicySaturation::Static(levels) => levels.clone(),
        PolicySaturation::AutoDetectee => (0..params.number_levels).map(Level).collect(),
    };

    let mut current = params.initial;
    let mut iterations = 0u32;
    loop {
        iterations += 1;
        if iterations > BOUND_ITERATIONS {
            return Err(CoreError::NotTerminaison {
                bound: BOUND_ITERATIONS,
            });
        }
        if let Some(observer) = &params.observer {
            observer.on_event(&TraceEvent::Round { number: iterations });
        }
        let step = RestrictedAuxLevels::new(params.transduction.clone(), levels_active.clone());
        // Spec § 7: saturate steps can produce a memory effect
        // (T3, M2-5) — ByLevel policy. The memory lives for one
        // pass: the runner (and its memory) is recreated at each iteration.
        let mut runner = match &params.observer {
            Some(observer) => Transducer::with_observer(
                step,
                lisage_algebra::interface::PolicyMemory::ByLevel,
                Arc::clone(observer),
            ),
            None => Transducer::with_policy(step, lisage_algebra::interface::PolicyMemory::ByLevel),
        };
        let (image, _) = runner.run_same_pool(params.pool, params.anchor, current)?;
        let next = union(params.pool, params.keys, params.anchor, current, image)?;
        if next == current {
            return Ok(ResultSaturation {
                node: current,
                iterations,
            });
        }
        if matches!(params.policy, PolicySaturation::AutoDetectee) {
            // Hamez policy: a level not modified by the iteration is
            // removed from the active set.
            let modified = runner.transduction().levels_modified();
            levels_active.retain(|level| modified.contains(level));
            if levels_active.is_empty() {
                return Ok(ResultSaturation {
                    node: next,
                    iterations,
                });
            }
        }
        current = next;
    }
}
