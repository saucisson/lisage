//! Reductions § 2.3 (M1-2): R-share (pool side, unique table, M1-1),
//! R-collapse and R-default (controller side, applied at handle creation).
//! Confluence tested by PBT: `reduce∘reduce = reduce` and quotient
//! commutation.

use std::fmt::Debug;
use std::hash::Hash;

use crate::interface::{ArcRun, ArcStore, NodeId};

/// Result of reducing a node form.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reduction<Key> {
    /// R-collapse: the form reduces to this target.
    Collapse(NodeId),
    /// Reduced form (R-default applied), to be interned as-is: a sorted
    /// run (MEM-5), the canonical arc storage.
    Form(ArcRun<Key>),
}

/// Pure R-collapse: all targets equal ⇒ `Some(target)`, otherwise `None`.
/// Sees only the ordered iteration of the `ArcStore` port (MEM-3).
pub fn collapse_arcs<Key, Store>(arcs: &Store) -> Option<NodeId>
where
    Key: Ord,
    Store: ArcStore<Key = Key> + ?Sized,
{
    let mut target: Option<NodeId> = None;
    for (_, child) in arcs.iter() {
        match target {
            None => target = Some(*child),
            Some(previous) if previous == *child => {}
            Some(_) => return None,
        }
    }
    target
}

/// Applies R-collapse then R-default to a node (spec § 2.3).
///
/// - `key_defaut`: δ(ℓ) if the level has a default arc;
/// - `est_defaut`: predicate "this node is the default diagram D_δ"
///   designated by the controller (the implicit default arc denotes D_δ);
/// - `cardinality`: |Val(ℓ)|, required when the level has no default.
pub fn reduce<Key: Clone + Ord + Hash + Debug + Send + Sync>(
    arcs: ArcRun<Key>,
    key_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinality: u64,
) -> Reduction<Key> {
    // R-collapse on the complete set: the implicit default arc denotes D_δ.
    if let Some(target) = collapse_arcs(&arcs) {
        // Coverage (spec § 2.2: any absent arc denotes δ(ℓ)): with a
        // default, collapse only holds if only the δ arc is explicit
        // (absent arcs take value δ, same path) or if the target is D_δ;
        // otherwise two explicit arcs to the same target denote distinct
        // paths (presence) and do not collapse.
        // Without a default, all values must be explicit.
        let couvre = match &key_defaut {
            None => arcs.len() as u64 >= cardinality,
            Some(key) => est_defaut(target) || (arcs.len() == 1 && arcs.contains_key(key)),
        };
        if couvre {
            return Reduction::Collapse(target);
        }
    }
    // R-default: the explicit arc δ(ℓ) leading to D_δ is removed.
    let arcs = match &key_defaut {
        Some(key) if arcs.get(key).is_some_and(|target| est_defaut(*target)) => {
            ArcRun::from_pairs(arcs.into_iter().filter(|(k, _)| k != key))
        }
        _ => arcs,
    };
    Reduction::Form(arcs)
}

/// R-default **alone** (without R-collapse): the explicit arc δ(ℓ) leading
/// to D_δ is removed, the form is never collapsed. Used to toggle
/// R-collapse independently (didactic workshop, backlog § 9).
pub fn reduce_without_collapse<Key: Clone + Ord + Hash + Debug + Send + Sync>(
    arcs: ArcRun<Key>,
    key_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
) -> Reduction<Key> {
    let arcs = match &key_defaut {
        Some(key) if arcs.get(key).is_some_and(|target| est_defaut(*target)) => {
            ArcRun::from_pairs(arcs.into_iter().filter(|(k, _)| k != key))
        }
        _ => arcs,
    };
    Reduction::Form(arcs)
}

/// Reverse order (R-default first, then R-collapse) — the confluence PBT
/// checks that both orders give the same result.
pub fn reduce_inverse<Key: Clone + Ord + Hash + Debug + Send + Sync>(
    arcs: ArcRun<Key>,
    key_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinality: u64,
) -> Reduction<Key> {
    let mut target_supprimee: Option<NodeId> = None;
    let arcs = match &key_defaut {
        Some(key) if let Some(target) = arcs.get(key).filter(|t| est_defaut(**t)) => {
            target_supprimee = Some(*target);
            ArcRun::from_pairs(arcs.into_iter().filter(|(k, _)| k != key))
        }
        _ => arcs,
    };
    // If the default arc was the only arc, the node denotes D_δ: collapse.
    if arcs.is_empty() {
        return match target_supprimee {
            Some(target) => Reduction::Collapse(target),
            None => Reduction::Form(arcs),
        };
    }
    if let Some(target) = collapse_arcs(&arcs) {
        // Same coverage as `reduce` (confluence): with a default, collapse
        // only holds if only the δ arc is explicit or if the target is D_δ;
        // without a default, all values must be explicit.
        let couvre = match &key_defaut {
            None => arcs.len() as u64 >= cardinality,
            Some(key) => est_defaut(target) || (arcs.len() == 1 && arcs.contains_key(key)),
        };
        if couvre {
            return Reduction::Collapse(target);
        }
    }
    Reduction::Form(arcs)
}

/// (D-abs) + (D-trans) — generalized default (spec § 2.4.1, M4-3, claim A6).
///
/// - **(D-abs)** `ν(ℓ, A ⊎ {v ↦ D^default | v ∈ R}) → ν(ℓ, A)` for any
///   subdomain R ⊆ Val(ℓ): any key whose target ≡ D^default is
///   absorbable, not only the arc labeled δ(ℓ) (R-default is only the
///   special case R = {δ(ℓ)}). Only applies if the level has a default
///   (otherwise absent arcs would not denote D^default).
/// - **(D-trans)** `ν(ℓ, A)` with A = ∅ or all targets ≡ D → D: transparent
///   node, composed gaps.
///
/// `defaut` is the `NodeId` of D^default designated by the controller when
/// known (used for D-trans on empty form A = ∅, where no arc gives it).
///
/// Canonicity remains relative to the contract (controller, order, anchor):
/// `≡ D^default` is tested by `est_defaut` (the pool being canonical, same
/// form = same node).
pub fn reduce_dabs<Key: Clone + Ord + Hash + Debug + Send + Sync>(
    arcs: ArcRun<Key>,
    key_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinality: u64,
    defaut: Option<NodeId>,
) -> Reduction<Key> {
    // Target of D^default if at least one arc leads to it (used for D-trans A = ∅).
    let target_defaut = defaut.or(arcs.values().find(|c| est_defaut(**c)).copied());
    // (D-abs): absorption over any subdomain, when the level has a default.
    let arcs: ArcRun<Key> = match &key_defaut {
        Some(_) => ArcRun::from_pairs(arcs.into_iter().filter(|(_, target)| !est_defaut(*target))),
        None => arcs,
    };
    // (D-trans): A = ∅ after absorption → D^default (transparent node).
    // Inactive without a default key: absent arcs would not denote D^default.
    if arcs.is_empty() && key_defaut.is_some() {
        return match target_defaut {
            Some(target) => Reduction::Collapse(target),
            None => Reduction::Form(arcs),
        };
    }
    // R-collapse / (D-trans) "all targets ≡ D", same coverage rule as
    // `reduce` (confluence): with a default, collapse only holds if the
    // target is D^default or if only the δ arc is explicit; without a
    // default, all values must be explicit.
    if let Some(target) = collapse_arcs(&arcs) {
        let couvre = match &key_defaut {
            None => arcs.len() as u64 >= cardinality,
            Some(key) => est_defaut(target) || (arcs.len() == 1 && arcs.contains_key(key)),
        };
        if couvre {
            return Reduction::Collapse(target);
        }
    }
    Reduction::Form(arcs)
}
