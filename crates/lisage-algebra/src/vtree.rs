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
    VariableEnDouble(Atom),
}

impl std::fmt::Display for ErrorVtree {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorVtree::VariableEnDouble(atome) => {
                write!(formatter, "variable {atome:?} duplicated in the vtree")
            }
        }
    }
}

impl std::error::Error for ErrorVtree {}

/// Vtree: binary tree whose leaves carry the variables.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Vtree {
    /// Leaf: one variable.
    Feuille(Atom),
    /// Internal node: left subtree, right subtree.
    Node {
        /// Subtree of left variables.
        gauche: Box<Vtree>,
        /// Subtree of right variables.
        droite: Box<Vtree>,
    },
}

impl Vtree {
    /// Builds a leaf.
    pub fn feuille(variable: Atom) -> Self {
        Vtree::Feuille(variable)
    }

    /// Builds an internal node.
    pub fn node(gauche: Vtree, droite: Vtree) -> Self {
        Vtree::Node {
            gauche: Box::new(gauche),
            droite: Box::new(droite),
        }
    }

    /// Variables of the vtree in the induced flat order: left subtree
    /// before right subtree, recursively.
    pub fn order_plat(&self) -> Vec<Atom> {
        match self {
            Vtree::Feuille(variable) => vec![*variable],
            Vtree::Node { gauche, droite } => {
                let mut order = gauche.order_plat();
                order.extend(droite.order_plat());
                order
            }
        }
    }

    /// Number of variables (size of the tree).
    pub fn number_variables(&self) -> usize {
        self.order_plat().len()
    }

    /// Depth of the tree (leaf = 1).
    pub fn profondeur(&self) -> usize {
        match self {
            Vtree::Feuille(_) => 1,
            Vtree::Node { gauche, droite } => 1 + gauche.profondeur().max(droite.profondeur()),
        }
    }

    /// Validates the vtree: each variable appears only once.
    pub fn valider(&self) -> Result<(), ErrorVtree> {
        let mut views = Vec::new();
        self.collecter_doublons(&mut views)?;
        Ok(())
    }

    /// Collects the first variable seen twice, if any.
    fn collecter_doublons(&self, views: &mut Vec<Atom>) -> Result<(), ErrorVtree> {
        match self {
            Vtree::Feuille(variable) => {
                if views.contains(variable) {
                    return Err(ErrorVtree::VariableEnDouble(*variable));
                }
                views.push(*variable);
                Ok(())
            }
            Vtree::Node { gauche, droite } => {
                gauche.collecter_doublons(views)?;
                droite.collecter_doublons(views)
            }
        }
    }

    /// Order constraints of the vtree: for each internal node, every
    /// left variable precedes every right variable.
    pub fn contraintes_order(&self) -> Vec<(Atom, Atom)> {
        let mut contraintes = Vec::new();
        self.collecter_contraintes(&mut contraintes);
        contraintes
    }

    /// Accumulates the (left × right) constraints of each internal node.
    fn collecter_contraintes(&self, contraintes: &mut Vec<(Atom, Atom)>) {
        if let Vtree::Node { gauche, droite } = self {
            for variable_gauche in gauche.order_plat() {
                for variable_droite in droite.order_plat() {
                    contraintes.push((variable_gauche, variable_droite));
                }
            }
            gauche.collecter_contraintes(contraintes);
            droite.collecter_contraintes(contraintes);
        }
    }
}
