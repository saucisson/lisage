//! Arena: bi-plan hash-consed pool, implementation of the `Pool` port (M1-1,
//! M4-1). Main and shadow plans (spec § 10 Q7), mark-and-sweep GC with
//! profiled compaction, cache metrics.
//!
//! Keyed hash-consing (controller, anchor, form): R-share is the unique
//! table. At handle creation, the controller's reductions apply (M1-2,
//! spec § 2.3): R-collapse and R-default.

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_algebra::reduction::{
    Reduction, reduire, reduire_dabs as crate_reduction_dabs, reduire_sans_collapse,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::biplan::{EntreeNoeud, IndexNiveaux, MetriquesPool, Plan, StatsGc, seuil_atteint};
use crate::observer::{Observer, ReductionRule, TraceEvent};

/// Hash-consing key: (controller, anchor, form).
type CleInterning<Key, Payload> = (ControllerId, Anchor, Form<Key, Payload>);

/// Arena: blind canonical storage of forms, addressed by content. Holds the
/// controller instance: its key algebra (parameterized by the schema, M1-3)
/// decides normal forms, cardinalities and defaults.
pub struct Arena<C: Controller> {
    nodes: Vec<EntreeNoeud<C::Key, C::Payload>>,
    unique: HashMap<CleInterning<C::Key, C::Payload>, NodeId>,
    /// Index of the shadow plan: level → live nodes of this level.
    index_niveaux: IndexNiveaux,
    metriques: MetriquesPool,
    /// Remapping table of the last compaction (empty otherwise).
    remap: Vec<Option<NodeId>>,
    observer: Box<dyn Observer>,
    controleur: C,
}

impl<C: Controller> Arena<C> {
    /// New arena with the given controller and trace observer.
    pub fn new(controleur: C, observer: Box<dyn Observer>) -> Self {
        Arena {
            nodes: Vec::new(),
            unique: HashMap::new(),
            index_niveaux: IndexNiveaux::new(),
            metriques: MetriquesPool::default(),
            remap: Vec::new(),
            observer,
            controleur,
        }
    }

    /// Controller owning this arena.
    pub fn controleur(&self) -> &C {
        &self.controleur
    }

    /// Interning of a terminal with payload (main plan).
    pub fn terminal(&mut self, anchor: Anchor, payload: C::Payload) -> NodeId {
        self.intern(anchor, Form::Terminal(payload))
    }

    /// Interning into a given plan (main by default via `intern`). On a
    /// hash-consing hit (form already present), the existing handle is
    /// returned and its plan is moved to `plan`.
    pub fn intern_plan(
        &mut self,
        anchor: Anchor,
        form: Form<C::Key, C::Payload>,
        plan: Plan,
    ) -> NodeId {
        let node = self.intern(anchor, form);
        if let Some(entree) = self.nodes.get_mut(node.0 as usize) {
            entree.plan = plan;
        }
        node
    }

    /// Plan of a live node.
    pub fn plan_de(&self, node: NodeId) -> Option<Plan> {
        self.nodes
            .get(node.0 as usize)?
            .forme
            .as_ref()
            .map(|_| self.nodes[node.0 as usize].plan)
    }

    /// Live nodes of a level (shadow plan index).
    pub fn index_niveau(&self, level: Level) -> &BTreeSet<NodeId> {
        static VIDE: std::sync::LazyLock<BTreeSet<NodeId>> =
            std::sync::LazyLock::new(BTreeSet::new);
        self.index_niveaux.get(&level).unwrap_or(&VIDE)
    }

    /// Pool metrics (cache + GC).
    pub fn metriques(&self) -> MetriquesPool {
        self.metriques
    }

    /// Remapping table of the last compaction: `remap[old] = new` (`None` if
    /// the old node was a tombstone). Empty if the last GC did not compact.
    pub fn dernier_remap(&self) -> &[Option<NodeId>] {
        &self.remap
    }

    /// Number of live nodes.
    pub fn vivants(&self) -> usize {
        self.nodes.iter().filter(|e| e.forme.is_some()).count()
    }

    /// Mark-and-sweep GC from the given roots: unreachable nodes become
    /// tombstones (removed from the unique table and the index); if the dead
    /// threshold is reached, profiled compaction with a `NodeId` remapping
    /// table.
    pub fn gc(&mut self, racines: &[NodeId]) -> StatsGc {
        let mut atteints = BTreeSet::new();
        let mut pile: Vec<NodeId> = racines.to_vec();
        while let Some(node) = pile.pop() {
            if !atteints.insert(node) {
                continue;
            }
            if let Some(EntreeNoeud {
                forme: Some(Form::Node { arcs, .. }),
                ..
            }) = self.nodes.get(node.0 as usize)
            {
                pile.extend(arcs.values().copied());
            }
        }
        let total = self.nodes.len() as u64;
        let mut collectes = 0u64;
        for (index, entree) in self.nodes.iter_mut().enumerate() {
            if entree.forme.is_some() && !atteints.contains(&NodeId(index as u32)) {
                entree.forme = None;
                collectes += 1;
            }
        }
        // Removal of tombstones from the unique table and the shadow index.
        self.unique
            .retain(|_, node| self.nodes[node.0 as usize].forme.is_some());
        for ensemble in self.index_niveaux.values_mut() {
            ensemble.retain(|node| self.nodes[node.0 as usize].forme.is_some());
        }
        // Live nodes counted on actually live entries (and not derived from
        // the number of slots, which includes tombstones from a previous GC
        // without compaction).
        let vivants = self.nodes.iter().filter(|e| e.forme.is_some()).count() as u64;
        let compaction = seuil_atteint(collectes, total);
        if compaction {
            self.remap = self.compacter();
        } else {
            self.remap.clear();
        }
        self.metriques.gc += 1;
        self.metriques.vivants = vivants;
        self.metriques.morts += collectes;
        if compaction {
            self.metriques.compactions += 1;
        }
        let stats = StatsGc {
            vivants,
            collectes,
            compaction,
        };
        self.observer.on_event(&TraceEvent::Gc {
            vivants,
            collectes,
            compaction,
        });
        stats
    }

    /// Compaction: rebuilds the node vector without tombstones and remaps
    /// the `NodeId`s (remapping table returned). Reserved for the `gc` path,
    /// which first cleans tombstones from the unique table: a direct call
    /// with tombstones would leave stale `NodeId`s in `unique`.
    fn compacter(&mut self) -> Vec<Option<NodeId>> {
        self.remap.clear();
        let mut remap: Vec<Option<NodeId>> = Vec::with_capacity(self.nodes.len());
        let mut nouveaux: Vec<EntreeNoeud<C::Key, C::Payload>> = Vec::new();
        for entree in self.nodes.drain(..) {
            if entree.forme.is_some() {
                remap.push(Some(NodeId(nouveaux.len() as u32)));
                nouveaux.push(entree);
            } else {
                remap.push(None);
            }
        }
        self.nodes = nouveaux;
        for node in self.unique.values_mut() {
            if let Some(Some(nouveau)) = remap.get(node.0 as usize) {
                *node = *nouveau;
            }
        }
        for ensemble in self.index_niveaux.values_mut() {
            *ensemble = ensemble
                .iter()
                .filter_map(|node| remap.get(node.0 as usize).copied().flatten())
                .collect();
        }
        remap
    }

    /// Normalized form of a node: keys go through the normal form of the
    /// controller's key algebra (otherwise hash-consing would violate C1).
    fn forme_normalisee(&self, form: Form<C::Key, C::Payload>) -> Form<C::Key, C::Payload> {
        match form {
            Form::Terminal(payload) => Form::Terminal(payload),
            Form::Node { level, arcs } => Form::Node {
                level,
                arcs: arcs
                    .into_iter()
                    .map(|(key, cible)| (self.controleur.keys().normal_form(&key), cible))
                    .collect(),
            },
        }
    }

    /// Reductions at handle creation (spec § 2.3, M1-2): R-collapse and
    /// R-default, the default diagram D_δ being designated by the controller.
    /// Returns either the collapse target or the reduced form to intern.
    fn reduit(
        &self,
        anchor: Anchor,
        level: Level,
        arcs: BTreeMap<C::Key, NodeId>,
    ) -> Result<Form<C::Key, C::Payload>, NodeId> {
        // Didactic workshop (backlog § 9): the controller can toggle R-default
        // and R-collapse one at a time.
        let defaut_actif = self.controleur.defaut_actif();
        let cle_defaut = if defaut_actif {
            self.controleur.keys().default(level)
        } else {
            None
        };
        let cardinalite = self.controleur.keys().cardinality(level);
        let payload_defaut = if defaut_actif {
            self.controleur.default_payload(level)
        } else {
            None
        };
        let est_defaut = |node: NodeId| -> bool {
            defaut_actif
                && matches!(self.resolve(node), Ok(Form::Terminal(payload)) if payload_defaut.as_ref() == Some(payload))
        };
        // M4-3: generalized default (D-abs/D-trans) if the controller enables it.
        let reduit = if !self.controleur.collapse_actif() {
            reduire_sans_collapse(arcs, cle_defaut, &est_defaut)
        } else if self.controleur.absorption_generalisee() {
            // NodeId of D^défaut under this anchor, if already interned (used
            // for D-trans on empty form A = ∅).
            let defaut = payload_defaut.as_ref().and_then(|p| {
                self.unique
                    .get(&(Self::CONTROLLER, anchor, Form::Terminal(p.clone())))
                    .copied()
            });
            crate_reduction_dabs(arcs, cle_defaut, &est_defaut, cardinalite, defaut)
        } else {
            reduire(arcs, cle_defaut, &est_defaut, cardinalite)
        };
        match reduit {
            Reduction::Collapse(cible) => Err(cible),
            Reduction::Forme(arcs_reduits) => Ok(Form::Node {
                level,
                arcs: arcs_reduits,
            }),
        }
    }
}

impl<C: Controller> Pool for Arena<C> {
    type Key = C::Key;
    type Payload = C::Payload;

    const CONTROLLER: ControllerId = C::FINGERPRINT;

    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId {
        let forme = self.forme_normalisee(form);
        let forme = match forme {
            Form::Node { level, arcs } => {
                let taille_avant = arcs.len();
                match self.reduit(anchor, level, arcs) {
                    Err(cible) => {
                        self.observer.on_event(&TraceEvent::ReductionApplied {
                            rule: ReductionRule::Collapse,
                            level,
                        });
                        return cible;
                    }
                    Ok(Form::Node { arcs, .. }) => {
                        if arcs.len() < taille_avant {
                            self.observer.on_event(&TraceEvent::ReductionApplied {
                                rule: ReductionRule::Default,
                                level,
                            });
                        }
                        Form::Node { level, arcs }
                    }
                    Ok(terminal) => terminal,
                }
            }
            terminal => terminal,
        };
        let cle = (Self::CONTROLLER, anchor, forme.clone());
        if let Some(node) = self.unique.get(&cle) {
            self.metriques.partages += 1;
            self.observer
                .on_event(&TraceEvent::NodeShared { node: *node });
            return *node;
        }
        let node = NodeId(self.nodes.len() as u32);
        let level = match &forme {
            Form::Terminal(_) => None,
            Form::Node { level, .. } => Some(*level),
        };
        self.nodes.push(EntreeNoeud {
            plan: Plan::Principal,
            forme: Some(forme),
        });
        if let Some(level) = level {
            self.index_niveaux.entry(level).or_default().insert(node);
        }
        self.unique.insert(cle, node);
        self.metriques.creations += 1;
        self.observer
            .on_event(&TraceEvent::NodeCreated { node, level });
        node
    }

    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError> {
        match self.nodes.get(node.0 as usize) {
            Some(entree) => entree.forme.as_ref().ok_or(PoolError::UnknownNode(node)),
            None => Err(PoolError::UnknownNode(node)),
        }
    }

    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle {
        Handle {
            controller: Self::CONTROLLER,
            anchor,
            node,
        }
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
