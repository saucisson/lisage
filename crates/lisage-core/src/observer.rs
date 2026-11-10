//! Port d'instrumentation (M1-0, M1-9) : trait `Observer` dans `core/`,
//! no-op par défaut. Les événements sont typés ; le sérialiseur de trace
//! JSONL versionné vit dans `lisage-liseuse`.

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

/// Événements typés émis par le pool à la construction des diagrammes.
/// C'est la trace que Liseuse sérialise et rend.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TraceEvent {
    /// Nœud créé dans le pool (après réductions).
    NodeCreated { node: NodeId, level: Option<Level> },
    /// Nœud partagé : la forme était déjà internée (R-share).
    NodeShared { node: NodeId },
    /// Réduction appliquée à la création d'un handle.
    ReductionApplied { rule: ReductionRule, node: NodeId },
    /// Diagramme achevé : handle racine et taille du pool.
    DiagramBuilt { handle: Handle, nodes: usize },
}

/// Observateur de trace. Implémentation par défaut no-op ; Liseuse fournit
/// un observateur qui sérialise la trace. Bornes `Send`/`Sync` figées (M1-0).
pub trait Observer: Send + Sync {
    /// Réception d'un événement de trace.
    fn on_event(&self, event: &TraceEvent);
}

/// Observateur no-op (défaut).
pub struct NoOpObserver;

impl Observer for NoOpObserver {
    fn on_event(&self, _event: &TraceEvent) {}
}
