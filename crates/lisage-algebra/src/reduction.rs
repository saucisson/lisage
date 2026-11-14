//! Reductions § 2.3 (M1-2): R-share (pool side, unique table, M1-1),
//! R-collapse and R-default (controller side, applied at handle creation).
//! Confluence tested by PBT: `reduce∘reduce = reduce` and quotient
//! commutation.

use std::collections::BTreeMap;

use crate::interface::NodeId;

/// Result of reducing a node form.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reduction<Key> {
    /// R-collapse: the form reduces to this target.
    Collapse(NodeId),
    /// Reduced form (R-default applied), to be interned as-is.
    Forme(BTreeMap<Key, NodeId>),
}

/// Pure R-collapse: all targets equal ⇒ `Some(target)`, otherwise `None`.
pub fn collapse_arcs<Key: Ord>(arcs: &BTreeMap<Key, NodeId>) -> Option<NodeId> {
    let mut target: Option<NodeId> = None;
    for child in arcs.values() {
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
/// - `cle_defaut`: δ(ℓ) if the level has a default arc;
/// - `est_defaut`: predicate "this node is the default diagram D_δ"
///   designated by the controller (the implicit default arc denotes D_δ);
/// - `cardinalite`: |Val(ℓ)|, required when the level has no default.
pub fn reduire<Key: Ord + Clone>(
    arcs: BTreeMap<Key, NodeId>,
    cle_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinalite: u64,
) -> Reduction<Key> {
    // R-collapse on the complete set: the implicit default arc denotes D_δ.
    if let Some(cible) = collapse_arcs(&arcs) {
        // Coverage (spec § 2.2: any absent arc denotes δ(ℓ)): with a
        // default, collapse only holds if only the δ arc is explicit
        // (absent arcs take value δ, same path) or if the target is D_δ;
        // otherwise two explicit arcs to the same target denote distinct
        // paths (presence) and do not collapse.
        // Without a default, all values must be explicit.
        let couvre = match &cle_defaut {
            None => arcs.len() as u64 >= cardinalite,
            Some(cle) => est_defaut(cible) || (arcs.len() == 1 && arcs.contains_key(cle)),
        };
        if couvre {
            return Reduction::Collapse(cible);
        }
    }
    // R-default: the explicit arc δ(ℓ) leading to D_δ is removed.
    let mut arcs = arcs;
    if let Some(cle) = &cle_defaut
        && arcs.get(cle).is_some_and(|cible| est_defaut(*cible))
    {
        arcs.remove(cle);
    }
    Reduction::Forme(arcs)
}

/// R-default **alone** (without R-collapse): the explicit arc δ(ℓ) leading
/// to D_δ is removed, the form is never collapsed. Used to toggle
/// R-collapse independently (didactic workshop, backlog § 9).
pub fn reduire_sans_collapse<Key: Ord + Clone>(
    arcs: BTreeMap<Key, NodeId>,
    cle_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
) -> Reduction<Key> {
    let mut arcs = arcs;
    if let Some(cle) = &cle_defaut
        && arcs.get(cle).is_some_and(|cible| est_defaut(*cible))
    {
        arcs.remove(cle);
    }
    Reduction::Forme(arcs)
}

/// Reverse order (R-default first, then R-collapse) — the confluence PBT
/// checks that both orders give the same result.
pub fn reduire_inverse<Key: Ord + Clone>(
    arcs: BTreeMap<Key, NodeId>,
    cle_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinalite: u64,
) -> Reduction<Key> {
    let mut arcs = arcs;
    let mut cible_supprimee: Option<NodeId> = None;
    if let Some(cle) = &cle_defaut
        && let Some(cible) = arcs.get(cle)
        && est_defaut(*cible)
    {
        cible_supprimee = Some(*cible);
        arcs.remove(cle);
    }
    // If the default arc was the only arc, the node denotes D_δ: collapse.
    if arcs.is_empty() {
        return match cible_supprimee {
            Some(cible) => Reduction::Collapse(cible),
            None => Reduction::Forme(arcs),
        };
    }
    if let Some(cible) = collapse_arcs(&arcs) {
        // Same coverage as `reduire` (confluence): with a default, collapse
        // only holds if only the δ arc is explicit or if the target is D_δ;
        // without a default, all values must be explicit.
        let couvre = match &cle_defaut {
            None => arcs.len() as u64 >= cardinalite,
            Some(cle) => est_defaut(cible) || (arcs.len() == 1 && arcs.contains_key(cle)),
        };
        if couvre {
            return Reduction::Collapse(cible);
        }
    }
    Reduction::Forme(arcs)
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
pub fn reduire_dabs<Key: Ord + Clone>(
    arcs: BTreeMap<Key, NodeId>,
    cle_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinalite: u64,
    defaut: Option<NodeId>,
) -> Reduction<Key> {
    // Target of D^default if at least one arc leads to it (used for D-trans A = ∅).
    let cible_defaut = defaut.or(arcs.values().find(|c| est_defaut(**c)).copied());
    // (D-abs): absorption over any subdomain, when the level has a default.
    let arcs: BTreeMap<Key, NodeId> = match &cle_defaut {
        Some(_) => arcs
            .into_iter()
            .filter(|(_, cible)| !est_defaut(*cible))
            .collect(),
        None => arcs,
    };
    // (D-trans): A = ∅ after absorption → D^default (transparent node).
    // Inactive without a default key: absent arcs would not denote D^default.
    if arcs.is_empty() && cle_defaut.is_some() {
        return match cible_defaut {
            Some(cible) => Reduction::Collapse(cible),
            None => Reduction::Forme(arcs),
        };
    }
    // R-collapse / (D-trans) "all targets ≡ D", same coverage rule as
    // `reduire` (confluence): with a default, collapse only holds if the
    // target is D^default or if only the δ arc is explicit; without a
    // default, all values must be explicit.
    if let Some(cible) = collapse_arcs(&arcs) {
        let couvre = match &cle_defaut {
            None => arcs.len() as u64 >= cardinalite,
            Some(cle) => est_defaut(cible) || (arcs.len() == 1 && arcs.contains_key(cle)),
        };
        if couvre {
            return Reduction::Collapse(cible);
        }
    }
    Reduction::Forme(arcs)
}
