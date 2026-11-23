//! Vtree (Darwiche): tree-structured order over variables —
//! a framework parameter, not a mechanism (proposal § 1.2, M5-3): sentential
//! SDDs install with a vtree as order (decomposition constraint: every
//! variable of the left subtree precedes every variable of the right
//! subtree) and a local anchor per node (controller tour as instance pattern).
//!
//! The vtree lives in `lisage-algebra` (interface layer) to be visible
//! from `lisage-plan` (which makes it a [`TypeOrder`]) as from `lisage-core`
//! and `lisage-bench` (which derive schemas and anchors from it).

use lisage_lang::value::Atom;

/// Typed error of vtree construction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorVtree {
    /// The same variable appears in two distinct leaves: a vtree
    /// is a decomposition, each variable appears only once.
    DuplicateVariable(Atom),
}

impl std::fmt::Display for ErrorVtree {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorVtree::DuplicateVariable(atom) => {
                write!(formatter, "variable {atom:?} duplicated in the vtree")
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
        /// Subtree of left variables.
        left: Box<Vtree>,
        /// Subtree of right variables.
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
    pub fn order_plat(&self) -> Vec<Atom> {
        match self {
            Vtree::Leaf(variable) => vec![*variable],
            Vtree::Node { left, right } => {
                let mut order = left.order_plat();
                order.extend(right.order_plat());
                order
            }
        }
    }

    /// Number of variables (size of the tree).
    pub fn number_variables(&self) -> usize {
        self.order_plat().len()
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
        let mut views = Vec::new();
        self.collect_doublons(&mut views)?;
        Ok(())
    }

    /// Collects the first variable seen twice, if any.
    fn collect_doublons(&self, views: &mut Vec<Atom>) -> Result<(), ErrorVtree> {
        match self {
            Vtree::Leaf(variable) => {
                if views.contains(variable) {
                    return Err(ErrorVtree::DuplicateVariable(*variable));
                }
                views.push(*variable);
                Ok(())
            }
            Vtree::Node { left, right } => {
                left.collect_doublons(views)?;
                right.collect_doublons(views)
            }
        }
    }

    /// Order constraints of the vtree: for each internal node, every
    /// left variable precedes every right variable.
    pub fn constraints_order(&self) -> Vec<(Atom, Atom)> {
        let mut constraints = Vec::new();
        self.collect_constraints(&mut constraints);
        constraints
    }

    /// Accumulates the (left × right) constraints of each internal node.
    fn collect_constraints(&self, constraints: &mut Vec<(Atom, Atom)>) {
        if let Vtree::Node { left, right } = self {
            for variable_left in left.order_plat() {
                for variable_right in right.order_plat() {
                    constraints.push((variable_left, variable_right));
                }
            }
            left.collect_constraints(constraints);
            right.collect_constraints(constraints);
        }
    }
}
