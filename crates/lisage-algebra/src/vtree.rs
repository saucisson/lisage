//! Vtree (Darwiche) : ordre structuré en arbre sur les variables —
//! paramètre du cadre, pas un mécanisme (proposition § 1.2, M5-3) : les
//! SDD sententiels s'installent avec un vtree comme ordre (contrainte de
//! décomposition : toute variable du sous-arbre gauche précède toute
//! variable du sous-arbre droit) et une ancre locale par nœud (tour de
//! contrôleurs comme motif d'instance).
//!
//! Le vtree vit dans `lisage-algebra` (couche interface) pour être visible
//! de `lisage-plan` (qui en fait un [`TypeOrdre`]) comme de `lisage-core`
//! et `lisage-bench` (qui en dérivent des schémas et des ancres).

use lisage_lang::value::Atom;

/// Erreur typée de construction d'un vtree.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurVtree {
    /// La même variable apparaît dans deux feuilles distinctes : un vtree
    /// est une décomposition, chaque variable n'y figure qu'une fois.
    VariableEnDouble(Atom),
}

impl std::fmt::Display for ErreurVtree {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurVtree::VariableEnDouble(atome) => {
                write!(formatter, "variable {atome:?} en double dans le vtree")
            }
        }
    }
}

impl std::error::Error for ErreurVtree {}

/// Vtree : arbre binaire dont les feuilles portent les variables.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Vtree {
    /// Feuille : une variable.
    Feuille(Atom),
    /// Nœud interne : sous-arbre gauche, sous-arbre droit.
    Noeud {
        /// Sous-arbre des variables de gauche.
        gauche: Box<Vtree>,
        /// Sous-arbre des variables de droite.
        droite: Box<Vtree>,
    },
}

impl Vtree {
    /// Construit une feuille.
    pub fn feuille(variable: Atom) -> Self {
        Vtree::Feuille(variable)
    }

    /// Construit un nœud interne.
    pub fn noeud(gauche: Vtree, droite: Vtree) -> Self {
        Vtree::Noeud {
            gauche: Box::new(gauche),
            droite: Box::new(droite),
        }
    }

    /// Variables du vtree dans l'ordre plat induit : sous-arbre gauche
    /// avant sous-arbre droit, récursivement.
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

    /// Nombre de variables (taille de l'arbre).
    pub fn nombre_variables(&self) -> usize {
        self.ordre_plat().len()
    }

    /// Profondeur de l'arbre (feuille = 1).
    pub fn profondeur(&self) -> usize {
        match self {
            Vtree::Feuille(_) => 1,
            Vtree::Noeud { gauche, droite } => 1 + gauche.profondeur().max(droite.profondeur()),
        }
    }

    /// Valide le vtree : chaque variable n'apparaît qu'une seule fois.
    pub fn valider(&self) -> Result<(), ErreurVtree> {
        let mut vues = Vec::new();
        self.collecter_doublons(&mut vues)?;
        Ok(())
    }

    /// Collecte la première variable vue en double, s'il y en a une.
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

    /// Contraintes d'ordre du vtree : pour chaque nœud interne, toute
    /// variable de gauche précède toute variable de droite.
    pub fn contraintes_ordre(&self) -> Vec<(Atom, Atom)> {
        let mut contraintes = Vec::new();
        self.collecter_contraintes(&mut contraintes);
        contraintes
    }

    /// Accumule les contraintes (gauche × droite) de chaque nœud interne.
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
