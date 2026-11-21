//! `saturate` — increasing fixpoint of a step transduction (M2-4,
//! spec § 7, statements T2/T3) : `saturate(τ, D) = νX. D ⊕ τ(X, h₀)`.
//!
//! Two saturation policies, two instances of the same fixpoint semantics
//! (same final result, different iteration strategy):
//! - [`PolicySaturation::Statique`] (Ciardo-style): `τ` is only applied
//!   at the declared levels;
//! - [`PolicySaturation::AutoDetectee`] (Hamez-style): all levels are
//!   active at first; a level whose current iteration changed nothing is
//!   removed from the active set.
//!
//! The construction merge `⊕` is the idempotent union (Bool, A5).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::observer::{
    Observer, SpanKind, TraceEvent, contextualiser, next_span_id, pop_span, push_span,
};
use crate::pli::union;
use crate::schema::SchemaKeys;
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Safety bound of the fixpoint loop (T2 guarantees termination on a
/// bounded schema; the bound is only there to detect a regression).
pub const BOUND_ITERATIONS: u32 = 1_000;

/// Fingerprint salt of the [`RestreintAuxLevels`] wrapper: domain-separates
/// the wrapper's fingerprint from that of the inner transduction.
const SEL_FINGERPRINT_WRAPPER: u64 = 0x5A7A_0000_0000_0000;

/// Saturation policy (M2-4): two instances of the same fixpoint semantics.
/// Both policies produce the same fixpoint when `Statique` declares all
/// levels; only the iteration strategy differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicySaturation {
    /// Static by level (Ciardo-style): only the listed levels are saturated
    /// at each iteration.
    Statique(Vec<Level>),
    /// Auto-detected (Hamez-style): all levels are active at first; a level
    /// is removed as soon as an iteration changes nothing at that level.
    AutoDetectee,
}

/// Parameters of a saturation (record, ADR-0003).
pub struct ParametresSaturation<'a, T, P> {
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
    /// Number of levels of the schema (for AutoDetectee: all levels).
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
pub struct RestreintAuxLevels<T> {
    inner: T,
    levels_actifs: Vec<Level>,
    levels_modifies: Mutex<BTreeSet<Level>>,
}

impl<T: Clone> Clone for RestreintAuxLevels<T> {
    fn clone(&self) -> Self {
        // The clone starts from a fresh tracking: `saturer` creates a new
        // wrapper at each iteration anyway.
        RestreintAuxLevels {
            inner: self.inner.clone(),
            levels_actifs: self.levels_actifs.clone(),
            levels_modifies: Mutex::new(BTreeSet::new()),
        }
    }
}

impl<T> RestreintAuxLevels<T> {
    /// New wrapper: `inner` restricted to the `levels_actifs`.
    pub fn new(inner: T, levels_actifs: Vec<Level>) -> Self {
        RestreintAuxLevels {
            inner,
            levels_actifs,
            levels_modifies: Mutex::new(BTreeSet::new()),
        }
    }

    /// Levels at which `φ_out` produced a node different from the input
    /// since the wrapper was created (AutoDetectee policy).
    pub fn levels_modifies(&self) -> BTreeSet<Level> {
        self.levels_modifies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl<T: Transduction> Transduction for RestreintAuxLevels<T> {
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
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffetMemory<Self::M>) {
        if self.levels_actifs.contains(&ctx.level) {
            let node_input = ctx.node_input;
            let level = ctx.level;
            let (node, effet) = self.inner.phi_out(pool, ctx);
            if node != node_input {
                self.levels_modifies
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(level);
            }
            (node, effet)
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
                crate::memory::EffetMemory::new(),
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
/// Termination (T2) is guaranteed on a bounded schema: the set of canonical
/// forms is finite and the chain is increasing for denotation inclusion.
/// [`BOUND_ITERATIONS`] is only a safety net against a regression.
pub fn saturer<T, P>(
    params: &mut ParametresSaturation<'_, T, P>,
) -> Result<ResultSaturation, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    // Active levels according to the policy.
    let mut levels_actifs: Vec<Level> = match params.policy {
        PolicySaturation::Statique(levels) => levels.clone(),
        PolicySaturation::AutoDetectee => (0..params.number_levels).map(Level).collect(),
    };

    let mut courant = params.initial;
    let mut iterations = 0u32;
    loop {
        iterations += 1;
        // D1-1: one Round span per iteration, closed on every exit path.
        let span = next_span_id();
        push_span(span);
        if iterations > BOUND_ITERATIONS {
            fermer_span(&params.observer, span);
            return Err(CoreError::NonTerminaison {
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
        let pool_avant = params.pool.node_count() as i64;
        let etape = RestreintAuxLevels::new(params.transduction.clone(), levels_actifs.clone());
        // Spec § 7: saturate steps can produce a memory effect
        // (T3, M2-5) — PerLevel policy. The memory lives for one pass:
        // the runner (and its memory) is recreated at each iteration.
        let mut runner = match &params.observer {
            Some(observer) => Transducteur::with_observer(
                etape,
                lisage_algebra::interface::MemoryPolicy::PerLevel,
                Arc::clone(observer),
            ),
            None => {
                Transducteur::with_policy(etape, lisage_algebra::interface::MemoryPolicy::PerLevel)
            }
        };
        let resultat = runner.run_same_pool(params.pool, params.anchor, courant);
        let resultat = match resultat {
            Ok(valeur) => valeur,
            Err(erreur) => {
                fermer_span(&params.observer, span);
                return Err(erreur);
            }
        };
        let (image, _) = resultat;
        let next = match union(params.pool, params.keys, params.anchor, courant, image) {
            Ok(valeur) => valeur,
            Err(erreur) => {
                fermer_span(&params.observer, span);
                return Err(erreur);
            }
        };
        // D1-2: the Round event carries the active/modified levels, the
        // before/image/after handles and the growth of the pool.
        if let Some(observer) = &params.observer {
            let niveaux_modifies: Vec<u32> = runner
                .transduction()
                .levels_modifies()
                .iter()
                .map(|level| level.0)
                .collect();
            observer.on_event(&contextualiser(TraceEvent::Round {
                numero: iterations,
                niveaux_actifs: levels_actifs.iter().map(|level| level.0).collect(),
                niveaux_modifies,
                avant: courant,
                image,
                apres: next,
                croissance: params.pool.node_count() as i64 - pool_avant,
                span: None,
            }));
        }
        if next == courant {
            fermer_span(&params.observer, span);
            return Ok(ResultSaturation {
                node: courant,
                iterations,
            });
        }
        if matches!(params.policy, PolicySaturation::AutoDetectee) {
            // Hamez policy: a level not modified by the iteration is removed
            // from the active set.
            let modifies = runner.transduction().levels_modifies();
            levels_actifs.retain(|level| modifies.contains(level));
            if levels_actifs.is_empty() {
                fermer_span(&params.observer, span);
                return Ok(ResultSaturation {
                    node: next,
                    iterations,
                });
            }
        }
        fermer_span(&params.observer, span);
        courant = next;
    }
}

/// Closes a span and emits its `SpanClose` event on the optional observer (D1-1).
fn fermer_span(observer: &Option<Arc<dyn Observer>>, span: crate::observer::SpanId) {
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::SpanClose { span });
    }
    pop_span();
}
