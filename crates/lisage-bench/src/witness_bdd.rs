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
pub const TERMINAL_FALSE: u32 = 0;
/// Index of the ⊤ terminal (true).
pub const TERMINAL_TRUE: u32 = 1;

/// Published binary node of the witness (MEM-6): the `Val(level) = 2`
/// case of the port — a node at level `LEVEL` has exactly two arcs,
/// `LOW` (variable = 0) and `HIGH` (variable = 1). The three fields of
/// the classic BDD node are const generics; the only runtime state is
/// the CUDD-style complement bit.
pub struct BinaryNode<const LEVEL: u32, const LOW: u32, const HIGH: u32> {
    complement: bool,
}

impl<const LEVEL: u32, const LOW: u32, const HIGH: u32> BinaryNode<LEVEL, LOW, HIGH> {
    /// Static size of the published binary node, in bytes.
    pub const SIZE: usize = size_of::<Self>();

    /// Builds the published node carrying the given complement bit.
    pub const fn new(complement: bool) -> Self {
        Self { complement }
    }

    /// Level (variable index) of the node.
    pub const fn level() -> u32 {
        LEVEL
    }

    /// Low arc (variable = 0).
    pub const fn low() -> u32 {
        LOW
    }

    /// High arc (variable = 1).
    pub const fn high() -> u32 {
        HIGH
    }

    /// CUDD-style complement bit of the node.
    pub const fn is_complemented(&self) -> bool {
        self.complement
    }

    /// Complement edge, CUDD style: flips the bit, shares the node.
    pub const fn negate(&self) -> Self {
        Self {
            complement: !self.complement,
        }
    }
}

/// The published binary node is exactly one byte: level/low/high live at
/// the type level (const generics), only the complement bit is runtime
/// state.
const _: () = assert!(BinaryNode::<0, TERMINAL_FALSE, TERMINAL_TRUE>::SIZE == 1);

/// Mini-BDD: nodes `(variable, low, high)` in a table, terminals at
/// indices 0 and 1.
pub struct MiniBdd {
    nodes: Vec<(u32, u32, u32)>,
    table_unique: HashMap<(u32, u32, u32), u32>,
}

impl MiniBdd {
    /// Fresh witness, terminals pre-installed.
    pub fn new() -> Self {
        MiniBdd {
            nodes: vec![(u32::MAX, 0, 0), (u32::MAX, 1, 1)],
            table_unique: HashMap::new(),
        }
    }

    /// `mk(var, bas, haut)` : shared + collapse (bas == haut ⇒ bas).
    fn fabriquer_node(&mut self, variable: u32, bas: u32, haut: u32) -> u32 {
        if bas == haut {
            return bas;
        }
        let triplet = (variable, bas, haut);
        if let Some(&index) = self.table_unique.get(&triplet) {
            return index;
        }
        let index = self.nodes.len() as u32;
        self.nodes.push(triplet);
        self.table_unique.insert(triplet, index);
        index
    }

    /// Materializes the node at `index` as the published binary node
    /// when its runtime fields match the const generics, `None`
    /// otherwise. The complement bit is a view (CUDD-style edge), not
    /// stored: it defaults to `false` and is flipped by
    /// [`BinaryNode::negate`].
    pub fn binary_node<const LEVEL: u32, const LOW: u32, const HIGH: u32>(
        &self,
        index: u32,
    ) -> Option<BinaryNode<LEVEL, LOW, HIGH>> {
        let (level, low, high) = self.nodes[index as usize];
        if level == LEVEL && low == LOW && high == HIGH {
            Some(BinaryNode::new(false))
        } else {
            None
        }
    }

    /// Builds the BDD of a truth table (`table[a]`, bit ℓ of `a` =
    /// variable ℓ) by recursive Shannon expansion.
    pub fn compiler_table(&mut self, number_variables: u32, table: &[bool]) -> u32 {
        assert_eq!(table.len(), 1usize << number_variables);
        self.build(0, number_variables, table)
    }

    fn build(&mut self, variable: u32, number_variables: u32, table: &[bool]) -> u32 {
        if variable == number_variables {
            return if table[0] {
                TERMINAL_TRUE
            } else {
                TERMINAL_FALSE
            };
        }
        // Bit ℓ of the index = variable ℓ: even positions = arc 0.
        let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
        let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
        let bas = self.build(variable + 1, number_variables, &bas_table);
        let haut = self.build(variable + 1, number_variables, &haut_table);
        self.fabriquer_node(variable, bas, haut)
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
            let (variable, bas, haut) = self.nodes[node as usize];
            if variable != u32::MAX {
                stack.push(bas);
                stack.push(haut);
            }
        }
        visit.len() as u64
    }

    /// Evaluates the BDD of index `root` on an assignment.
    pub fn evaluate(&self, root: u32, assignment: u64) -> bool {
        let mut node = root;
        loop {
            let (variable, bas, haut) = self.nodes[node as usize];
            if variable == u32::MAX {
                return haut == TERMINAL_TRUE;
            }
            let bit = (assignment >> variable) & 1;
            node = if bit == 0 { bas } else { haut };
        }
    }
}

impl Default for MiniBdd {
    fn default() -> Self {
        Self::new()
    }
}
