//! `inplace` — in-place saturation (M2-4): the kernel that saturates
//! the diagram locally, node by node, instead of rebuilding the whole
//! image in rounds.
//!
//! The round kernel [`crate::saturate::saturate`] rebuilds the **full
//! image** `τ(X, h₀)` at each iteration — fresh runner, fresh memo — and
//! unions it with `X`: breadth-first rounds over the whole diagram. On
//! a local controller (events can fire everywhere at once) each round
//! re-visits the whole current diagram, and the `AutoDetected`
//! deactivation heuristic rides on top of that same kernel: it can
//! remove fires by position, never the traversal itself.
//!
//! The in-place kernel [`saturate_in_place`] implements the saturation
//! of the literature (Ciardo-Luttgen-Siminiceanu for MDDs, Hamez and
//! colleagues for DDDs): events fire **locally**, per node, in
//! depth-first post-order — the children are saturated before the node
//! fires, and the node loops on its own level until local
//! stabilization ("automatic saturation": no policy is needed, the
//! recursion order replaces it).
//!
//! Hash-consing makes the kernel natural:
//! - node contents are immutable, so the local-fixpoint memo
//!   `(node, h) ↦ saturated node` is valid forever;
//! - the per-level step runners (a `RestrictedToLevels` wrapper with a
//!   single active level) keep their memo (L1) for the **whole**
//!   saturation: each `(node, h)` pair is computed once, globally.
//!   Within the subtree of the visited node, the node is the only one
//!   at its own level, so restricting the wrapper to `[level]` is
//!   exactly "fire here".
//!
//! Preconditions, same fixpoint semantics as the round kernel
//! (`νX. D ⊕ τ(X, h₀)`, T2 on a bounded schema, increasing chain):
//! - the step transduction must be **stateless** (stable fingerprint,
//!   no mutable state read by `φ_out`) — the memos are persistent
//!   across the whole saturation, where the round kernel recreated its
//!   runner at each iteration;
//! - fires happen at **nodes**: an initial terminal diagram is returned
//!   as-is (`ψ` has no level to fire at; same degenerate case as a
//!   policy excluding every level).
//!
//! The `policy` and `number_levels` fields of [`SaturationParameters`]
//! are ignored by the kernel.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, MemoryPolicy, NodeId, Pool};

use crate::error::CoreError;
use crate::fold::union;
use crate::observer::Observer;
use crate::saturate::{RestrictedToLevels, ResultSaturation, SaturationParameters};
use crate::schema::SchemaKeys;
use crate::transduction::{Transducer, Transduction};

/// Safety bound of the in-place kernel, in number of **local fires**
/// (much finer grain than a [`crate::saturate::BOUND_ITERATIONS`]
/// round). T2 guarantees termination on a bounded schema; exceeding
/// the bound signals a regression.
pub const BOUND_FIRES: u32 = 1_000_000;

/// In-place saturation of a step transduction: same least fixpoint
/// `νX. D ⊕ τ(X, h₀)` as [`crate::saturate::saturate`], computed by
/// local fires in depth-first post-order instead of global rounds.
///
/// `ResultSaturation.iterations` carries the number of local fires.
pub fn saturate_in_place<T, P>(
    params: &mut SaturationParameters<'_, T, P>,
) -> Result<ResultSaturation, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    let mut engine = InPlace {
        transduction: params.transduction.clone(),
        keys: params.keys,
        runners: BTreeMap::new(),
        fixpoints: HashMap::new(),
        observer: params.observer.clone(),
        fires: 0,
    };
    let h0 = engine.transduction.h0();
    let root = saturate_subtree(&mut engine, params.pool, params.anchor, params.initial, h0)?;
    Ok(ResultSaturation {
        node: root,
        iterations: engine.fires,
    })
}

/// Driver state: one persistent step runner per level (its memo L1
/// lives for the whole saturation) and the local-fixpoint memo
/// `(node, h)`.
struct InPlace<'k, T: Transduction> {
    /// Clone of the step transduction (the drivers wrap it per level).
    transduction: T,
    /// Key algebra of the schema (for the local unions `⊕`).
    keys: &'k SchemaKeys,
    /// Persistent per-level runners: `level ↦ runner of τ at [level]`.
    runners: BTreeMap<u32, Transducer<RestrictedToLevels<T>>>,
    /// Local fixpoints: `(node, h) ↦ saturated node`.
    fixpoints: HashMap<(NodeId, T::H), NodeId>,
    /// Trace observer (M2-6), wired into every created runner.
    observer: Option<Arc<dyn Observer>>,
    /// Number of local fires (safety bound `BOUND_FIRES`).
    fires: u32,
}

impl<'k, T: Transduction + Clone> InPlace<'k, T> {
    /// Persistent runner of the level, created lazily from a fresh
    /// `RestrictedToLevels` wrapper with the single active level.
    fn runner(&mut self, level: u32) -> &mut Transducer<RestrictedToLevels<T>> {
        self.runners.entry(level).or_insert_with(|| {
            let transduction = self.transduction.clone();
            let observer = self.observer.clone();
            let step = RestrictedToLevels::new(transduction, vec![Level(level)]);
            match observer {
                Some(observer) => Transducer::with_observer(step, MemoryPolicy::PerLevel, observer),
                None => Transducer::with_policy(step, MemoryPolicy::PerLevel),
            }
        })
    }
}

/// Saturates the children of a node (post-order), returning the
/// rebuilt node with saturated children.
fn saturate_children<T, P>(
    engine: &mut InPlace<'_, T>,
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    arcs: &ArcRun<u64>,
    h: &T::H,
) -> Result<Vec<(u64, NodeId)>, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    let mut saturated: Vec<(u64, NodeId)> = Vec::with_capacity(arcs.len());
    for (arc, target) in arcs.iter() {
        let h_child = engine.transduction.phi_disp(level, h, *arc);
        let child_saturated = saturate_subtree(engine, pool, anchor, *target, h_child)?;
        saturated.push((*arc, child_saturated));
    }
    Ok(saturated)
}

/// One local fire round: runs the level-restricted transducer on
/// `current`, unions with the previous state, and re-saturates the
/// children if the node changed. Returns the new current node.
fn fire_round<T, P>(
    engine: &mut InPlace<'_, T>,
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    current: NodeId,
    h: &T::H,
) -> Result<NodeId, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    engine.fires += 1;
    if engine.fires > BOUND_FIRES {
        return Err(CoreError::NonTermination { bound: BOUND_FIRES });
    }
    // Restricted to `[level]`, the runner fires only here:
    // `current` is the only node at its level in its subtree.
    let runner = engine.runner(level.0);
    let (image, _) = runner.run_same_pool_from(pool, anchor, current, h.clone())?;
    let next = union(pool, engine.keys, anchor, current, image)?;
    if next == current {
        return Ok(current); // Local fixpoint reached.
    }
    // The fire may have introduced deep, not-yet-saturated content:
    // saturate the children of the new node (fresh ones compute,
    // saturated ones hit the memo).
    let form = pool.resolve(next)?;
    let Form::Node {
        level: level_again,
        arcs,
    } = form
    else {
        // Defensive: a same-level union never collapses to a terminal;
        // if it ever did, the state is already stable.
        return Ok(next);
    };
    let resaturated = saturate_children(engine, pool, anchor, level_again, &arcs, h)?;
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: level_again,
            arcs: ArcRun::from_pairs(resaturated),
        },
    ))
}

/// Saturates the subdiagram `node` under the inherited attribute `h`:
/// depth-first post-order — the children are saturated first, then the
/// node fires locally (its own level, `τ` restricted to `[level]`) until
/// local stabilization. Returns the saturated node, whose denotation is
/// a superset of the input's (T2: the chain is increasing).
fn saturate_subtree<T, P>(
    engine: &mut InPlace<'_, T>,
    pool: &mut P,
    anchor: Anchor,
    node: NodeId,
    h: T::H,
) -> Result<NodeId, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    let form = pool.resolve(node)?;
    match form {
        // Terminals have no level to fire at: already saturated.
        Form::Terminal(_) => Ok(node),
        Form::Node { level, arcs } => {
            if let Some(&saturated) = engine.fixpoints.get(&(node, h.clone())) {
                return Ok(saturated);
            }
            // (1) Depth-first: saturate the children (post-order).
            let saturated_children = saturate_children(engine, pool, anchor, level, &arcs, &h)?;
            let mut current = pool.intern(
                anchor,
                Form::Node {
                    level,
                    arcs: ArcRun::from_pairs(saturated_children),
                },
            );
            // (2) Local fires at `level` until local stabilization.
            loop {
                let next = fire_round(engine, pool, anchor, level, current, &h)?;
                if next == current {
                    break; // Local fixpoint reached.
                }
                current = next;
            }
            engine.fixpoints.insert((node, h), current);
            Ok(current)
        }
    }
}
