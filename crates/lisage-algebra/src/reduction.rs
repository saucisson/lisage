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
        // défaut, le collapse ne vaut que si seul l'arc δ est explicite
        // (les arcs absents prennent la valeur δ, même chemin) ou si la
        // cible est D_δ ; sinon deux arcs explicites vers la même cible
        // dénotent des chemins distincts (présence) et ne collapsent pas.
        // Sans défaut, toutes les valeurs doivent être explicites.
        let couvre = match &cle_defaut {
            None => arcs.len() as u64 >= cardinalite,
            Some(cle) => est_defaut(cible) || (arcs.len() == 1 && arcs.contains_key(cle)),
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
        // Même couverture que `reduire` (confluence) : avec un défaut, le
        // collapse ne vaut que si seul l'arc δ est explicite ou si la cible
        // est D_δ ; sans défaut, toutes les valeurs doivent être explicites.
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

/// (D-abs) + (D-trans) — défaut généralisé (spec § 2.4.1, M4-3, énoncé A6).
///
/// - **(D-abs)** `ν(ℓ, A ⊎ {v ↦ D^déf | v ∈ R}) → ν(ℓ, A)` pour tout
///   sous-domaine R ⊆ Val(ℓ) : toute clé dont la cible ≡ D^déf est
///   absorbable, pas seulement l'arc étiqueté δ(ℓ) (R-default n'en est
///   que le cas particulier R = {δ(ℓ)}). Ne s'applique que si le niveau a
///   un défaut (sinon les arcs absents ne vaudraient pas D^déf).
/// - **(D-trans)** `ν(ℓ, A)` avec A = ∅ ou toutes cibles ≡ D → D : nœud
///   transparent, écarts composés.
///
/// `defaut` est le `NodeId` de D^déf désigné par le contrôleur quand il est
/// connu (sert au D-trans sur forme vide A = ∅, où aucun arc ne le donne).
///
/// La canonicité reste relative au contrat (contrôleur, ordre, ancre) :
/// `≡ D^déf` se teste par `est_defaut` (le pool étant canonique, même
/// forme = même nœud).
pub fn reduire_dabs<Key: Ord + Clone>(
    arcs: BTreeMap<Key, NodeId>,
    cle_defaut: Option<Key>,
    est_defaut: &dyn Fn(NodeId) -> bool,
    cardinalite: u64,
    defaut: Option<NodeId>,
) -> Reduction<Key> {
    // Cible de D^déf si au moins un arc y mène (sert au D-trans A = ∅).
    let cible_defaut = defaut.or(arcs.values().find(|c| est_defaut(**c)).copied());
    // (D-abs) : absorption sur tout sous-domaine, quand le niveau a un défaut.
    let arcs: BTreeMap<Key, NodeId> = match &cle_defaut {
        Some(_) => arcs
            .into_iter()
            .filter(|(_, cible)| !est_defaut(*cible))
            .collect(),
        None => arcs,
    };
    // (D-trans) : A = ∅ après absorption → D^déf (nœud transparent).
    // Inactif sans clé de défaut : les arcs absents ne vaudraient pas D^déf.
    if arcs.is_empty() && cle_defaut.is_some() {
        return match cible_defaut {
            Some(cible) => Reduction::Collapse(cible),
            None => Reduction::Forme(arcs),
        };
    }
    // R-collapse / (D-trans) « toutes cibles ≡ D », même règle de
    // couverture que `reduire` (confluence) : avec un défaut, le collapse
    // ne vaut que si la cible est D^déf ou si seul l'arc δ est explicite ;
    // sans défaut, toutes les valeurs doivent être explicites.
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
