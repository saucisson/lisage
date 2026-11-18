//! Arena: two-plane hash-consed pool, implementation of the `Pool` port
//! (M1-1, M4-1). Main and shadow planes (spec § 10 Q7), mark-and-sweep GC
//! with profiled compaction, cache metrics.
//!
//! Keyed hash-consing (controller, anchor, form): R-share is the unique
//! table. At handle creation, the controller's reductions apply (M1-2,
//! spec § 2.3): R-collapse and R-default.

use lisage_algebra::class_bits::TERMINAL_CLASS;
use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool,
    PoolError,
};
use lisage_algebra::reduction::{
    Reduction, reduce, reduce_dabs as crate_reduction_dabs, reduce_without_collapse,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::biplan::{IndexLevels, MetriquesPool, NodeEntry, Plan, StatsGc, threshold_atteint};
use crate::classes::{ClassArena, ClassMap, ClassMetrics};
use crate::observer::{Observer, ReductionRule, TraceEvent};
use crate::unique::{TableUnique, hash_cle};

/// Arena: canonical, content-addressed storage of forms. Holds the
/// controller instance: its key algebra (parameterized by the schema, M1-3)
/// decides normal forms, cardinalities and defaults.
pub struct Arena<C: Controller> {
    /// Frozen class map (MEM-9, ADR-0103): class bits + index bits of the
    /// `NodeId`, fixed at construction.
    class_map: ClassMap,
    /// Per-class arenas, created lazily (MEM-9).
    arenas: Vec<Option<ClassArena<C::Key, C::Payload>>>,
    /// Unique table (MEM-14): open addressing of `(hash, anchor, NodeId)`
    /// — the semantic form is compared through the stored entry, never
    /// cloned into the key; `ControllerId` is constant per pool and out
    /// of the key.
    unique: TableUnique,
    /// Terminal singletons of the pool (MEM-11): keyed by payload only —
    /// canonicity by handle concerns internal nodes, terminals are
    /// pool-wide singletons, shared across anchors.
    unique_terminals: HashMap<C::Payload, NodeId>,
    /// Shadow-plane index: level → live nodes of that level.
    index_levels: IndexLevels,
    metriques: MetriquesPool,
    /// Remapping table of the last compaction, keyed by the old raw
    /// `NodeId` (empty otherwise).
    remap: BTreeMap<u32, Option<NodeId>>,
    observer: Box<dyn Observer>,
    controleur: C,
}

impl<C: Controller> Arena<C> {
    /// New arena with the given controller and trace observer. The class
    /// map is derived from the controller's key algebra and frozen here.
    pub fn new(controleur: C, observer: Box<dyn Observer>) -> Self {
        let class_map = ClassMap::new(controleur.keys(), controleur.keys().sweep_levels());
        let arenas = (0..class_map.number_classes()).map(|_| None).collect();
        Arena {
            class_map,
            arenas,
            unique: TableUnique::new(),
            unique_terminals: HashMap::new(),
            index_levels: IndexLevels::new(),
            metriques: MetriquesPool::default(),
            remap: BTreeMap::new(),
            observer,
            controleur,
        }
    }

    /// Class map of the pool (frozen at construction, MEM-9).
    pub fn class_map(&self) -> ClassMap {
        self.class_map
    }

    /// Per-class metrics: live and peak per class (traced, MEM-9).
    pub fn class_metrics(&self) -> Vec<ClassMetrics> {
        self.arenas
            .iter()
            .enumerate()
            .filter_map(|(class, arena)| arena.as_ref().map(|a| a.metrics(class as u32)))
            .collect()
    }

    /// Decode a `NodeId` into `(class, index)` with the effective width.
    fn decode(&self, node: NodeId) -> (u32, u32) {
        node.decode(self.class_map.class_bits)
    }

    /// Entry of a node, if the identifier is valid.
    fn entry(&self, node: NodeId) -> Option<&NodeEntry<C::Key, C::Payload>> {
        let (class, index) = self.decode(node);
        self.arenas.get(class as usize)?.as_ref()?.get(index)
    }

    /// Lazily-created arena of a class.
    fn arena_mut(&mut self, class: u32) -> &mut ClassArena<C::Key, C::Payload> {
        self.arenas
            .get_mut(class as usize)
            .expect("class within the frozen map")
            .get_or_insert_with(ClassArena::new)
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
        let (class, index) = self.decode(node);
        if let Some(input) = self
            .arenas
            .get_mut(class as usize)
            .and_then(|arena| arena.as_mut())
            .and_then(|arena| arena.get_mut(index))
        {
            input.plan = plan;
        }
        node
    }

    /// Plane of a live node.
    pub fn plan_de(&self, node: NodeId) -> Option<Plan> {
        self.entry(node)
            .and_then(|input| input.form.as_ref().map(|_| input.plan))
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

    /// Remapping table of the last compaction, keyed by the old raw
    /// `NodeId`: `remap[old] = Some(new)` (`None` if the old node was a
    /// tombstone). Empty if the last GC did not compact.
    pub fn last_remap(&self) -> &BTreeMap<u32, Option<NodeId>> {
        &self.remap
    }

    /// Number of keys in the unique table (MEM-14).
    pub fn unique_len(&self) -> usize {
        self.unique.len()
    }

    /// End-to-end memory per live node in bytes (MEM-14 counter): unique
    /// table + class arenas + shadow index, over the live node count.
    pub fn bytes_per_node(&self) -> f64 {
        let live = self.live().max(1) as f64;
        let unique = self.unique.bytes() as f64;
        let arenas: usize = self
            .arenas
            .iter()
            .flatten()
            .map(|arena| arena.bytes())
            .sum();
        let index: usize = self
            .index_levels
            .values()
            .map(|set| set.len() * size_of::<NodeId>())
            .sum();
        (unique + arenas as f64 + index as f64) / live
    }

    /// Number of live nodes.
    pub fn live(&self) -> usize {
        self.arenas
            .iter()
            .flatten()
            .map(|arena| arena.live_count() as usize)
            .sum()
    }

    /// Mark-and-sweep GC from the given roots: unreachable nodes become
    /// tombstones (removed from the unique table and the index); if the
    /// dead threshold is reached, profiled compaction with a `NodeId`
    /// remapping table.
    pub fn gc(&mut self, roots: &[NodeId]) -> StatsGc {
        // Marking: 1 bit per node, one bitmap per class arena (MEM-10),
        // replacing the old `BTreeSet<NodeId>`.
        let class_bits = self.class_map.class_bits;
        let mut marks: Vec<Option<Vec<u64>>> = self
            .arenas
            .iter()
            .map(|arena| arena.as_ref().map(|a| a.empty_marks()))
            .collect();
        let mut stack: Vec<NodeId> = roots.to_vec();
        while let Some(node) = stack.pop() {
            let (class, index) = node.decode(class_bits);
            let Some(Some(words)) = marks.get_mut(class as usize) else {
                continue;
            };
            if ClassArena::<C::Key, C::Payload>::mark_of(words, index) {
                continue;
            }
            ClassArena::<C::Key, C::Payload>::set_mark(words, index);
            if let Some(NodeEntry {
                form: Some(Form::Node { arcs, .. }),
                ..
            }) = self.entry(node)
            {
                stack.extend(arcs.values().copied());
            }
        }
        let mark_bytes: u64 = marks
            .iter()
            .flatten()
            .map(|words| words.len() as u64 * 8)
            .sum();
        let total = self.node_count() as u64;
        let mut collected = 0u64;
        for class in 0..self.class_map.number_classes() {
            if let Some(arena) = self.arenas.get_mut(class as usize).and_then(|a| a.as_mut()) {
                let words = marks[class as usize].as_ref().expect("marks allocated");
                collected += arena.sweep(words);
            }
        }
        // Removal of tombstones from the unique table and the shadow index.
        let arenas = &self.arenas;
        let is_live = |node: NodeId| -> bool {
            let (class, index) = node.decode(class_bits);
            arenas
                .get(class as usize)
                .and_then(|arena| arena.as_ref())
                .and_then(|arena| arena.get(index))
                .is_some_and(|entry| entry.form.is_some())
        };
        self.unique.retain(&is_live);
        self.unique_terminals.retain(|_, node| is_live(*node));
        for set in self.index_levels.values_mut() {
            set.retain(|node| is_live(*node));
        }
        // Live nodes counted on actually-live entries (not derived from the
        // slot count, which includes tombstones from a previous GC without
        // compaction).
        let live = self.live() as u64;
        let compaction = threshold_atteint(collected, total);
        if compaction {
            self.remap = self.compacter();
        } else {
            self.remap.clear();
            // Page rendering: an emptied class releases its slabs, keeping
            // the slot count as high-water mark (no index recycled outside
            // the remap protocol, MEM-10).
            for arena in self.arenas.iter_mut().flatten() {
                if arena.live_count() == 0 {
                    arena.release_pages();
                }
            }
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
            mark_bytes,
        };
        self.observer.on_event(&TraceEvent::Gc {
            live,
            collected,
            compaction,
        });
        stats
    }

    /// Compaction: rebuilds each class arena without tombstones and remaps
    /// the `NodeId`s (returned remapping table, keyed by the old raw id).
    /// Reserved for the `gc` path, which first cleans the tombstones from
    /// the unique table: a direct call with tombstones would leave stale
    /// `NodeId`s in `unique`.
    fn compacter(&mut self) -> BTreeMap<u32, Option<NodeId>> {
        let mut remap = BTreeMap::new();
        let class_bits = self.class_map.class_bits;
        for class in 0..self.class_map.number_classes() {
            let Some(arena) = self.arenas.get_mut(class as usize).and_then(|a| a.as_mut()) else {
                continue;
            };
            let index_remap = arena.compact();
            for (old_index, new_index) in index_remap.into_iter().enumerate() {
                let old = NodeId::encode(class, old_index as u32, class_bits);
                remap.insert(
                    old.0,
                    new_index.map(|new| NodeId::encode(class, new, class_bits)),
                );
            }
        }
        for node in self.unique.values_mut() {
            if let Some(Some(new)) = remap.get(&node.0) {
                *node = *new;
            }
        }
        for set in self.index_levels.values_mut() {
            *set = set
                .iter()
                .filter_map(|node| remap.get(&node.0).copied().flatten())
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
        _anchor: Anchor,
        level: Level,
        arcs: ArcRun<C::Key>,
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
            let defaut = payload_defaut
                .as_ref()
                .and_then(|p| self.unique_terminals.get(p).copied());
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
        // MEM-11: terminals are pool-wide singletons (class 0), keyed by
        // payload only — the anchor does not enter their canonicity.
        if let Form::Terminal(payload) = &form {
            if let Some(node) = self.unique_terminals.get(payload) {
                self.metriques.partages += 1;
                self.observer
                    .on_event(&TraceEvent::NodeShared { node: *node });
                return *node;
            }
            let index = self.arena_mut(TERMINAL_CLASS).push(NodeEntry {
                plan: Plan::Principal,
                form: Some(form.clone()),
            });
            let node = NodeId::encode(TERMINAL_CLASS, index, self.class_map.class_bits);
            self.unique_terminals.insert(payload.clone(), node);
            self.metriques.terminaux += 1;
            self.metriques.creations += 1;
            self.observer
                .on_event(&TraceEvent::NodeCreated { node, level: None });
            return node;
        }
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
                        // MEM-12: the dense ordinal family is chosen in
                        // normalization, after the reductions so that the
                        // choice survives on the stored (hash-consed) form.
                        let arcs = ArcRun::choose_dense_family(level, self.controleur.keys(), arcs);
                        Form::Node { level, arcs }
                    }
                    Ok(terminal) => terminal,
                }
            }
            terminal => terminal,
        };
        // MEM-14: the hash-consing key is still the semantic form
        // (anchor, form), hashed in place — no form clone in the table.
        let hash = hash_cle(anchor, &form);
        let meme_forme = |node: NodeId| {
            self.entry(node)
                .and_then(|input| input.form.as_ref())
                .is_some_and(|stored| *stored == form)
        };
        if let Some(node) = self.unique.get(hash, anchor, meme_forme) {
            self.metriques.partages += 1;
            self.observer.on_event(&TraceEvent::NodeShared { node });
            return node;
        }
        // MEM-9: the node is routed to the arena of its class (degree),
        // the NodeId carries class bits + index bits.
        let (degree, level) = match &form {
            Form::Terminal(_) => (0, None),
            Form::Node { level, arcs } => (arcs.len() as u32, Some(*level)),
        };
        let class = self.class_map.class_of_degree(degree);
        let index = self.arena_mut(class).push(NodeEntry {
            plan: Plan::Principal,
            form: Some(form),
        });
        let node = NodeId::encode(class, index, self.class_map.class_bits);
        if let Some(level) = level {
            self.index_levels.entry(level).or_default().insert(node);
        }
        self.unique.insert(hash, anchor, node);
        self.metriques.creations += 1;
        self.observer
            .on_event(&TraceEvent::NodeCreated { node, level });
        node
    }

    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError> {
        match self.entry(node).and_then(|input| input.form.as_ref()) {
            Some(form) => Ok(form),
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
        self.arenas
            .iter()
            .flatten()
            .map(|arena| arena.slot_count())
            .sum()
    }
}
