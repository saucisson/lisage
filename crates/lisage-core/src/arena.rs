//! Arena: two-plane hash-consed pool, implementation of the `Pool` port
//! (M1-1, M4-1). Main and shadow planes (spec § 10 Q7), mark-and-sweep GC
//! with profiled compaction, cache metrics.
//!
//! Keyed hash-consing (controller, anchor, form): R-share is the unique
//! table. At handle creation, the controller's reductions apply (M1-2,
//! spec § 2.3): R-collapse and R-default.

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_algebra::reduction::{
    Reduction, reduce, reduce_dabs as crate_reduction_dabs, reduce_without_collapse,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::biplan::{IndexLevels, MetriquesPool, NodeEntry, Plan, StatsGc, threshold_atteint};
use crate::observer::{Observer, ReductionRule, TraceEvent};

/// Hash-consing key: (controller, anchor, form).
type KeyInterning<Key, Payload> = (ControllerId, Anchor, Form<Key, Payload>);

/// Arena: canonical, content-addressed storage of forms. Holds the
/// controller instance: its key algebra (parameterized by the schema, M1-3)
/// decides normal forms, cardinalities and defaults.
pub struct Arena<C: Controller> {
    nodes: Vec<NodeEntry<C::Key, C::Payload>>,
    unique: HashMap<KeyInterning<C::Key, C::Payload>, NodeId>,
    /// Shadow-plane index: level → live nodes of that level.
    index_levels: IndexLevels,
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
            index_levels: IndexLevels::new(),
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

    /// Interning of a payload terminal (main plane).
    pub fn terminal(&mut self, anchor: Anchor, payload: C::Payload) -> NodeId {
        self.intern(anchor, Form::Terminal(payload))
    }

    /// Interning into a given plane (main by default via `intern`). On a
    /// hash-consing hit (form already present), the existing handle is
    /// returned and its plane is moved to `plan`.
    pub fn intern_plan(
        &mut self,
        anchor: Anchor,
        form: Form<C::Key, C::Payload>,
        plan: Plan,
    ) -> NodeId {
        let node = self.intern(anchor, form);
        if let Some(input) = self.nodes.get_mut(node.0 as usize) {
            input.plan = plan;
        }
        node
    }

    /// Plane of a live node.
    pub fn plan_de(&self, node: NodeId) -> Option<Plan> {
        self.nodes
            .get(node.0 as usize)?
            .form
            .as_ref()
            .map(|_| self.nodes[node.0 as usize].plan)
    }

    /// Live nodes of a level (shadow-plane index).
    pub fn index_level(&self, level: Level) -> &BTreeSet<NodeId> {
        static EMPTY: std::sync::LazyLock<BTreeSet<NodeId>> =
            std::sync::LazyLock::new(BTreeSet::new);
        self.index_levels.get(&level).unwrap_or(&EMPTY)
    }

    /// Pool metrics (cache + GC).
    pub fn metriques(&self) -> MetriquesPool {
        self.metriques
    }

    /// Remapping table of the last compaction: `remap[old] = new` (`None` if
    /// the old node was a tombstone). Empty if the last GC did not compact.
    pub fn last_remap(&self) -> &[Option<NodeId>] {
        &self.remap
    }

    /// Number of live nodes.
    pub fn live(&self) -> usize {
        self.nodes.iter().filter(|e| e.form.is_some()).count()
    }

    /// Mark-and-sweep GC from the given roots: unreachable nodes become
    /// tombstones (removed from the unique table and the index); if the
    /// dead threshold is reached, profiled compaction with a `NodeId`
    /// remapping table.
    pub fn gc(&mut self, roots: &[NodeId]) -> StatsGc {
        let mut atteints = BTreeSet::new();
        let mut stack: Vec<NodeId> = roots.to_vec();
        while let Some(node) = stack.pop() {
            if !atteints.insert(node) {
                continue;
            }
            if let Some(NodeEntry {
                form: Some(Form::Node { arcs, .. }),
                ..
            }) = self.nodes.get(node.0 as usize)
            {
                stack.extend(arcs.values().copied());
            }
        }
        let total = self.nodes.len() as u64;
        let mut collected = 0u64;
        for (index, input) in self.nodes.iter_mut().enumerate() {
            if input.form.is_some() && !atteints.contains(&NodeId(index as u32)) {
                input.form = None;
                collected += 1;
            }
        }
        // Removal of tombstones from the unique table and the shadow index.
        self.unique
            .retain(|_, node| self.nodes[node.0 as usize].form.is_some());
        for set in self.index_levels.values_mut() {
            set.retain(|node| self.nodes[node.0 as usize].form.is_some());
        }
        // Live nodes counted on actually-live entries (not derived from the
        // slot count, which includes tombstones from a previous GC without
        // compaction).
        let live = self.nodes.iter().filter(|e| e.form.is_some()).count() as u64;
        let compaction = threshold_atteint(collected, total);
        if compaction {
            self.remap = self.compacter();
        } else {
            self.remap.clear();
        }
        self.metriques.gc += 1;
        self.metriques.live = live;
        self.metriques.dead += collected;
        if compaction {
            self.metriques.compactions += 1;
        }
        let stats = StatsGc {
            live,
            collected,
            compaction,
        };
        self.observer.on_event(&TraceEvent::Gc {
            live,
            collected,
            compaction,
        });
        stats
    }

    /// Compaction: rebuilds the node vector without tombstones and remaps
    /// the `NodeId`s (returned remapping table). Reserved for the `gc` path,
    /// which first cleans the tombstones from the unique table: a direct
    /// call with tombstones would leave stale `NodeId`s in `unique`.
    fn compacter(&mut self) -> Vec<Option<NodeId>> {
        self.remap.clear();
        let mut remap: Vec<Option<NodeId>> = Vec::with_capacity(self.nodes.len());
        let mut new_nodes: Vec<NodeEntry<C::Key, C::Payload>> = Vec::new();
        for input in self.nodes.drain(..) {
            if input.form.is_some() {
                remap.push(Some(NodeId(new_nodes.len() as u32)));
                new_nodes.push(input);
            } else {
                remap.push(None);
            }
        }
        self.nodes = new_nodes;
        for node in self.unique.values_mut() {
            if let Some(Some(new)) = remap.get(node.0 as usize) {
                *node = *new;
            }
        }
        for set in self.index_levels.values_mut() {
            *set = set
                .iter()
                .filter_map(|node| remap.get(node.0 as usize).copied().flatten())
                .collect();
        }
        remap
    }

    /// Normalized form of a node: keys go through the normal form of the
    /// controller's key algebra (otherwise hash-consing would violate C1).
    fn form_normalisee(&self, form: Form<C::Key, C::Payload>) -> Form<C::Key, C::Payload> {
        match form {
            Form::Terminal(payload) => Form::Terminal(payload),
            Form::Node { level, arcs } => Form::Node {
                level,
                arcs: arcs
                    .into_iter()
                    .map(|(key, target)| (self.controleur.keys().normal_form(&key), target))
                    .collect(),
            },
        }
    }

    /// Reductions at handle creation (spec § 2.3, M1-2): R-collapse and
    /// R-default, the default diagram D_δ being designated by the controller.
    /// Returns either the collapse target or the reduced form to intern.
    fn reduced(
        &self,
        anchor: Anchor,
        level: Level,
        arcs: BTreeMap<C::Key, NodeId>,
    ) -> Result<Form<C::Key, C::Payload>, NodeId> {
        // Didactic workshop (backlog § 9): the controller can toggle
        // R-default and R-collapse one at a time.
        let default_active = self.controleur.default_active();
        let key_defaut = if default_active {
            self.controleur.keys().default(level)
        } else {
            None
        };
        let cardinality = self.controleur.keys().cardinality(level);
        let payload_defaut = if default_active {
            self.controleur.default_payload(level)
        } else {
            None
        };
        let est_defaut = |node: NodeId| -> bool {
            default_active
                && matches!(self.resolve(node), Ok(Form::Terminal(payload)) if payload_defaut.as_ref() == Some(payload))
        };
        // M4-3: generalized default (D-abs/D-trans) if the controller enables it.
        let reduced = if !self.controleur.collapse_active() {
            reduce_without_collapse(arcs, key_defaut, &est_defaut)
        } else if self.controleur.generalized_absorption() {
            // NodeId of D^default under this anchor, if already interned (used
            // for D-trans on empty form A = ∅).
            let defaut = payload_defaut.as_ref().and_then(|p| {
                self.unique
                    .get(&(Self::CONTROLLER, anchor, Form::Terminal(p.clone())))
                    .copied()
            });
            crate_reduction_dabs(arcs, key_defaut, &est_defaut, cardinality, defaut)
        } else {
            reduce(arcs, key_defaut, &est_defaut, cardinality)
        };
        match reduced {
            Reduction::Collapse(target) => Err(target),
            Reduction::Form(arcs_reduceds) => Ok(Form::Node {
                level,
                arcs: arcs_reduceds,
            }),
        }
    }
}

impl<C: Controller> Pool for Arena<C> {
    type Key = C::Key;
    type Payload = C::Payload;

    const CONTROLLER: ControllerId = C::FINGERPRINT;

    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId {
        let form = self.form_normalisee(form);
        let form = match form {
            Form::Node { level, arcs } => {
                let size_avant = arcs.len();
                match self.reduced(anchor, level, arcs) {
                    Err(target) => {
                        self.observer.on_event(&TraceEvent::ReductionApplied {
                            rule: ReductionRule::Collapse,
                            level,
                        });
                        return target;
                    }
                    Ok(Form::Node { arcs, .. }) => {
                        if arcs.len() < size_avant {
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
        let key = (Self::CONTROLLER, anchor, form.clone());
        if let Some(node) = self.unique.get(&key) {
            self.metriques.partages += 1;
            self.observer
                .on_event(&TraceEvent::NodeShared { node: *node });
            return *node;
        }
        let node = NodeId(self.nodes.len() as u32);
        let level = match &form {
            Form::Terminal(_) => None,
            Form::Node { level, .. } => Some(*level),
        };
        self.nodes.push(NodeEntry {
            plan: Plan::Principal,
            form: Some(form),
        });
        if let Some(level) = level {
            self.index_levels.entry(level).or_default().insert(node);
        }
        self.unique.insert(key, node);
        self.metriques.creations += 1;
        self.observer
            .on_event(&TraceEvent::NodeCreated { node, level });
        node
    }

    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError> {
        match self.nodes.get(node.0 as usize) {
            Some(input) => input.form.as_ref().ok_or(PoolError::UnknownNode(node)),
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
