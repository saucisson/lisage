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
    entrees: HashMap<CleMemoire, V>,
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
            entrees: HashMap::new(),
            version: 0,
        }
    }

    /// Current version (L1): enters the memo key.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Read: value stored at `(noeud, niveau)`, if present.
    pub fn lire(&self, noeud: NodeId, niveau: Level) -> Option<&V> {
        self.entrees.get(&(noeud, niveau))
    }

    /// Write: stores `valeur` at `(noeud, niveau)` and increments the version
    /// (L1). Returns the old value if it existed.
    pub fn ecrire(&mut self, noeud: NodeId, niveau: Level, valeur: V) -> Option<V> {
        self.version += 1;
        self.entrees.insert((noeud, niveau), valeur)
    }

    /// Number of stored entries (for tests).
    pub fn taille(&self) -> usize {
        self.entrees.len()
    }
}

/// Memory effect produced by `φ_out` (spec § 3.1: `M ← M ⊕ μ`): list of writes
/// to apply after the node restructuring.
#[derive(Clone, Debug)]
pub struct EffetMemoire<V: Clone + Eq + Hash + Debug> {
    /// Writes `(noeud, niveau, valeur)` to apply in the memory.
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
    pub fn une(noeud: NodeId, niveau: Level, valeur: V) -> Self {
        EffetMemoire {
            ecritures: vec![(noeud, niveau, valeur)],
        }
    }

    /// Applies the effect to the memory (increments the version per write).
    pub fn appliquer(&self, memoire: &mut MemoireNiveau<V>) {
        for (noeud, niveau, valeur) in &self.ecritures {
            memoire.ecrire(*noeud, *niveau, valeur.clone());
        }
    }
}
