//! Core interface (M1-0): contracts seen by `algebra` from the `core` pool,
//! blind canonical storage (forms and handles). `Send`/`Sync` bounds
//! frozen from M1-0 (roadmap § 10).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

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
    /// `None` = hole = default not stored.
    Dense(Box<[Option<(Key, NodeId)>]>),
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
    pub fn from_pairs(pairs: impl IntoIterator<Item = (Key, NodeId)>) -> Self {
        let mut map: BTreeMap<Key, NodeId> = BTreeMap::new();
        for (key, target) in pairs {
            map.insert(key, target);
        }
        ArcRun::Sorted(map.into_iter().collect())
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
        if 4 * cardinality > 12 * arcs_len {
            return arcs;
        }
        let mut slots: Vec<Option<(Key, NodeId)>> = vec![None; cardinality as usize];
        for (key, target) in pairs.iter() {
            match keys.rank(level, key) {
                Some(rang) if (rang as usize) < slots.len() => {
                    slots[rang as usize] = Some((key.clone(), *target));
                }
                _ => return arcs,
            }
        }
        ArcRun::Dense(slots.into_boxed_slice())
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
            ArcRun::Dense(slots) => slots.iter().find_map(|slot| {
                slot.as_ref()
                    .and_then(|(k, target)| (k == key).then_some(target))
            }),
        }
    }

    /// Is `key` present?
    pub fn contains_key(&self, key: &Key) -> bool {
        self.get(key).is_some()
    }

    /// Ordered iteration of the `(key, target)` pairs (key order for the
    /// sorted run, rank order for the dense family).
    pub fn iter(&self) -> Box<dyn DoubleEndedIterator<Item = (&Key, &NodeId)> + '_> {
        match self {
            ArcRun::Sorted(pairs) => Box::new(pairs.iter().map(|(key, target)| (key, target))),
            ArcRun::Dense(slots) => Box::new(slots.iter().filter(|slot| slot.is_some()).map(
                |slot| match slot {
                    Some((key, target)) => (key, target),
                    None => unreachable!("filtered above"),
                },
            )),
        }
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
            ArcRun::Dense(slots) => slots.iter().filter(|slot| slot.is_some()).count(),
        }
    }

    /// True when the run holds no arc.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
            ArcRun::Dense(slots) => slots
                .into_vec()
                .into_iter()
                .flatten()
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
    /// `Some(rang)` when the level's domain is bounded and the key belongs
    /// to it; `None` otherwise. The default `None` declares the absence of
    /// rank properly (unbounded domain: the DDD path is unchanged), so the
    /// dense ordinal storage (MEM-B) writes no code per controller.
    /// Rank availability is exposed to the normalization: `normal_form`
    /// may encode keys by their rank when it exists.
    fn rank(&self, _level: Level, _key: &Self::Key) -> Option<u64> {
        None
    }

    /// Inverse of [`KeyAlgebra::rank`]: the key of ordinal `rang` in the
    /// level's enumeration, `None` when the domain is unbounded or `rang`
    /// is out of range.
    fn unrank(&self, _level: Level, _rang: u64) -> Option<Self::Key> {
        None
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
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    /// Payload of terminals (Bool for a set, Option(V) for lookup…).
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;
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
    ) -> Option<&dyn crate::relatif::CycleGroup<Key = Self::Key, Argument = i64>> {
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
    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError>;

    /// Canonical handle of a node for this reading (controller, anchor).
    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle;

    /// Number of interned nodes, terminals included.
    fn node_count(&self) -> usize;
}
