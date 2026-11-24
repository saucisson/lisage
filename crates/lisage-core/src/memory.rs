//! Level memory (M2-3, spec § 3.1, statement T3): state stored between two
//! visits of the same level, key `(id(node), ℓ)`. The version (L1) falsifies
//! the computed-table entries when the memory changes.
//!
//! Storage (MEM-15, ADR-0103): the `HashMap<(NodeId, Level), V>` (~48 B per
//! entry) is replaced by **dense parallel arrays per class** — one presence
//! bitmap (1 bit/slot), one `u32` level array and one value array, indexed
//! by the node's index inside its class arena (~4 B + 1 bit + `size_of::<V>()`
//! per entry). The bounded eviction policy is unchanged: the arrays are
//! bounded by the pool's arena high-water mark, no entry is ever evicted.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

use lisage_algebra::interface::{Level, NodeId};

/// Level-memory key: (emitting node, level).
pub type KeyMemory = (NodeId, Level);

/// Dense parallel arrays of one class, indexed by the node's index in its
/// class arena: presence bitmap + level array + value array. The common
/// case is one memory entry per node (the node's own level); a node with
/// entries at several levels keeps the first in the dense slot and the
/// others in the small `overflow` list (linear scan).
#[derive(Clone, Debug)]
struct ClassMemory<V> {
    /// Presence bitmap: 1 bit per slot.
    present: Vec<u64>,
    /// Level of the stored entry, per slot (parallel array).
    levels: Vec<u32>,
    /// Stored value, per slot (parallel array; `None` on absent slots).
    values: Vec<Option<V>>,
    /// Extra `(index, level, value)` entries when a node has several levels.
    overflow: Vec<(u32, u32, V)>,
    /// Number of present dense slots.
    live: usize,
}

impl<V> ClassMemory<V> {
    fn new() -> Self {
        ClassMemory {
            present: Vec::new(),
            levels: Vec::new(),
            values: Vec::new(),
            overflow: Vec::new(),
            live: 0,
        }
    }

    /// Grow the parallel arrays to hold `index`.
    fn grow_to(&mut self, index: usize) {
        if index < self.levels.len() {
            return;
        }
        let words = (index + 64) / 64;
        self.present.resize(words, 0);
        self.levels.resize(index + 1, 0);
        self.values.resize_with(index + 1, || None);
    }

    fn is_present(&self, index: usize) -> bool {
        self.present
            .get(index / 64)
            .is_some_and(|word| (word >> (index % 64)) & 1 == 1)
    }

    fn set_present(&mut self, index: usize) {
        self.present[index / 64] |= 1 << (index % 64);
    }

    /// Read the entry `(index, level)`: dense slot first, then overflow.
    fn read(&self, index: u32, level: u32) -> Option<&V> {
        let index = index as usize;
        if self.is_present(index) && self.levels[index] == level {
            return self.values[index].as_ref();
        }
        self.overflow
            .iter()
            .find(|(i, l, _)| *i == index as u32 && *l == level)
            .map(|(_, _, value)| value)
    }

    /// Write `(index, level, value)`; returns the old value of that key.
    fn write(&mut self, index: u32, level: u32, value: V) -> Option<V> {
        let slot = index as usize;
        if !self.is_present(slot) {
            self.grow_to(slot);
            self.set_present(slot);
            self.live += 1;
            self.levels[slot] = level;
            return self.values[slot].replace(value);
        }
        if self.levels[slot] == level {
            return self.values[slot].replace(value);
        }
        // Second level for this node: overflow list.
        if let Some(entry) = self
            .overflow
            .iter_mut()
            .find(|(i, l, _)| *i == index && *l == level)
        {
            return Some(std::mem::replace(&mut entry.2, value));
        }
        self.overflow.push((index, level, value));
        None
    }

    /// Memory footprint of this class in bytes (MEM-15 counter).
    fn bytes(&self) -> usize {
        self.present.capacity() * 8
            + self.levels.capacity() * 4
            + self.values.capacity() * size_of::<Option<V>>()
            + self.overflow.capacity() * (4 + 4 + size_of::<V>())
    }
}

/// Versioned level memory: `M : (id(node), ℓ) ⇀ V` (spec § 3.1). The version
/// is incremented at each write; it enters the computed-table key (lemma L1:
/// `τ(n, h)` pure in `(id(n), h, version(M))`).
///
/// Storage is dense per class (MEM-15): `classes[class]` holds the parallel
/// arrays indexed by the node's index in its class arena, decoded with the
/// pool's class width `b` ([`MemoryLevel::set_class_bits`]).
#[derive(Clone, Debug)]
pub struct MemoryLevel<V: Clone + Eq + Hash + Debug> {
    /// Class width `b` of the pool's `NodeId` layout (0: single class, the
    /// raw id is the index).
    class_bits: u32,
    /// Per-class dense arrays, created lazily.
    classes: Vec<Option<ClassMemory<V>>>,
    /// Per-fragment versions (M2-5, the #388 core remainder): one
    /// version counter per level fragment written so far, bumped
    /// alongside the global version by every write of the fragment.
    /// The memo of the runner keys on the versions of the fragments
    /// the transduction DECLARES to read (`read_fragments`), so a write
    /// to one fragment never invalidates the memo entries of the
    /// others.
    fragment_versions: BTreeMap<u32, u64>,
    version: u64,
    len: usize,
}

impl<V: Clone + Eq + Hash + Debug> Default for MemoryLevel<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> MemoryLevel<V> {
    /// Empty memory, version 0, single class (`class_bits = 0`).
    pub fn new() -> Self {
        Self::with_class_bits(0)
    }

    /// Empty memory decoding `NodeId`s with the given class width `b`.
    pub fn with_class_bits(class_bits: u32) -> Self {
        MemoryLevel {
            class_bits,
            classes: Vec::new(),
            fragment_versions: BTreeMap::new(),
            version: 0,
            len: 0,
        }
    }

    /// Set the class width `b` of the owning pool (MEM-15): to be called
    /// before any entry, e.g. by the runner at the start of a transduction.
    pub fn set_class_bits(&mut self, class_bits: u32) {
        self.class_bits = class_bits;
    }

    /// Current version (L1): enters the memo key.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Current version of one fragment (M2-5, the #388 core remainder):
    /// the number of writes the fragment received, 0 when it was never
    /// written. Enters the memo key alongside the nodes and the `h`
    /// states, scoped to the fragments the transduction declares to
    /// read: the entries of the other fragments survive its writes.
    pub fn version_of(&self, fragment: u32) -> u64 {
        self.fragment_versions.get(&fragment).copied().unwrap_or(0)
    }

    /// Read: value stored at `(node, level)`, if present.
    pub fn read(&self, node: NodeId, level: Level) -> Option<&V> {
        let (class, index) = node.decode(self.class_bits);
        let memory = self.classes.get(class as usize)?.as_ref()?;
        memory.read(index, level.0)
    }

    /// Write: stores `value` at `(node, level)` and increments the version
    /// (L1). Returns the old value if it existed.
    pub fn write(&mut self, node: NodeId, level: Level, value: V) -> Option<V> {
        let (class, index) = node.decode(self.class_bits);
        if self.classes.len() <= class as usize {
            self.classes.resize_with(class as usize + 1, || None);
        }
        let memory = self.classes[class as usize].get_or_insert_with(ClassMemory::new);
        let old = memory.write(index, level.0, value);
        if old.is_none() {
            self.len += 1;
        }
        // M2-5: the fragment of the written level carries its own
        // version, independent of the untouched fragments.
        *self.fragment_versions.entry(level.0).or_insert(0) += 1;
        self.version += 1;
        old
    }

    /// Number of stored entries (for tests).
    pub fn size(&self) -> usize {
        self.len
    }

    /// Memory footprint of the memory in bytes (MEM-15 counter): presence
    /// bitmaps + level arrays + value arrays, capacities included.
    pub fn bytes(&self) -> usize {
        self.classes
            .iter()
            .flatten()
            .map(|memory| memory.bytes())
            .sum()
    }

    /// Apply a compaction remap (MEM-15): the memory holders translate
    /// their stored node ids through the pool's `last_remap` table; dead
    /// nodes (`old → None`) drop their entry. Bumps the version once when
    /// anything moved (L1: the memory changed).
    pub fn remap(&mut self, remap: &BTreeMap<u32, Option<NodeId>>) {
        let mut entries: Vec<(NodeId, Level, V)> = Vec::with_capacity(self.len);
        for (class, memory) in self.classes.iter().enumerate() {
            let Some(memory) = memory else { continue };
            for index in 0..memory.levels.len() {
                if memory.is_present(index) {
                    let node = NodeId::encode(class as u32, index as u32, self.class_bits);
                    entries.push((
                        node,
                        Level(memory.levels[index]),
                        memory.values[index]
                            .clone()
                            .expect("present slot has a value"),
                    ));
                }
            }
            for (index, level, value) in &memory.overflow {
                let node = NodeId::encode(class as u32, *index, self.class_bits);
                entries.push((node, Level(*level), value.clone()));
            }
        }
        self.classes.clear();
        self.len = 0;
        let mut moved = false;
        for (node, level, value) in entries {
            let Some(Some(new)) = remap.get(&node.0) else {
                moved = true;
                continue;
            };
            if *new != node {
                moved = true;
            }
            self.write(*new, level, value);
        }
        if moved {
            self.version += 1;
        }
    }
}

/// Memory effect produced by `φ_out` (spec § 3.1: `M ← M ⊕ μ`): list of writes
/// to apply after the node restructuring.
#[derive(Clone, Debug)]
pub struct EffectMemory<V: Clone + Eq + Hash + Debug> {
    /// Writes `(node, level, value)` to apply in the memory.
    pub writes: Vec<(NodeId, Level, V)>,
}

impl<V: Clone + Eq + Hash + Debug> Default for EffectMemory<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> EffectMemory<V> {
    /// Empty effect: no write.
    pub fn new() -> Self {
        EffectMemory { writes: Vec::new() }
    }

    /// Effect with a single write.
    pub fn one(node: NodeId, level: Level, value: V) -> Self {
        EffectMemory {
            writes: vec![(node, level, value)],
        }
    }

    /// Applies the effect to the memory (increments the version per write).
    pub fn apply(&self, memory: &mut MemoryLevel<V>) {
        for (node, level, value) in &self.writes {
            memory.write(*node, *level, value.clone());
        }
    }
}
