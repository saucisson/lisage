//! Level memory (M2-3, spec § 3.1, statement T3): state stored between two
//! visits of the same level, key `(id(node), ℓ)`. The version (L1) falsifies
//! the computed table entries when the memory changes.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use lisage_algebra::interface::{Level, NodeId};

/// Key of the level memory: (emitting node, level).
pub type KeyMemory = (NodeId, Level);

/// Versioned level memory: `M : (id(node), ℓ) ⇀ V` (spec § 3.1).
/// The version is incremented at each write; it enters the key of
/// the computed table (lemma L1: `τ(n, h)` pure in `(id(n), h, version(M))`).
#[derive(Clone, Debug)]
pub struct MemoryLevel<V: Clone + Eq + Hash + Debug> {
    inputs: HashMap<KeyMemory, V>,
    version: u64,
}

impl<V: Clone + Eq + Hash + Debug> Default for MemoryLevel<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> MemoryLevel<V> {
    /// Empty memory, version 0.
    pub fn new() -> Self {
        MemoryLevel {
            inputs: HashMap::new(),
            version: 0,
        }
    }

    /// Current version (L1): enters the memo key.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Read: value stored at `(node, level)`, if present.
    pub fn read(&self, node: NodeId, level: Level) -> Option<&V> {
        self.inputs.get(&(node, level))
    }

    /// Write: stores `value` at `(node, level)` and increments the
    /// version (L1). Returns the old value if it existed.
    pub fn write(&mut self, node: NodeId, level: Level, value: V) -> Option<V> {
        self.version += 1;
        self.inputs.insert((node, level), value)
    }

    /// Number of stored entries (for tests).
    pub fn size(&self) -> usize {
        self.inputs.len()
    }
}

/// Memory effect produced by `φ_out` (spec § 3.1: `M ← M ⊕ μ`): list
/// of writes to apply after the node restructuring.
#[derive(Clone, Debug)]
pub struct EffectMemory<V: Clone + Eq + Hash + Debug> {
    /// Writes `(node, level, value)` to apply in the memory.
    pub ecritures: Vec<(NodeId, Level, V)>,
}

impl<V: Clone + Eq + Hash + Debug> Default for EffectMemory<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> EffectMemory<V> {
    /// Empty effect: no write.
    pub fn new() -> Self {
        EffectMemory {
            ecritures: Vec::new(),
        }
    }

    /// Effect with a single write.
    pub fn one(node: NodeId, level: Level, value: V) -> Self {
        EffectMemory {
            ecritures: vec![(node, level, value)],
        }
    }

    /// Applies the effect to the memory (increments the version per write).
    pub fn apply(&self, memory: &mut MemoryLevel<V>) {
        for (node, level, value) in &self.ecritures {
            memory.write(*node, *level, value.clone());
        }
    }
}
