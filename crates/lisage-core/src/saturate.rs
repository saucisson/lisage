//! `saturate` — increasing fixed point of a step transduction (M2-4,
//! spec § 7, statements T2/T3): `saturate(τ, D) = νX. D ⊕ τ(X, h₀)`.
//!
//! Two saturation policies, two instances of the same fixed-point
//! semantics (same final result, different iteration strategy):
//! - [`PolitiqueSaturation::Statique`] (à la Ciardo): `τ` is only applied
//!   to the declared levels;
//! - [`PolitiqueSaturation::AutoDetectee`] (à la Hamez): all levels are
//!   active at the start; a level whose current iteration changed nothing
//!   is removed from the active set.
//!
//! The construction merge `⊕` is the idempotent union (Bool, A5).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::observer::{Observer, TraceEvent};
use crate::pli::union;
use crate::schema::SchemaKeys;
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Safety bound for the fixed-point loop (T2 guarantees termination on a
/// bounded schema; the bound is only there to detect a regression).
pub const BORNE_ITERATIONS: u32 = 1_000;

/// Salt for the fingerprint of the [`RestreintAuxNiveaux`] wrapper:
/// domain-separates the wrapper's fingerprint from that of the inner
/// transduction.
const SEL_FINGERPRINT_WRAPPER: u64 = 0x5A7A_0000_0000_0000;

/// Saturation policy (M2-4): two instances of the same fixed-point
/// semantics. Both policies produce the same fixed point when `Statique`
/// declares all levels; only the iteration strategy differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolitiqueSaturation {
    /// Static per level (à la Ciardo): only the listed levels are saturated
    /// at each iteration.
    Statique(Vec<Level>),
    /// Auto-detected (à la Hamez): all levels are active at the start; a
    /// level is removed as soon as an iteration changes nothing at that
    /// level.
    AutoDetectee,
}

/// Parameters of a saturation (record, ADR-0003).
pub struct ParametresSaturation<'a, T, P> {
    /// Step transduction `τ` (cloneable for iterations).
    pub transduction: &'a T,
    /// Saturation policy.
    pub politique: &'a PolitiqueSaturation,
    /// Unique pool (read + write, payload `bool`): the current iteration and
    /// the result live in the same pool.
    pub pool: &'a mut P,
    /// Key algebra of the schema (for the union).
    pub keys: &'a SchemaKeys,
    /// Anchor of the root.
    pub anchor: Anchor,
    /// Initial node `D`.
    pub initial: NodeId,
    /// Number of levels in the schema (for AutoDetectee: all levels).
    pub nombre_niveaux: u32,
    /// Trace observer (M2-6): receives one `Round` per iteration and the
    /// step runner's events. `None` = no trace.
    pub observateur: Option<Arc<dyn Observer>>,
}

/// Result of a saturation: fixed-point node + number of iterations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultatSaturation {
    /// Fixed-point node reached.
    pub noeud: NodeId,
    /// Number of iterations of the fixed-point loop.
    pub iterations: u32,
}

/// Wrapper transduction that restricts a step transduction to a set of
/// active levels: at active levels, it delegates to the inner transduction
/// and records the modified levels; at the others, identity copy.
pub struct RestreintAuxNiveaux<T> {
    inner: T,
    niveaux_actifs: Vec<Level>,
    niveaux_modifies: Mutex<BTreeSet<Level>>,
}

impl<T: Clone> Clone for RestreintAuxNiveaux<T> {
    fn clone(&self) -> Self {
        // The clone starts from a fresh tracking: `saturer` creates a new
        // wrapper at each iteration anyway.
        RestreintAuxNiveaux {
            inner: self.inner.clone(),
            niveaux_actifs: self.niveaux_actifs.clone(),
            niveaux_modifies: Mutex::new(BTreeSet::new()),
        }
    }
}

impl<T> RestreintAuxNiveaux<T> {
    /// New wrapper: `inner` restricted to `niveaux_actifs`.
    pub fn new(inner: T, niveaux_actifs: Vec<Level>) -> Self {
        RestreintAuxNiveaux {
            inner,
            niveaux_actifs,
            niveaux_modifies: Mutex::new(BTreeSet::new()),
        }
    }

    /// Levels at which `φ_out` produced a node different from the input since
    /// the wrapper was created (AutoDetectee policy).
    pub fn niveaux_modifies(&self) -> BTreeSet<Level> {
        self.niveaux_modifies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl<T: Transduction> Transduction for RestreintAuxNiveaux<T> {
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
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
        if self.niveaux_actifs.contains(&ctx.level) {
            let noeud_entree = ctx.noeud_entree;
            let level = ctx.level;
            let (noeud, effet) = self.inner.phi_out(pool, ctx);
            if noeud != noeud_entree {
                self.niveaux_modifies
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(level);
            }
            (noeud, effet)
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
                crate::memoire::EffetMemoire::new(),
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

/// Increasing fixed point `saturate(τ, D)` on the pool (spec § 7, T2):
/// iterates `X ← X ⊕ τ(X)` until stabilization, applying `τ` according to
/// the policy (restricted to active levels; AutoDetectee removes
/// stabilized levels from the active set).
///
/// Termination (T2) is guaranteed on a bounded schema: the set of canonical
/// forms is finite and the chain is increasing for denotation inclusion.
/// [`BORNE_ITERATIONS`] is only a safety net against a regression.
pub fn saturer<T, P>(
    params: &mut ParametresSaturation<'_, T, P>,
) -> Result<ResultatSaturation, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    // Active levels according to the policy.
    let mut niveaux_actifs: Vec<Level> = match params.politique {
        PolitiqueSaturation::Statique(niveaux) => niveaux.clone(),
        PolitiqueSaturation::AutoDetectee => (0..params.nombre_niveaux).map(Level).collect(),
    };

    let mut courant = params.initial;
    let mut iterations = 0u32;
    loop {
        iterations += 1;
        if iterations > BORNE_ITERATIONS {
            return Err(CoreError::NonTerminaison {
                borne: BORNE_ITERATIONS,
            });
        }
        if let Some(observateur) = &params.observateur {
            observateur.on_event(&TraceEvent::Round { numero: iterations });
        }
        let etape = RestreintAuxNiveaux::new(params.transduction.clone(), niveaux_actifs.clone());
        // Spec § 7: saturate steps may produce a memory effect (T3, M2-5) —
        // ParNiveau policy. The memory lives for one pass: the runner (and
        // its memory) is recreated at each iteration.
        let mut runner = match &params.observateur {
            Some(observateur) => Transducteur::avec_observateur(
                etape,
                lisage_algebra::interface::PolitiqueMemoire::ParNiveau,
                Arc::clone(observateur),
            ),
            None => Transducteur::avec_politique(
                etape,
                lisage_algebra::interface::PolitiqueMemoire::ParNiveau,
            ),
        };
        let (image, _) = runner.run_meme_pool(params.pool, params.anchor, courant)?;
        let suivant = union(params.pool, params.keys, params.anchor, courant, image)?;
        if suivant == courant {
            return Ok(ResultatSaturation {
                noeud: courant,
                iterations,
            });
        }
        if matches!(params.politique, PolitiqueSaturation::AutoDetectee) {
            // Hamez policy: a level not modified by the iteration is removed
            // from the active set.
            let modifies = runner.transduction().niveaux_modifies();
            niveaux_actifs.retain(|niveau| modifies.contains(niveau));
            if niveaux_actifs.is_empty() {
                return Ok(ResultatSaturation {
                    noeud: suivant,
                    iterations,
                });
            }
        }
        courant = suivant;
    }
}
