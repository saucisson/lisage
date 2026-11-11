//! Port d'instrumentation (M1-0, M1-9) : `Observer` dans `core/`, no-op par
//! défaut ; sérialiseur JSONL dans `lisage-liseuse`.

use lisage_algebra::interface::{Handle, Level, NodeId};

/// Règle de réduction appliquée à la création d'un handle (spec § 2.3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReductionRule {
    /// R-share : la forme existait déjà, le nœud est partagé.
    Share,
    /// R-collapse : tous les arcs mènent à la même cible.
    Collapse,
    /// R-default : l'arc δ(ℓ) menait au diagramme de défaut, il est supprimé.
    Default,
}

/// Canal d'un attribut (vue 3 « pas à pas », canaux colorés — M2-6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CanalAttr {
    /// Attribut **hérité** (descendant, `φ_disp`) — rendu en bleu.
    Herite,
    /// Attribut **synthétisé** (remontant, `φ_aggr`) — rendu en orange.
    Synthétise,
    /// **Mémoire de niveau** (écriture par `φ_out`) — rendu en violet.
    Memoire,
}

/// Événements typés émis par le pool à la construction des diagrammes et
/// par le runner de transductions pendant l'exécution (M2-6). C'est la
/// trace que Liseuse sérialise et rend.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TraceEvent {
    /// Nœud créé dans le pool (après réductions).
    NodeCreated { node: NodeId, level: Option<Level> },
    /// Nœud partagé : la forme était déjà internée (R-share).
    NodeShared { node: NodeId },
    /// Réduction appliquée à la création d'un handle.
    ReductionApplied { rule: ReductionRule, level: Level },
    /// Diagramme achevé : handle racine et taille du pool.
    DiagramBuilt { handle: Handle, nodes: usize },
    /// Attribut circulant sur un canal (hérité/synthétisé/mémoire), M2-6.
    Attr {
        /// Canal de l'attribut.
        canal: CanalAttr,
        /// Nœud concerné.
        noeud: NodeId,
        /// Valeur (format `Debug`), affichée telle quelle.
        valeur: String,
    },
    /// Mémo hit : le couple (op, nœud, h, version) était en computed table.
    MemoHit {
        /// Nœud dont la visite a hit.
        noeud: NodeId,
    },
    /// Mémo miss : visite complète du sous-arbre.
    MemoMiss {
        /// Nœud dont la visite a miss.
        noeud: NodeId,
    },
    /// GC du pool bi-plan (M4-1) : vivants, collectés, compaction profilée.
    Gc {
        /// Nœuds vivants après collecte.
        vivants: u64,
        /// Nœuds collectés.
        collectes: u64,
        /// Compaction effectuée (seuil de morts atteint).
        compaction: bool,
    },
    /// Début d'un round de saturation (M2-6) : numéro de l'itération.
    Round {
        /// Numéro du round (1-based).
        numero: u32,
    },
}

/// Observateur de trace, no-op par défaut (Liseuse fournit le sien).
/// Bornes `Send`/`Sync` figées (M1-0).
pub trait Observer: Send + Sync {
    /// Réception d'un événement de trace.
    fn on_event(&self, event: &TraceEvent);
}

/// Observateur no-op (défaut).
pub struct NoOpObserver;

impl Observer for NoOpObserver {
    fn on_event(&self, _event: &TraceEvent) {}
}
