//! Core interface (M1-0): contracts seen by `algebra` from the `core` pool,
//! blind canonical storage (forms and handles). `Send`/`Sync` bounds
//! frozen from M1-0 (roadmap § 10).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

/// Node identifier in the pool, in creation order, never moved.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub u32);

/// Anchor: absolute position of the root (absolute encoding in M1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Anchor(pub u32);

/// Controller fingerprint (hash of the eight fields, proposal § 1.2):
/// several controllers coexist in the same pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ControllerId(pub u64);

/// Handle = (controller, anchor, form): canonicity migrates from the node to
/// the handle (spec § 2.4.3, claim C1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Handle {
    pub controller: ControllerId,
    pub anchor: Anchor,
    pub node: NodeId,
}

/// Decomposition level: a key of the swept universe, ordered.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Level(pub u32);

/// Form of a node: terminal with payload, or node with explicit arcs (the
/// default arc δ(ℓ) is stored like the others; R-default makes it implicit).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Form<Key, Payload> {
    Terminal(Payload),
    Node { level: Level, arcs: ArcRun<Key> },
}

/// Arc storage port (MEM-3, ADR-0103): the minimal contract the pool and
/// the reductions see — ordered iteration of `(key, target)` pairs, plus
/// the size. Insertion lives on the builder side ([`ArcStoreBuilder`]).
/// The default instance is the current `BTreeMap<Key, NodeId>`; another
/// layout (sorted run, MEM-5) implements the same port.
pub trait ArcStore: Send + Sync {
    /// Arc key: decision value of a level.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Builder producing this store (insertion side).
    type Builder: ArcStoreBuilder<Store = Self>;

    /// Ordered iteration of the `(key, target)` pairs.
    fn iter(&self) -> impl Iterator<Item = (&Self::Key, &NodeId)>;

    /// Number of arcs.
    fn len(&self) -> usize;

    /// True when the store holds no arc.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builder side of an [`ArcStore`]: insertion of `(key, target)` pairs,
/// then build. The reductions and transductions never see this side.
pub trait ArcStoreBuilder {
    /// Arc key: decision value of a level.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Store produced by [`ArcStoreBuilder::build`].
    type Store: ArcStore<Key = Self::Key>;

    /// Insert one `(key, target)` pair.
    fn insert(&mut self, key: Self::Key, target: NodeId);

    /// Build the store.
    fn build(self) -> Self::Store;
}

impl<Key> ArcStore for BTreeMap<Key, NodeId>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Key = Key;
    type Builder = Self;

    fn iter(&self) -> impl Iterator<Item = (&Self::Key, &NodeId)> {
        BTreeMap::iter(self)
    }

    fn len(&self) -> usize {
        BTreeMap::len(self)
    }
}

impl<Key> ArcStoreBuilder for BTreeMap<Key, NodeId>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Key = Key;
    type Store = Self;

    fn insert(&mut self, key: Self::Key, target: NodeId) {
        BTreeMap::insert(self, key, target);
    }

    fn build(self) -> Self::Store {
        self
    }
}

/// Payload of the dense ordinal family (MEM-12, ALIGN-2).
///
/// Holds one `Option<NodeId>` slot per rank of the level (8 bytes — the key
/// is elided; by ordinal construction the key of slot `r` is `keys[r]`),
/// plus the shared per-level enumeration that re-emits the keys by rank
/// (rank/unrank of MEM-16). Boxed so the `ArcRun` enum keeps its 16-byte
/// layout: the per-slot saving stays out of the inline entry size.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DenseRun<Key> {
    /// Shared enumeration of the level's domain in rank order, one `Arc`
    /// per level cloned per node (no per-node allocation).
    pub keys: Arc<[Key]>,
    /// Targets by rank; `None` = hole = default not stored.
    pub slots: Box<[Option<NodeId>]>,
}

/// Arc storage of a node (MEM-5 sorted run, MEM-12 dense ordinal,
/// ADR-0103): two families behind the same type, the family being chosen
/// in normalization (see [`ArcRun::choose_dense_family`]).
/// - [`ArcRun::Sorted`]: `Box<[(key, target)]>` sorted by key, one entry
///   per key — the default family, built by [`ArcRun::from_pairs`].
/// - [`ArcRun::Dense`]: one slot per rank of the level, holes = default
///   not stored — the dense ordinal family (MEM-12).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ArcRun<Key> {
    /// Sorted run (MEM-5): the default family.
    Sorted(Box<[(Key, NodeId)]>),
    /// Dense ordinal family (MEM-12): one slot per rank of the level,
    /// `None` = hole = default not stored. ALIGN-2: the key is elided from
    /// the slots (`Option<NodeId>`, 8 bytes) and re-emitted by rank from
    /// the shared enumeration instead of being stored per slot.
    Dense(Box<DenseRun<Key>>),
}

impl<Key> Default for ArcRun<Key> {
    fn default() -> Self {
        ArcRun::Sorted(Box::default())
    }
}

impl<Key> ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    /// Build a sorted run from pairs, keeping the LAST target of each key.
    /// Vec + sort + dedup (PERF-2): one allocation for the common
    /// duplicate-free case with an exact size hint, against one tree-node
    /// allocation per pair for the previous `BTreeMap` build.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (Key, NodeId)>) -> Self {
        let iter = pairs.into_iter();
        let (lower, upper) = iter.size_hint();
        let mut vec: Vec<(Key, NodeId)> = Vec::with_capacity(upper.unwrap_or(lower));
        vec.extend(iter);
        vec.sort_by(|a, b| a.0.cmp(&b.0));
        // Last write wins: on equal keys, the later pair overwrites the
        // retained one and is then removed.
        vec.dedup_by(|later, retained| {
            if later.0 == retained.0 {
                retained.1 = later.1;
                true
            } else {
                false
            }
        });
        ArcRun::Sorted(vec.into_boxed_slice())
    }

    /// MEM-12: choose the dense ordinal family when every key of `arcs`
    /// has a rank in `keys` at `level` and the fill is at least one third
    /// (threshold `4·|Val(ℓ)| ≤ 12·arcs`); otherwise keep the sorted run.
    /// The choice is deterministic in (level, keys, content), so C1 holds:
    /// the same semantic form always lands in the same family.
    pub fn choose_dense_family<K: KeyAlgebra<Key = Key>>(
        level: Level,
        keys: &K,
        arcs: ArcRun<Key>,
    ) -> ArcRun<Key> {
        let ArcRun::Sorted(pairs) = &arcs else {
            return arcs;
        };
        let cardinality = keys.cardinality(level);
        let arcs_len = pairs.len() as u64;
        // Checked arithmetic: an open domain (cardinality near u64::MAX)
        // can never satisfy the threshold — never dense, no overflow.
        if cardinality
            .checked_mul(4)
            .is_none_or(|c4| c4 > 12 * arcs_len)
        {
            return arcs;
        }
        // ALIGN-2: the key is elided from the slots; the shared per-level
        // enumeration (rank/unrank of MEM-16) re-emits it by rank.
        let table = keys.enumeration(level);
        let mut slots: Vec<Option<NodeId>> = vec![None; table.len()];
        for (key, target) in pairs.iter() {
            match keys.rank(level, key) {
                Some(rank) if (rank as usize) < slots.len() => {
                    slots[rank as usize] = Some(*target);
                }
                _ => return arcs,
            }
        }
        ArcRun::Dense(Box::new(DenseRun {
            keys: table,
            slots: slots.into_boxed_slice(),
        }))
    }

    /// Is this the dense ordinal family (MEM-12)?
    pub fn is_dense(&self) -> bool {
        matches!(self, ArcRun::Dense(_))
    }

    /// Target of `key`, if present.
    pub fn get(&self, key: &Key) -> Option<&NodeId> {
        match self {
            ArcRun::Sorted(pairs) => pairs
                .binary_search_by(|(k, _)| k.cmp(key))
                .ok()
                .map(|index| &pairs[index].1),
            ArcRun::Dense(dense) => dense
                .keys
                .iter()
                .position(|k| k == key)
                .and_then(|rank| dense.slots.get(rank).and_then(|slot| slot.as_ref())),
        }
    }

    /// Is `key` present?
    pub fn contains_key(&self, key: &Key) -> bool {
        self.get(key).is_some()
    }

    /// Ordered iteration of the `(key, target)` pairs (key order for the
    /// sorted run, rank order for the dense family). Concrete iterator
    /// (PERF-2): the intern path calls `iter()` several times per
    /// interning (unique-table probes, normalization, reductions) and each
    /// boxed `dyn` iterator was one heap allocation.
    pub fn iter(&self) -> ArcRunIter<'_, Key> {
        ArcRunIter::new(self)
    }

    /// Iteration over the targets, in key/rank order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &NodeId> {
        self.iter().map(|(_, target)| target)
    }

    /// Iteration over the keys, in order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &Key> {
        self.iter().map(|(key, _)| key)
    }

    /// Number of arcs.
    pub fn len(&self) -> usize {
        match self {
            ArcRun::Sorted(pairs) => pairs.len(),
            ArcRun::Dense(dense) => dense.slots.iter().filter(|slot| slot.is_some()).count(),
        }
    }

    /// True when the run holds no arc.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Allocation-free double-ended iteration over an [`ArcRun`] (PERF-2):
/// scans the underlying storage between `front` and `back`, skipping the
/// holes of the dense family.
pub struct ArcRunIter<'a, Key> {
    run: &'a ArcRun<Key>,
    front: usize,
    back: usize,
}

impl<'a, Key> ArcRunIter<'a, Key> {
    fn new(run: &'a ArcRun<Key>) -> Self {
        let len = match run {
            ArcRun::Sorted(pairs) => pairs.len(),
            ArcRun::Dense(dense) => dense.slots.len(),
        };
        ArcRunIter {
            run,
            front: 0,
            back: len,
        }
    }

    fn get(&self, index: usize) -> Option<(&'a Key, &'a NodeId)> {
        match self.run {
            ArcRun::Sorted(pairs) => pairs.get(index).map(|(key, target)| (key, target)),
            // ALIGN-2: the key of slot `index` is re-emitted by rank from
            // the shared enumeration — the iteration order (rank order)
            // and the yielded pairs are exactly those of the stored form.
            ArcRun::Dense(dense) => dense
                .slots
                .get(index)
                .and_then(|slot| slot.as_ref().map(|target| (&dense.keys[index], target))),
        }
    }
}

impl<'a, Key> Iterator for ArcRunIter<'a, Key> {
    type Item = (&'a Key, &'a NodeId);

    fn next(&mut self) -> Option<Self::Item> {
        while self.front < self.back {
            let index = self.front;
            self.front += 1;
            if let Some(pair) = self.get(index) {
                return Some(pair);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.back - self.front))
    }
}

impl<'a, Key> DoubleEndedIterator for ArcRunIter<'a, Key> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.front < self.back {
            self.back -= 1;
            if let Some(pair) = self.get(self.back) {
                return Some(pair);
            }
        }
        None
    }
}

impl<Key> std::iter::FromIterator<(Key, NodeId)> for ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    /// Collect with "last write wins" semantics, then sort.
    fn from_iter<T: IntoIterator<Item = (Key, NodeId)>>(iter: T) -> Self {
        ArcRun::from_pairs(iter)
    }
}

impl<Key> IntoIterator for ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Item = (Key, NodeId);
    type IntoIter = std::vec::IntoIter<(Key, NodeId)>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ArcRun::Sorted(pairs) => pairs.into_vec().into_iter(),
            ArcRun::Dense(dense) => dense
                .slots
                .into_vec()
                .into_iter()
                .enumerate()
                .filter_map(|(rank, slot)| slot.map(|target| (dense.keys[rank].clone(), target)))
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }
}

impl<'a, Key> IntoIterator for &'a ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Item = (&'a Key, &'a NodeId);
    type IntoIter = Box<dyn Iterator<Item = (&'a Key, &'a NodeId)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

impl<Key> From<BTreeMap<Key, NodeId>> for ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    fn from(map: BTreeMap<Key, NodeId>) -> Self {
        ArcRun::Sorted(map.into_iter().collect())
    }
}

impl<Key> std::ops::Index<&Key> for ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Output = NodeId;

    fn index(&self, key: &Key) -> &Self::Output {
        self.get(key).expect("arc key absent from the run")
    }
}

impl<Key> ArcStore for ArcRun<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Key = Key;
    type Builder = ArcRunBuilder<Key>;

    fn iter(&self) -> impl Iterator<Item = (&Self::Key, &NodeId)> {
        self.iter()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

/// Builder of an [`ArcRun`]: insertion with "last write wins" semantics,
/// sorted run produced by [`ArcStoreBuilder::build`].
pub struct ArcRunBuilder<Key> {
    map: BTreeMap<Key, NodeId>,
}

impl<Key> Default for ArcRunBuilder<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    fn default() -> Self {
        ArcRunBuilder {
            map: BTreeMap::new(),
        }
    }
}

impl<Key> ArcStoreBuilder for ArcRunBuilder<Key>
where
    Key: Clone + Ord + Hash + Debug + Send + Sync,
{
    type Key = Key;
    type Store = ArcRun<Key>;

    fn insert(&mut self, key: Self::Key, target: NodeId) {
        self.map.insert(key, target);
    }

    fn build(self) -> Self::Store {
        ArcRun::from(self.map)
    }
}

/// Typed error of the pool port: unknown identifiers, cause preserved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PoolError {
    UnknownNode(NodeId),
    UnknownHandle(Handle),
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::UnknownNode(node) => write!(formatter, "unknown node: {node:?}"),
            PoolError::UnknownHandle(handle) => write!(formatter, "unknown handle: {handle:?}"),
        }
    }
}

impl std::error::Error for PoolError {}

/// Key algebra: contract carried by the controller, never by the pool.
/// Instance methods from M1-3: the algebra is parameterized by the
/// controller's schema (finite domains of levels), runtime data.
pub trait KeyAlgebra: Send + Sync {
    /// Arc key: decision value of a level.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Normal form of a key (otherwise hash-consing would violate C1).
    fn normal_form(&self, key: &Self::Key) -> Self::Key;

    /// Cardinality |Val(ℓ)|, which enters the fold aggregation (A4).
    fn cardinality(&self, level: Level) -> u64;

    /// Default value δ(ℓ) of the level, if the level has one.
    fn default(&self, level: Level) -> Option<Self::Key>;

    /// Ordinal rank of a key in the level's enumeration (MEM-4, ADR-0103),
    /// derived from the region enumeration the key contract requires.
    /// `Some(rank)` when the level's domain is bounded and the key belongs
    /// to it; `None` otherwise. The default `None` declares the absence of
    /// rank properly (unbounded domain: the DDD path is unchanged), so the
    /// dense ordinal storage (MEM-B) writes no code per controller.
    /// Rank availability is exposed to the normalization: `normal_form`
    /// may encode keys by their rank when it exists. ALIGN-2: the dense
    /// family also needs [`KeyAlgebra::unrank`] (via
    /// [`KeyAlgebra::enumeration`]) to re-emit the key of a slot by its
    /// rank — an algebra defining `rank` without `unrank` stays on the
    /// sorted family.
    fn rank(&self, _level: Level, _key: &Self::Key) -> Option<u64> {
        None
    }

    /// Inverse of [`KeyAlgebra::rank`]: the key of ordinal `rank` in the
    /// level's enumeration, `None` when the domain is unbounded or `rank`
    /// is out of range.
    fn unrank(&self, _level: Level, _rank: u64) -> Option<Self::Key> {
        None
    }

    /// Shared enumeration of the level's domain in rank order (MEM-16):
    /// `enumeration(level)[rank] == unrank(level, rank)`. The dense arc
    /// family (ALIGN-2) stores one `Arc` clone of this table per node and
    /// re-emits the key of a slot by its rank, so algebras with a bounded
    /// domain should override this with a cached table — the default
    /// builds it from `unrank` on each call (one allocation per call).
    fn enumeration(&self, level: Level) -> Arc<[Self::Key]> {
        let cardinality = self.cardinality(level);
        let mut table = Vec::new();
        for rank in 0..cardinality {
            match self.unrank(level, rank) {
                Some(key) => table.push(key),
                None => break,
            }
        }
        table.into()
    }

    /// Declarative overload of the class field width `b` of `NodeId`
    /// (MEM-8, ADR-0103): when declared, it wins over the runtime
    /// derivation from `cardinality`, clamped to the compile-time window.
    /// `None` by default: the width is derived from the cardinalities.
    fn declared_class_bits(&self) -> Option<u32> {
        None
    }

    /// Number of levels of the universe sweep, used to derive the class
    /// width `b` at pool creation (MEM-8/9). Default `1`: algebras whose
    /// cardinality is level-independent (BDD, CDD, DBM, region) need no
    /// more; the reference instance overrides it with the schema's sweep.
    fn sweep_levels(&self) -> u32 {
        1
    }

    /// Declared key width of a level (MEM-16): `u8` when the cardinality
    /// fits in 8 bits, `u16` up to 16 bits, `u32` up to 32 bits; `None`
    /// on an open domain (cardinality beyond 32 bits), where no
    /// narrowing is lossless and the key/value run path is unchanged.
    fn key_width(&self, level: Level) -> Option<KeyWidth> {
        match self.cardinality(level) {
            c if c <= 1 << 8 => Some(KeyWidth::U8),
            c if c <= 1 << 16 => Some(KeyWidth::U16),
            c if c <= u32::MAX as u64 => Some(KeyWidth::U32),
            _ => None,
        }
    }

    /// Narrow a key to its 32-bit storage form (MEM-16): the ordinal rank
    /// of the key in the level's enumeration, when the level's domain is
    /// bounded (a declared key width exists) and the rank fits in 32 bits.
    /// `None` on an open domain: the node keeps the full-width key/value
    /// run (catch-all class).
    fn narrow(&self, level: Level, key: &Self::Key) -> Option<u32> {
        self.key_width(level)?;
        self.rank(level, key)
            .and_then(|rank| u32::try_from(rank).ok())
    }

    /// Inverse of [`KeyAlgebra::narrow`]: the key of a narrowed 32-bit
    /// form, `None` when the domain is open or the form is out of range.
    fn widen(&self, level: Level, narrowed: u32) -> Option<Self::Key> {
        self.unrank(level, narrowed as u64)
    }
}

/// Path-predicate port (D3-2, issue #165): membership of one diagram arc
/// in a path region, implemented controller-side. The reference instance
/// (`SchemaKeys`) uses **arc equality**; `DbmKeys` uses **region
/// inclusion** (`arc ⊆ region`). The liseuse evaluator drives its
/// incremental path automaton through this port, identically in replay
/// and at the gate.
pub trait PathPredicate {
    /// Arc key type (the key algebra's key).
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Does the arc `arc` at `level` belong to the path region `region`?
    fn arc_in_region(&self, level: Level, region: &Self::Key, arc: &Self::Key) -> bool;
}

/// Declared key width of a level (MEM-16), derived from the level's
/// cardinality: the narrowest lossless storage width.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum KeyWidth {
    /// Cardinality ≤ 2^8.
    U8,
    /// Cardinality ≤ 2^16.
    U16,
    /// Cardinality ≤ 2^32.
    U32,
}

/// Level-memory storage policy (M2-3, spec § 3.1): declared by the
/// controller, it decides which entries of `M : (node id, ℓ) ⇀ V` are kept
/// between two visits to a level.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MemoryPolicy {
    /// No level memory: memory-effect transductions are rejected
    /// (default behavior, M1).
    #[default]
    NoMemory,
    /// Memory kept per `(node, level)`: each visit to a level can read the
    /// state stored by a previous visit to the same node.
    PerLevel,
}

/// Extended controller (proposal § 1.2): instance of the eight fields that
/// decide reductions and encoding. The pool stays blind.
pub trait Controller: Send + Sync {
    type Key: Clone + Ord + Hash + Debug + Send + Sync + 'static;
    /// Payload of terminals (Bool for a set, Option(V) for lookup…).
    type Payload: Clone + Eq + Hash + Debug + Send + Sync + 'static;
    /// Key algebra of the instance.
    type Keys: KeyAlgebra<Key = Self::Key>;
    /// Arc storage of the instance (MEM-3, ADR-0103): default is the
    /// current `BTreeMap<Key, NodeId>`; another layout implements the
    /// same `ArcStore` port.
    type Arcs: ArcStore<Key = Self::Key>;

    /// Fingerprint placed in every handle and every cache key.
    const FINGERPRINT: ControllerId;

    /// Key algebra of this controller instance.
    fn keys(&self) -> &Self::Keys;

    /// Payload of the default diagram D_δ of a level: the implicit arc δ(ℓ)
    /// (R-default) denotes this terminal. `None`: no designated default.
    fn default_payload(&self, level: Level) -> Option<Self::Payload>;

    /// R-collapse (spec § 2.3): all arcs lead to the same target ⇒ the
    /// form reduces to that target. Pure, tested by PBT. Sees only the
    /// ordered iteration of the `ArcStore` port.
    fn collapse<Store>(arcs: &Store) -> Option<NodeId>
    where
        Store: ArcStore<Key = Self::Key> + ?Sized,
    {
        crate::reduction::collapse_arcs(arcs)
    }

    /// R-collapse active? `false` switches the reduction off (didactic
    /// workshop, backlog § 9: each reduction toggled one by one).
    fn collapse_active(&self) -> bool {
        true
    }

    /// R-default active? `false` switches off the absorption of the arc δ(ℓ)
    /// leading to D_δ (didactic workshop, same).
    fn default_active(&self) -> bool {
        true
    }

    /// Level-memory storage policy (M2-3, T3): declared by the controller.
    /// `NoMemory` by default (M1); `PerLevel` allows memory-effect transductions.
    fn memory_policy(&self) -> MemoryPolicy {
        MemoryPolicy::NoMemory
    }

    /// Generalized default (D-abs/D-trans, spec § 2.4.1, M4-3, claim A6):
    /// absorption of arcs toward D^default over **any subdomain** (not
    /// only δ(ℓ)) and transparent node (A = ∅ or all targets ≡ D →
    /// D). `false` by default: § 2.3 system (R-default on δ(ℓ) only).
    fn generalized_absorption(&self) -> bool {
        false
    }

    /// Key transformation group designated by the controller (M4-2,
    /// spec § 2.4.5): normalizes forms modulo G by canonical section
    /// of orbits, the argument living on the incoming arc. `None` by default:
    /// no relative forms (M1, absolute encoding).
    fn cycle_group(
        &self,
    ) -> Option<&dyn crate::relative::CycleGroup<Key = Self::Key, Argument = i64>> {
        None
    }
}

/// Pool: port "store / retrieve a node by key", implemented by
/// `lisage-core` (proposal § 8: `algebra` only sees this port).
pub trait Pool: Send + Sync {
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;

    /// Fingerprint of the pool's owning controller.
    const CONTROLLER: ControllerId;

    /// Hash-consed interning by key (controller, anchor, reduced form):
    /// R-default then R-collapse, R-share via the unique table.
    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId;

    /// Form of a node, or typed error if the identifier is unknown.
    /// Owned since MEM-16 (ADR-0104): exact-class entries store the arcs
    /// inline at stride `8·d + header` and no longer hold a `Form` in
    /// memory, so the form is rebuilt on resolution (keys widened back
    /// from their narrowed storage form).
    fn resolve(&self, node: NodeId) -> Result<Form<Self::Key, Self::Payload>, PoolError>;

    /// Canonical handle of a node for this reading (controller, anchor).
    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle;

    /// Number of interned nodes, terminals included.
    fn node_count(&self) -> usize;

    /// Class width `b` of the pool's `NodeId` layout (MEM-8/MEM-13): used
    /// by co-located memories to decode `(class, index)` (MEM-15).
    /// Default 0: single class, the raw id is the index.
    fn class_bits(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ALIGN-0 witness: one dense-family slot is `Option<(u32, NodeId)>` =
    /// 12 bytes (no niche). Pinned as the starting point; ALIGN-2 turns it
    /// into `Option<NodeId>` = 8 bytes by eliding the key (rank = position).
    #[test]
    fn test_align0_option_dense_slot_is_12_bytes() {
        assert_eq!(size_of::<(u32, NodeId)>(), 8);
        assert_eq!(
            size_of::<Option<(u32, NodeId)>>(),
            12,
            "ALIGN-0 premise: dense slot stores (narrowed key, target), 12 bytes"
        );
        assert_eq!(size_of::<Option<NodeId>>(), 8);
    }

    /// ALIGN-2 witness: the dense-family slot is now `Option<NodeId>` =
    /// 8 bytes (was 12 at ALIGN-0 for a u32 key, 24 for a u64 key) — the
    /// key is elided and re-emitted by rank from the shared enumeration.
    #[test]
    fn test_align2_dense_slot_is_8_bytes() {
        assert_eq!(
            size_of::<Option<NodeId>>(),
            8,
            "ALIGN-2: dense slot = Option<NodeId>, key elided (rank = position)"
        );
        // The per-slot saving for the production u64 keys of BDD/schema.
        assert_eq!(size_of::<(u64, NodeId)>(), 16);
        assert_eq!(
            size_of::<Option<(u64, NodeId)>>(),
            24,
            "ALIGN-0 layout for u64 keys: no niche, 24 bytes per slot"
        );
    }
}
