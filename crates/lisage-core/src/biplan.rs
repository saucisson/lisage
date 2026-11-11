//! Pool bi-plan (M4-1, énoncé C5) : plans **principal** et **d'ombre**, GC
//! mark-and-sweep, compaction **profilée** (seuil de morts), et métriques
//! de cache (créations/partages du hash-consing, vivants/morts du GC).
//!
//! Le plan d'ombre porte l'index niveau → formes (spec § 10 Q7 : « l'index
//! niveau → formes vit dans le plan d'ombre »), le plan principal porte les
//! nœuds du diagramme. Le stockage reste partagé (C1) ; seul le parcours
//! distingue les plans.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Level, NodeId};

/// Plan d'un nœud du pool bi-plan.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Plan {
    /// Plan principal : les nœuds du diagramme.
    Principal,
    /// Plan d'ombre : l'index niveau → formes et les nœuds auxiliaires.
    Ombre,
}

/// Seuil de compaction profilée : on compacte quand la proportion de
/// tombstones atteint 1/4 des slots (morts × 4 ≥ total).
pub const SEUIL_COMPACTION_NUM: u64 = 1;
/// Dénominateur du seuil de compaction.
pub const SEUIL_COMPACTION_DEN: u64 = 4;

/// Base de pourcentage pour `taux_hit` (retourné en pour cent, entier).
pub const BASE_POURCENTAGE: u64 = 100;

/// Métriques du pool : cache du hash-consing et compteurs de GC.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MetriquesPool {
    /// Nœuds créés (miss du hash-consing).
    pub creations: u64,
    /// Nœuds partagés (hit du hash-consing, R-share).
    pub partages: u64,
    /// Nœuds vivants après le dernier GC.
    pub vivants: u64,
    /// Nœuds collectés au total.
    pub morts: u64,
    /// Nombre de GC exécutés.
    pub gc: u64,
    /// Nombre de compactions exécutées.
    pub compactions: u64,
}

impl MetriquesPool {
    /// Taux de hit du cache = partages / (créations + partages) ; 0 si vide.
    pub fn taux_hit(&self) -> u64 {
        let total = self.creations + self.partages;
        self.partages
            .checked_mul(BASE_POURCENTAGE)
            .and_then(|p| p.checked_div(total))
            .unwrap_or(0)
    }
}

/// Statistiques d'un GC : vivants, collectés, compaction effectuée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatsGc {
    /// Nœuds vivants (atteints depuis les racines).
    pub vivants: u64,
    /// Nœuds collectés (tombstones posées).
    pub collectes: u64,
    /// Compaction effectuée (seuil de morts atteint).
    pub compaction: bool,
}

/// Entrée du pool bi-plan : plan + forme (tombstone après GC).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct EntreeNoeud<Key, Payload> {
    pub plan: Plan,
    pub forme: Option<lisage_algebra::interface::Form<Key, Payload>>,
}

/// Index du plan d'ombre : niveau → nœuds vivants de ce niveau.
pub(crate) type IndexNiveaux = BTreeMap<Level, BTreeSet<NodeId>>;

/// Décide si le seuil de compaction profilée est atteint.
pub fn seuil_atteint(morts: u64, total: u64) -> bool {
    morts * SEUIL_COMPACTION_DEN >= total * SEUIL_COMPACTION_NUM && total > 0
}
