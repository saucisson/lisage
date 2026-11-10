//! Mémoire de niveau (M2-3, spec § 3.1, énoncé T3) : état stocké entre deux
//! visites d'un même niveau, clé `(id(nœud), ℓ)`. La version (L1) falsifie
//! les entrées de la computed table quand la mémoire change.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use lisage_algebra::interface::{Level, NodeId};

/// Clé de la mémoire de niveau : (nœud émetteur, niveau).
pub type CleMemoire = (NodeId, Level);

/// Mémoire de niveau versionnée : `M : (id(nœud), ℓ) ⇀ V` (spec § 3.1).
/// La version est incrémentée à chaque écrit ; elle entre dans la clé de
/// la computed table (lemme L1 : `τ(n, h)` pur en `(id(n), h, version(M))`).
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
    /// Mémoire vide, version 0.
    pub fn new() -> Self {
        MemoireNiveau {
            entrees: HashMap::new(),
            version: 0,
        }
    }

    /// Version courante (L1) : entre dans la clé de mémo.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Lecture : valeur stockée à `(nœud, niveau)`, si présente.
    pub fn lire(&self, noeud: NodeId, niveau: Level) -> Option<&V> {
        self.entrees.get(&(noeud, niveau))
    }

    /// Écriture : stocke `valeur` à `(nœud, niveau)` et incrémente la
    /// version (L1). Retourne l'ancienne valeur si elle existait.
    pub fn ecrire(&mut self, noeud: NodeId, niveau: Level, valeur: V) -> Option<V> {
        self.version += 1;
        self.entrees.insert((noeud, niveau), valeur)
    }

    /// Nombre d'entrées stockées (pour les tests).
    pub fn taille(&self) -> usize {
        self.entrees.len()
    }
}

/// Effet mémoire produit par `φ_out` (spec § 3.1 : `M ← M ⊕ μ`) : liste
/// d'écritures à appliquer après la restructuration du nœud.
#[derive(Clone, Debug)]
pub struct EffetMemoire<V: Clone + Eq + Hash + Debug> {
    /// Écritures `(nœud, niveau, valeur)` à appliquer dans la mémoire.
    pub ecritures: Vec<(NodeId, Level, V)>,
}

impl<V: Clone + Eq + Hash + Debug> Default for EffetMemoire<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + Hash + Debug> EffetMemoire<V> {
    /// Effet vide : aucune écriture.
    pub fn new() -> Self {
        EffetMemoire {
            ecritures: Vec::new(),
        }
    }

    /// Effet avec une seule écriture.
    pub fn une(noeud: NodeId, niveau: Level, valeur: V) -> Self {
        EffetMemoire {
            ecritures: vec![(noeud, niveau, valeur)],
        }
    }

    /// Applique l'effet à la mémoire (incrémente la version par écriture).
    pub fn appliquer(&self, memoire: &mut MemoireNiveau<V>) {
        for (noeud, niveau, valeur) in &self.ecritures {
            memoire.ecrire(*noeud, *niveau, valeur.clone());
        }
    }
}
