//! Level memory (M2-3, spec § 3.1, statement T3): state stored between two
//! visits of the same level, key `(id(node), ℓ)`. The version (L1) falsifies
//! the computed-table entries when the memory changes.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use lisage_algebra::interface::{Level, NodeId};

/// Level-memory key: (emitting node, level).
pub type CleMemoire = (NodeId, Level);

/// Versioned level memory: `M : (id(node), ℓ) ⇀ V` (spec § 3.1). The version
/// is incremented at each write; it enters the computed-table key (lemma L1:
/// `τ(n, h)` pure in `(id(n), h, version(M))`).
#[derive(Clone, Debug)]
pub struct MemoireNiveau<V: Clone + Eq + Hash + Debug> {
    inputs: HashMap<CleMemoire, V>,
    version: u64,
}

impl<V: Clone + Eq + Hash + Debug> Default for MemoireNiveau<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> MemoireNiveau<V> {
    /// Empty memory, version 0.
    pub fn new() -> Self {
        MemoireNiveau {
            inputs: HashMap::new(),
            version: 0,
        }
    }

    /// Current version (L1): enters the memo key.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Read: value stored at `(node, niveau)`, if present.
    pub fn lire(&self, node: NodeId, niveau: Level) -> Option<&V> {
        self.inputs.get(&(node, niveau))
    }

    /// Write: stores `valeur` at `(node, niveau)` and increments the version
    /// (L1). Returns the old value if it existed.
    pub fn ecrire(&mut self, node: NodeId, niveau: Level, valeur: V) -> Option<V> {
        self.version += 1;
        self.inputs.insert((node, niveau), valeur)
    }

    /// Number of stored entries (for tests).
    pub fn taille(&self) -> usize {
        self.inputs.len()
    }
}

/// Memory effect produced by `φ_out` (spec § 3.1: `M ← M ⊕ μ`): list of writes
/// to apply after the node restructuring.
#[derive(Clone, Debug)]
pub struct EffetMemoire<V: Clone + Eq + Hash + Debug> {
    /// Writes `(node, niveau, valeur)` to apply in the memory.
    pub ecritures: Vec<(NodeId, Level, V)>,
}

impl<V: Clone + Eq + Hash + Debug> Default for EffetMemoire<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> EffetMemoire<V> {
    /// Empty effect: no write.
    pub fn new() -> Self {
        EffetMemoire {
            ecritures: Vec::new(),
        }
    }

    /// Effect with a single write.
    pub fn une(node: NodeId, niveau: Level, valeur: V) -> Self {
        EffetMemoire {
            ecritures: vec![(node, niveau, valeur)],
        }
    }

    /// Applies the effect to the memory (increments the version per write).
    pub fn apply(&self, memoire: &mut MemoireNiveau<V>) {
        for (node, niveau, valeur) in &self.ecritures {
            memoire.ecrire(*node, *niveau, valeur.clone());
        }
    }
}
