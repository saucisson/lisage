//! Vtree (Darwiche): order structured as a tree over the variables —
//! a parameter of the framework, not a mechanism (proposal § 1.2, M5-3):
//! sentential SDDs install with a vtree as order (decomposition constraint:
//! every variable of the left subtree precedes every variable of the right
//! subtree) and a local anchor per node (controller tower as instance
//! pattern).
//!
//! The vtree lives in `lisage-algebra` (interface layer) to be visible from
//! `lisage-plan` (which makes a [`TypeOrder`] of it) as well as `lisage-core`
//! and `lisage-bench` (which derive schemas and anchors from it).

use lisage_lang::value::Atom;

/// Typed error of vtree construction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorVtree {
    /// The same variable appears in two distinct leaves: a vtree is a
    /// decomposition, each variable appears in it only once.
    VariableInDouble(Atom),
}

impl std::fmt::Display for ErrorVtree {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorVtree::VariableInDouble(atom) => {
                write!(formatter, "variable {atom:?} appears twice in the vtree")
            }
        }
    }
}

impl std::error::Error for ErrorVtree {}

/// Vtree: binary tree whose leaves carry the variables.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Vtree {
    /// Leaf: one variable.
    Leaf(Atom),
    /// Internal node: left subtree, right subtree.
    Node {
        /// Left subtree of variables.
        left: Box<Vtree>,
        /// Right subtree of variables.
        right: Box<Vtree>,
    },
}

impl Vtree {
    /// Builds a leaf.
    pub fn leaf(variable: Atom) -> Self {
        Vtree::Leaf(variable)
    }

    /// Builds an internal node.
    pub fn node(left: Vtree, right: Vtree) -> Self {
        Vtree::Node {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Variables of the vtree in the induced flat order: left subtree
    /// before right subtree, recursively.
    pub fn order_flat(&self) -> Vec<Atom> {
        match self {
            Vtree::Leaf(variable) => vec![*variable],
            Vtree::Node { left, right } => {
                let mut order = left.order_flat();
                order.extend(right.order_flat());
                order
            }
        }
    }

    /// Number of variables (size of the tree).
    pub fn number_variables(&self) -> usize {
        self.order_flat().len()
    }

    /// Depth of the tree (leaf = 1).
    pub fn depth(&self) -> usize {
        match self {
            Vtree::Leaf(_) => 1,
            Vtree::Node { left, right } => 1 + left.depth().max(right.depth()),
        }
    }

    /// Validates the vtree: each variable appears only once.
    pub fn validate(&self) -> Result<(), ErrorVtree> {
        let mut vues = Vec::new();
        self.collect_duplicates(&mut vues)?;
        Ok(())
    }

    /// Collects the first variable seen twice, if any.
    fn collect_duplicates(&self, vues: &mut Vec<Atom>) -> Result<(), ErrorVtree> {
        match self {
            Vtree::Leaf(variable) => {
                if vues.contains(variable) {
                    return Err(ErrorVtree::VariableInDouble(*variable));
                }
                vues.push(*variable);
                Ok(())
            }
            Vtree::Node { left, right } => {
                left.collect_duplicates(vues)?;
                right.collect_duplicates(vues)
            }
        }
    }

    /// Order constraints of the vtree: for each internal node, every left
    /// variable precedes every right variable.
    pub fn constraints_order(&self) -> Vec<(Atom, Atom)> {
        let mut constraints = Vec::new();
        self.collect_constraints(&mut constraints);
        constraints
    }

    /// Accumulates the constraints (left × right) of each internal node.
    fn collect_constraints(&self, constraints: &mut Vec<(Atom, Atom)>) {
        if let Vtree::Node { left, right } = self {
            for variable_left in left.order_flat() {
                for variable_right in right.order_flat() {
                    constraints.push((variable_left, variable_right));
                }
            }
            left.collect_constraints(constraints);
            right.collect_constraints(constraints);
        }
    }
}
