//! Arena: two-plane hash-consed pool, implementation of the `Pool` port
//! (M1-1, M4-1). Main and shadow planes (spec § 10 Q7), mark-and-sweep GC
//! with profiled compaction, cache metrics.
//!
//! Hash-consing keyed by (controller, anchor, form): R-share is the unique
//! table. At handle creation, the controller's reductions apply
//! (M1-2, spec § 2.3): R-collapse and R-default.

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_algebra::reduction::{
    Reduction, reduce, reduce_dabs as crate_reduction_dabs, reduce_without_collapse,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::biplan::{IndexLevels, InputNode, MetricsPool, Plan, StatsGc, threshold_reached};
use crate::observer::{Observer, ReductionRule, TraceEvent};

/// Hash-consing key: (controller, anchor, form).
type KeyInterning<Key, Payload> = (ControllerId, Anchor, Form<Key, Payload>);

/// Arena: blind canonical storage of forms, addressed by content.
/// Holds the controller instance: its key algebra (parameterized by the
/// schema, M1-3) decides normal forms, cardinalities and defaults.
pub struct Arena<C: Controller> {
    nodes: Vec<InputNode<C::Key, C::Payload>>,
    unique: HashMap<KeyInterning<C::Key, C::Payload>, NodeId>,
    /// Shadow plane index: level → live nodes of that level.
    index_levels: IndexLevels,
    metrics: MetricsPool,
    /// Remapping table of the last compaction (empty otherwise).
    remap: Vec<Option<NodeId>>,
    observer: Box<dyn Observer>,
    controller: C,
}

impl<C: Controller> Arena<C> {
    /// New arena with the given controller and trace observer.
    pub fn new(controller: C, observer: Box<dyn Observer>) -> Self {
        Arena {
            nodes: Vec::new(),
            unique: HashMap::new(),
            index_levels: IndexLevels::new(),
            metrics: MetricsPool::default(),
            remap: Vec::new(),
            observer,
            controller,
        }
    }

    /// Controller owning this arena.
    pub fn controller(&self) -> &C {
        &self.controller
    }

    /// Interning of a terminal with payload (main plane).
    pub fn terminal(&mut self, anchor: Anchor, payload: C::Payload) -> NodeId {
        self.intern(anchor, Form::Terminal(payload))
    }

    /// Interning into a given plane (main by default via `intern`).
    /// On a hash-consing hit (form already present), the existing handle
    /// is returned and its plane is moved to `plan`.
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
    pub fn plan_of(&self, node: NodeId) -> Option<Plan> {
        self.nodes
            .get(node.0 as usize)?
            .form
            .as_ref()
            .map(|_| self.nodes[node.0 as usize].plan)
    }

    /// Live nodes of a level (shadow plane index).
    pub fn index_level(&self, level: Level) -> &BTreeSet<NodeId> {
        static EMPTY: std::sync::LazyLock<BTreeSet<NodeId>> =
            std::sync::LazyLock::new(BTreeSet::new);
        self.index_levels.get(&level).unwrap_or(&EMPTY)
    }

    /// Pool metrics (cache + GC).
    pub fn metrics(&self) -> MetricsPool {
        self.metrics
    }

    /// Remapping table of the last compaction: `remap[old] = new`
    /// (`None` if the old node was a tombstone). Empty if the last GC
    /// did not compact.
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
            if let Some(InputNode {
                form: Some(Form::Node { edges, .. }),
                ..
            }) = self.nodes.get(node.0 as usize)
            {
                stack.extend(edges.values().copied());
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
        // Live counted on entries actually alive (and not derived from the
        // number of slots, which includes tombstones from a previous GC
        // without compaction).
        let live = self.nodes.iter().filter(|e| e.form.is_some()).count() as u64;
        let compaction = threshold_reached(collected, total);
        if compaction {
            self.remap = self.compacter();
        } else {
            self.remap.clear();
        }
        self.metrics.gc += 1;
        self.metrics.live = live;
        self.metrics.dead += collected;
        if compaction {
            self.metrics.compactions += 1;
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

    /// Compaction: rebuilds the node vector without tombstones and
    /// remaps the `NodeId`s (remapping table returned). Reserved for the
    /// `gc` path, which first cleans tombstones from the unique table:
    /// a direct call with tombstones would leave stale `NodeId`s
    /// in `unique`.
    fn compacter(&mut self) -> Vec<Option<NodeId>> {
        self.remap.clear();
        let mut remap: Vec<Option<NodeId>> = Vec::with_capacity(self.nodes.len());
        let mut nouveaux: Vec<InputNode<C::Key, C::Payload>> = Vec::new();
        for input in self.nodes.drain(..) {
            if input.form.is_some() {
                remap.push(Some(NodeId(nouveaux.len() as u32)));
                nouveaux.push(input);
            } else {
                remap.push(None);
            }
        }
        self.nodes = nouveaux;
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
    fn form_normalized(&self, form: Form<C::Key, C::Payload>) -> Form<C::Key, C::Payload> {
        match form {
            Form::Terminal(payload) => Form::Terminal(payload),
            Form::Node { level, edges } => Form::Node {
                level,
                edges: edges
                    .into_iter()
                    .map(|(key, target)| (self.controller.keys().normal_form(&key), target))
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
        edges: BTreeMap<C::Key, NodeId>,
    ) -> Result<Form<C::Key, C::Payload>, NodeId> {
        // Didactic workshop (backlog § 9): the controller can toggle
        // R-default and R-collapse one at a time.
        let default_active = self.controller.default_active();
        let key_default = if default_active {
            self.controller.keys().default(level)
        } else {
            None
        };
        let cardinality = self.controller.keys().cardinality(level);
        let payload_default = if default_active {
            self.controller.default_payload(level)
        } else {
            None
        };
        let is_default = |node: NodeId| -> bool {
            default_active
                && matches!(self.resolve(node), Ok(Form::Terminal(payload)) if payload_default.as_ref() == Some(payload))
        };
        // M4-3: generalized default (D-abs/D-trans) if the controller enables it.
        let reduced = if !self.controller.collapse_active() {
            reduce_without_collapse(edges, key_default, &is_default)
        } else if self.controller.absorption_generalized() {
            // NodeId of D^default under this anchor, if already interned (used
            // for D-trans on empty form A = ∅).
            let default = payload_default.as_ref().and_then(|p| {
                self.unique
                    .get(&(Self::CONTROLLER, anchor, Form::Terminal(p.clone())))
                    .copied()
            });
            crate_reduction_dabs(edges, key_default, &is_default, cardinality, default)
        } else {
            reduce(edges, key_default, &is_default, cardinality)
        };
        match reduced {
            Reduction::Collapse(target) => Err(target),
            Reduction::Form(edges_reduced) => Ok(Form::Node {
                level,
                edges: edges_reduced,
            }),
        }
    }
}

impl<C: Controller> Pool for Arena<C> {
    type Key = C::Key;
    type Payload = C::Payload;

    const CONTROLLER: ControllerId = C::FINGERPRINT;

    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId {
        let form = self.form_normalized(form);
        let form = match form {
            Form::Node { level, edges } => {
                let size_before = edges.len();
                match self.reduced(anchor, level, edges) {
                    Err(target) => {
                        self.observer.on_event(&TraceEvent::ReductionApplied {
                            rule: ReductionRule::Collapse,
                            level,
                        });
                        return target;
                    }
                    Ok(Form::Node { edges, .. }) => {
                        if edges.len() < size_before {
                            self.observer.on_event(&TraceEvent::ReductionApplied {
                                rule: ReductionRule::Default,
                                level,
                            });
                        }
                        Form::Node { level, edges }
                    }
                    Ok(terminal) => terminal,
                }
            }
            terminal => terminal,
        };
        let key = (Self::CONTROLLER, anchor, form.clone());
        if let Some(node) = self.unique.get(&key) {
            self.metrics.shared += 1;
            self.observer
                .on_event(&TraceEvent::NodeShared { node: *node });
            return *node;
        }
        let node = NodeId(self.nodes.len() as u32);
        let level = match &form {
            Form::Terminal(_) => None,
            Form::Node { level, .. } => Some(*level),
        };
        self.nodes.push(InputNode {
            plan: Plan::Main,
            form: Some(form),
        });
        if let Some(level) = level {
            self.index_levels.entry(level).or_default().insert(node);
        }
        self.unique.insert(key, node);
        self.metrics.creations += 1;
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
