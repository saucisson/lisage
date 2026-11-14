//! **Independent** BDD witness (M1-8, statement C6, pure-Rust degraded mode):
//! mini-BDD written on purpose for this witness, sharing no code with
//! `lisage-core` — its own unique table, recursive Shannon construction.
//! The C6 differential compares the **node count** obtained in the arena
//! (classic BDD configuration of `lisage-core::bdd`) and at this witness:
//! identical reproduction required. The external CUDD/Sylvan witness (FFI)
//! remains escalated — see `docs/spike-ffi-cudd-sylvan.md` and the
//! `needs-human` comment of issue #14.

use std::collections::HashMap;

/// Index of the ⊥ terminal (false).
pub const TERMINAL_FAUX: u32 = 0;
/// Index of the ⊤ terminal (true).
pub const TERMINAL_VRAI: u32 = 1;

/// Mini-BDD: nodes `(variable, low, high)` in a table, terminals at
/// indices 0 and 1.
pub struct MiniBdd {
    noeuds: Vec<(u32, u32, u32)>,
    table_unique: HashMap<(u32, u32, u32), u32>,
}

impl MiniBdd {
    /// Fresh witness, terminals pre-installed.
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

    /// Builds the BDD of a truth table (`table[a]`, bit ℓ of `a` =
    /// variable ℓ) by recursive Shannon expansion.
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
        // Bit ℓ of the index = variable ℓ: even positions = arc 0.
        let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
        let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
        let bas = self.construire(variable + 1, nombre_variables, &bas_table);
        let haut = self.construire(variable + 1, nombre_variables, &haut_table);
        self.fabriquer_noeud(variable, bas, haut)
    }

    /// Number of nodes reachable from `racine`, terminals included
    /// (same convention as `lisage_core::bdd::compter_noeuds`).
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

    /// Evaluates the BDD of index `racine` on an assignment.
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
