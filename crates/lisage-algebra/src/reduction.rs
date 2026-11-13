//! Reductions § 2.3 (M1-2): R-share (pool side, unique table, M1-1),
//! R-collapse and R-default (controller side, applied at handle creation).
//! Confluence tested by PBT: `reduce∘reduce = reduce` and commutation of
//! quotients.

use std::collections::BTreeMap;

use crate::interface::NodeId;

/// Result of reducing a node form.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reduction<Key> {
    /// R-collapse: the form reduces to that target.
    Collapse(NodeId),
    /// Reduced form (R-default applied), to be interned as is.
    Form(BTreeMap<Key, NodeId>),
}

/// Pure R-collapse: all targets equal ⇒ `Some(target)`, else `None`.
pub fn collapse_edges<Key: Ord>(edges: &BTreeMap<Key, NodeId>) -> Option<NodeId> {
    let mut target: Option<NodeId> = None;
    for child in edges.values() {
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
/// - `key_default` : δ(ℓ) if the level has a default edge ;
/// - `is_default` : predicate "this node is the default diagram D_δ"
///   designated by the controller (the implicit default edge is D_δ) ;
/// - `cardinality` : |Val(ℓ)|, required when the level has no default.
pub fn reduce<Key: Ord + Clone>(
    edges: BTreeMap<Key, NodeId>,
    key_default: Option<Key>,
    is_default: &dyn Fn(NodeId) -> bool,
    cardinality: u64,
) -> Reduction<Key> {
    // R-collapse over the whole set: the implicit default edge is D_δ.
    if let Some(target) = collapse_edges(&edges) {
        // Coverage (spec § 2.2: every absent edge is δ(ℓ)): with a default,
        // the collapse only holds if only the δ edge is explicit (absent
        // edges take the value δ, same path) or if the target is D_δ;
        // otherwise two explicit edges to the same target denote distinct
        // paths (presence) and do not collapse. Without a default, all
        // values must be explicit.
        let covers = match &key_default {
            None => edges.len() as u64 >= cardinality,
            Some(key) => is_default(target) || (edges.len() == 1 && edges.contains_key(key)),
        };
        if covers {
            return Reduction::Collapse(target);
        }
    }
    // R-default: the explicit δ(ℓ) edge leading to D_δ is removed.
    let mut edges = edges;
    if let Some(key) = &key_default
        && edges.get(key).is_some_and(|target| is_default(*target))
    {
        edges.remove(key);
    }
    Reduction::Form(edges)
}

/// R-default **only** (without R-collapse): the explicit δ(ℓ) edge leading
/// to D_δ is removed, the form is never collapsed. Used to switch off
/// R-collapse independently (didactic workshop, backlog § 9).
pub fn reduce_without_collapse<Key: Ord + Clone>(
    edges: BTreeMap<Key, NodeId>,
    key_default: Option<Key>,
    is_default: &dyn Fn(NodeId) -> bool,
) -> Reduction<Key> {
    let mut edges = edges;
    if let Some(key) = &key_default
        && edges.get(key).is_some_and(|target| is_default(*target))
    {
        edges.remove(key);
    }
    Reduction::Form(edges)
}

/// Inverse order (R-default first, then R-collapse) — the confluence PBT
/// checks that both orders give the same result.
pub fn reduce_inverse<Key: Ord + Clone>(
    edges: BTreeMap<Key, NodeId>,
    key_default: Option<Key>,
    is_default: &dyn Fn(NodeId) -> bool,
    cardinality: u64,
) -> Reduction<Key> {
    let mut edges = edges;
    let mut target_removed: Option<NodeId> = None;
    if let Some(key) = &key_default
        && let Some(target) = edges.get(key)
        && is_default(*target)
    {
        target_removed = Some(*target);
        edges.remove(key);
    }
    // If the default edge was the only edge, the node is D_δ: collapse.
    if edges.is_empty() {
        return match target_removed {
            Some(target) => Reduction::Collapse(target),
            None => Reduction::Form(edges),
        };
    }
    if let Some(target) = collapse_edges(&edges) {
        // Same coverage as `reduce` (confluence): with a default, the
        // collapse only holds if only the δ edge is explicit or if the
        // target is D_δ; without a default, all values must be explicit.
        let covers = match &key_default {
            None => edges.len() as u64 >= cardinality,
            Some(key) => is_default(target) || (edges.len() == 1 && edges.contains_key(key)),
        };
        if covers {
            return Reduction::Collapse(target);
        }
    }
    Reduction::Form(edges)
}

/// (D-abs) + (D-trans) — generalized default (spec § 2.4.1, M4-3, statement
/// A6).
///
/// - **(D-abs)** `ν(ℓ, A ⊎ {v ↦ D^def | v ∈ R}) → ν(ℓ, A)` for any
///   subdomain R ⊆ Val(ℓ): any key whose target ≡ D^def is absorbable,
///   not only the edge labeled δ(ℓ) (R-default is only the special case
///   R = {δ(ℓ)}). Only applies if the level has a default (otherwise absent
///   edges would not be D^def).
/// - **(D-trans)** `ν(ℓ, A)` with A = ∅ or all targets ≡ D → D: transparent
///   node, composed offsets.
///
/// `default` is the `NodeId` of D^def designated by the controller when
/// known (used for D-trans on the empty form A = ∅, where no edge gives
/// it).
///
/// Canonicity remains relative to the contract (controller, order, anchor):
/// `≡ D^def` is tested by `is_default` (the pool being canonical, same
/// form = same node).
pub fn reduce_dabs<Key: Ord + Clone>(
    edges: BTreeMap<Key, NodeId>,
    key_default: Option<Key>,
    is_default: &dyn Fn(NodeId) -> bool,
    cardinality: u64,
    default: Option<NodeId>,
) -> Reduction<Key> {
    // Target of D^def if at least one edge leads to it (used for D-trans
    // A = ∅).
    let target_default = default.or(edges.values().find(|c| is_default(**c)).copied());
    // (D-abs): absorption over any subdomain, when the level has a default.
    let edges: BTreeMap<Key, NodeId> = match &key_default {
        Some(_) => edges
            .into_iter()
            .filter(|(_, target)| !is_default(*target))
            .collect(),
        None => edges,
    };
    // (D-trans): A = ∅ after absorption → D^def (transparent node).
    // Inactive without a default key: absent edges would not be D^def.
    if edges.is_empty() && key_default.is_some() {
        return match target_default {
            Some(target) => Reduction::Collapse(target),
            None => Reduction::Form(edges),
        };
    }
    // R-collapse / (D-trans) "all targets ≡ D", same coverage rule as
    // `reduce` (confluence): with a default, the collapse only holds if the
    // target is D^def or if only the δ edge is explicit; without a default,
    // all values must be explicit.
    if let Some(target) = collapse_edges(&edges) {
        let covers = match &key_default {
            None => edges.len() as u64 >= cardinality,
            Some(key) => is_default(target) || (edges.len() == 1 && edges.contains_key(key)),
        };
        if covers {
            return Reduction::Collapse(target);
        }
    }
    Reduction::Form(edges)
}
