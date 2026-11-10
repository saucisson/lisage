//! Réductions § 2.3 (M1-2) : R-share (côté pool, table unique, M1-1),
//! R-collapse et R-default (côté contrôleur, appliquées à la création du
//! handle). Confluence testée par PBT : `reduce∘reduce = reduce` et
//! commutation des quotients.

use std::collections::BTreeMap;

use crate::interface::NodeId;

/// Résultat de la réduction d'une forme de nœud.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reduction<Key> {
    /// R-collapse : la forme se réduit à cette cible.
    Collapse(NodeId),
    /// Forme réduite (R-default appliqué), à interniser telle quelle.
    Forme(BTreeMap<Key, NodeId>),
}

/// R-collapse pur : toutes les cibles égales ⇒ `Some(cible)`, sinon `None`.
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

/// Applique R-collapse puis R-default à un nœud (spec § 2.3).
///
/// - `cle_defaut` : δ(ℓ) si le niveau a un arc par défaut ;
/// - `est_defaut` : prédicat « ce nœud est le diagramme de défaut D_δ »
///   désigné par le contrôleur (l'arc par défaut implicite vaut D_δ) ;
/// - `cardinalite` : |Val(ℓ)|, exigée quand le niveau n'a pas de défaut.
pub fn reduire<Key: Ord + Clone>(
    arcs: BTreeMap<Key, NodeId>,
    cle_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinalite: u64,
) -> Reduction<Key> {
    // R-collapse sur l'ensemble complet : l'arc par défaut implicite vaut D_δ.
    if let Some(cible) = collapse_arcs(&arcs) {
        // Couverture (spec § 2.2 : tout arc absent vaut δ(ℓ)) : avec un
        // défaut, les arcs absents mènent à la même continuation que δ, donc
        // le collapse vaut si δ est explicite ou si la cible est D_δ ;
        // sans défaut, toutes les valeurs doivent être explicites.
        let couvre = match &cle_defaut {
            None => arcs.len() as u64 >= cardinalite,
            Some(cle) => arcs.contains_key(cle) || est_defaut(cible),
        };
        if couvre {
            return Reduction::Collapse(cible);
        }
    }
    // R-default : l'arc δ(ℓ) explicite menant à D_δ est supprimé.
    let mut arcs = arcs;
    if let Some(cle) = &cle_defaut
        && arcs.get(cle).is_some_and(|cible| est_defaut(*cible))
    {
        arcs.remove(cle);
    }
    Reduction::Forme(arcs)
}

/// Ordre inverse (R-default d'abord, puis R-collapse) — la PBT de confluence
/// vérifie que les deux ordres donnent le même résultat.
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
    // Si l'arc de défaut était le seul arc, le nœud vaut D_δ : collapse.
    if arcs.is_empty() {
        return match cible_supprimee {
            Some(cible) => Reduction::Collapse(cible),
            None => Reduction::Forme(arcs),
        };
    }
    if let Some(cible) = collapse_arcs(&arcs) {
        // Couverture (spec § 2.2 : tout arc absent vaut δ(ℓ)) : avec un
        // défaut, les arcs absents mènent à la même continuation que δ, donc
        // le collapse vaut si δ est explicite ou si la cible est D_δ ;
        // sans défaut, toutes les valeurs doivent être explicites.
        let couvre = match &cle_defaut {
            None => arcs.len() as u64 >= cardinalite,
            Some(cle) => arcs.contains_key(cle) || est_defaut(cible),
        };
        if couvre {
            return Reduction::Collapse(cible);
        }
    }
    Reduction::Forme(arcs)
}
