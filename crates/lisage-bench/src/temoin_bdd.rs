//! Témoin BDD **indépendant** (M1-8, énoncé C6, mode dégradé pur Rust) :
//! mini-BDD écrit exprès pour ce témoin, sans partager de code avec
//! `lisage-core` — table unique propre, construction de Shannon récursive.
//! Le différentiel C6 compare le **nombre de nœuds** obtenu dans l'arène
//! (configuration BDD classique de `lisage-core::bdd`) et chez ce témoin :
//! reproduction à l'identique exigée. Le témoin externe CUDD/Sylvan (FFI)
//! reste escaladé — voir `docs/spike-ffi-cudd-sylvan.md` et le commentaire
//! `needs-human` de l'issue #14.

use std::collections::HashMap;

/// Indice du terminal ⊥ (faux).
pub const TERMINAL_FAUX: u32 = 0;
/// Indice du terminal ⊤ (vrai).
pub const TERMINAL_VRAI: u32 = 1;

/// Mini-BDD : nœuds `(variable, bas, haut)` dans une table, terminaux en
/// tête d'indices 0 et 1.
pub struct MiniBdd {
    noeuds: Vec<(u32, u32, u32)>,
    table_unique: HashMap<(u32, u32, u32), u32>,
}

impl MiniBdd {
    /// Témoin neuf, terminaux pré-installés.
    pub fn new() -> Self {
        MiniBdd {
            noeuds: vec![(u32::MAX, 0, 0), (u32::MAX, 1, 1)],
            table_unique: HashMap::new(),
        }
    }

    /// `mk(var, bas, haut)` : partage + collapse (bas == haut ⇒ bas).
    fn fabriquer_noeud(&mut self, variable: u32, bas: u32, haut: u32) -> u32 {
        if bas == haut {
            return bas;
        }
        let triplet = (variable, bas, haut);
        if let Some(&indice) = self.table_unique.get(&triplet) {
            return indice;
        }
        let indice = self.noeuds.len() as u32;
        self.noeuds.push(triplet);
        self.table_unique.insert(triplet, indice);
        indice
    }

    /// Construit le BDD d'une table de vérité (`table[a]`, bit ℓ de `a` =
    /// variable ℓ) par expansion de Shannon récursive.
    pub fn compiler_table(&mut self, nombre_variables: u32, table: &[bool]) -> u32 {
        assert_eq!(table.len(), 1usize << nombre_variables);
        self.construire(0, nombre_variables, table)
    }

    fn construire(&mut self, variable: u32, nombre_variables: u32, table: &[bool]) -> u32 {
        if variable == nombre_variables {
            return if table[0] {
                TERMINAL_VRAI
            } else {
                TERMINAL_FAUX
            };
        }
        // Bit ℓ de l'indice = variable ℓ : positions paires = arc 0.
        let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
        let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
        let bas = self.construire(variable + 1, nombre_variables, &bas_table);
        let haut = self.construire(variable + 1, nombre_variables, &haut_table);
        self.fabriquer_noeud(variable, bas, haut)
    }

    /// Nombre de nœuds atteignables depuis `racine`, terminaux compris
    /// (même convention que `lisage_core::bdd::compter_noeuds`).
    pub fn compter_noeuds(&self, racine: u32) -> u64 {
        let mut visite = std::collections::BTreeSet::new();
        let mut pile = vec![racine];
        while let Some(noeud) = pile.pop() {
            if !visite.insert(noeud) {
                continue;
            }
            let (variable, bas, haut) = self.noeuds[noeud as usize];
            if variable != u32::MAX {
                pile.push(bas);
                pile.push(haut);
            }
        }
        visite.len() as u64
    }

    /// Évalue le BDD d'indice `racine` sur une affectation.
    pub fn evaluer(&self, racine: u32, affectation: u64) -> bool {
        let mut noeud = racine;
        loop {
            let (variable, bas, haut) = self.noeuds[noeud as usize];
            if variable == u32::MAX {
                return haut == TERMINAL_VRAI;
            }
            let bit = (affectation >> variable) & 1;
            noeud = if bit == 0 { bas } else { haut };
        }
    }
}

impl Default for MiniBdd {
    fn default() -> Self {
        Self::new()
    }
}
