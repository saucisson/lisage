//! **Independent** BDD witness (M1-8, statement C6, pure-Rust degraded mode):
//! mini-BDD written specifically for this witness, sharing no code with
//! `lisage-core` — its own unique table, recursive Shannon construction.
//! Differential C6 compares the **node count** obtained in the arena
//! (classic BDD configuration of `lisage-core::bdd`) and at this witness:
//! exact reproduction required. The external CUDD/Sylvan witness (FFI)
//! remains escalated — see `docs/spike-ffi-cudd-sylvan.md` and the
//! `needs-human` comment on issue #14.

use std::collections::HashMap;

/// Index of the ⊥ (false) terminal.
pub const TERMINAL_FAUX: u32 = 0;
/// Index of the ⊤ (true) terminal.
pub const TERMINAL_VRAI: u32 = 1;

/// Mini-BDD: nodes `(variable, low, top)` in a table, terminals at the
/// head of indices 0 and 1.
pub struct MiniBdd {
    nodes: Vec<(u32, u32, u32)>,
    table_unique: HashMap<(u32, u32, u32), u32>,
}

impl MiniBdd {
    /// New witness, terminals pre-installed.
    pub fn new() -> Self {
        MiniBdd {
            nodes: vec![(u32::MAX, 0, 0), (u32::MAX, 1, 1)],
            table_unique: HashMap::new(),
        }
    }

    /// `mk(var, low, top)`: sharing + collapse (low == top ⇒ low).
    fn make_node(&mut self, variable: u32, low: u32, top: u32) -> u32 {
        if low == top {
            return low;
        }
        let triplet = (variable, low, top);
        if let Some(&indice) = self.table_unique.get(&triplet) {
            return indice;
        }
        let indice = self.nodes.len() as u32;
        self.nodes.push(triplet);
        self.table_unique.insert(triplet, indice);
        indice
    }

    /// Builds the BDD of a truth table (`table[a]`, bit ℓ of `a` =
    /// variable ℓ) by recursive Shannon expansion.
    pub fn compile_table(&mut self, number_variables: u32, table: &[bool]) -> u32 {
        assert_eq!(table.len(), 1usize << number_variables);
        self.build(0, number_variables, table)
    }

    fn build(&mut self, variable: u32, number_variables: u32, table: &[bool]) -> u32 {
        if variable == number_variables {
            return if table[0] {
                TERMINAL_VRAI
            } else {
                TERMINAL_FAUX
            };
        }
        // Bit ℓ of the index = variable ℓ: even positions = edge 0.
        let low_table: Vec<bool> = table.iter().step_by(2).copied().collect();
        let top_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
        let low = self.build(variable + 1, number_variables, &low_table);
        let top = self.build(variable + 1, number_variables, &top_table);
        self.make_node(variable, low, top)
    }

    /// Number of nodes reachable from `root`, terminals included
    /// (same convention as `lisage_core::bdd::count_nodes`).
    pub fn count_nodes(&self, root: u32) -> u64 {
        let mut visit = std::collections::BTreeSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !visit.insert(node) {
                continue;
            }
            let (variable, low, top) = self.nodes[node as usize];
            if variable != u32::MAX {
                stack.push(low);
                stack.push(top);
            }
        }
        visit.len() as u64
    }

    /// Evaluates the BDD at index `root` on an assignment.
    pub fn evaluer(&self, root: u32, affectation: u64) -> bool {
        let mut node = root;
        loop {
            let (variable, low, top) = self.nodes[node as usize];
            if variable == u32::MAX {
                return top == TERMINAL_VRAI;
            }
            let bit = (affectation >> variable) & 1;
            node = if bit == 0 { low } else { top };
        }
    }
}

impl Default for MiniBdd {
    fn default() -> Self {
        Self::new()
    }
}
