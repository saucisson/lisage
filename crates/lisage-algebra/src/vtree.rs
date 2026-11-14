//! Vtree (Darwiche): tree-structured order over variables —
//! a framework parameter, not a mechanism (proposal § 1.2, M5-3): sentential
//! SDDs install with a vtree as order (decomposition constraint: every
//! variable of the left subtree precedes every variable of the right
//! subtree) and a local anchor per node (controller tour as instance pattern).
//!
//! The vtree lives in `lisage-algebra` (interface layer) to be visible
//! from `lisage-plan` (which makes it a [`TypeOrdre`]) as from `lisage-core`
//! and `lisage-bench` (which derive schemas and anchors from it).

use lisage_lang::value::Atom;

/// Typed error of vtree construction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurVtree {
    /// The same variable appears in two distinct leaves: a vtree
    /// is a decomposition, each variable appears only once.
    VariableEnDouble(Atom),
}

impl std::fmt::Display for ErreurVtree {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurVtree::VariableEnDouble(atome) => {
                write!(formatter, "variable {atome:?} duplicated in the vtree")
            }
        }
    }
}

impl std::error::Error for ErreurVtree {}

/// Vtree: binary tree whose leaves carry the variables.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Vtree {
    /// Leaf: one variable.
    Feuille(Atom),
    /// Internal node: left subtree, right subtree.
    Noeud {
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
    pub fn noeud(gauche: Vtree, droite: Vtree) -> Self {
        Vtree::Noeud {
            gauche: Box::new(gauche),
            droite: Box::new(droite),
        }
    }

    /// Variables of the vtree in the induced flat order: left subtree
    /// before right subtree, recursively.
    pub fn ordre_plat(&self) -> Vec<Atom> {
        match self {
            Vtree::Feuille(variable) => vec![*variable],
            Vtree::Noeud { gauche, droite } => {
                let mut ordre = gauche.ordre_plat();
                ordre.extend(droite.ordre_plat());
                ordre
            }
        }
    }

    /// Number of variables (size of the tree).
    pub fn nombre_variables(&self) -> usize {
        self.ordre_plat().len()
    }

    /// Depth of the tree (leaf = 1).
    pub fn profondeur(&self) -> usize {
        match self {
            Vtree::Feuille(_) => 1,
            Vtree::Noeud { gauche, droite } => 1 + gauche.profondeur().max(droite.profondeur()),
        }
    }

    /// Validates the vtree: each variable appears only once.
    pub fn valider(&self) -> Result<(), ErreurVtree> {
        let mut vues = Vec::new();
        self.collecter_doublons(&mut vues)?;
        Ok(())
    }

    /// Collects the first variable seen twice, if any.
    fn collecter_doublons(&self, vues: &mut Vec<Atom>) -> Result<(), ErreurVtree> {
        match self {
            Vtree::Feuille(variable) => {
                if vues.contains(variable) {
                    return Err(ErreurVtree::VariableEnDouble(*variable));
                }
                vues.push(*variable);
                Ok(())
            }
            Vtree::Noeud { gauche, droite } => {
                gauche.collecter_doublons(vues)?;
                droite.collecter_doublons(vues)
            }
        }
    }

    /// Order constraints of the vtree: for each internal node, every
    /// left variable precedes every right variable.
    pub fn contraintes_ordre(&self) -> Vec<(Atom, Atom)> {
        let mut contraintes = Vec::new();
        self.collecter_contraintes(&mut contraintes);
        contraintes
    }

    /// Accumulates the (left × right) constraints of each internal node.
    fn collecter_contraintes(&self, contraintes: &mut Vec<(Atom, Atom)>) {
        if let Vtree::Noeud { gauche, droite } = self {
            for variable_gauche in gauche.ordre_plat() {
                for variable_droite in droite.ordre_plat() {
                    contraintes.push((variable_gauche, variable_droite));
                }
            }
            gauche.collecter_contraintes(contraintes);
            droite.collecter_contraintes(contraintes);
        }
    }
}
