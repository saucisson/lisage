//! Formes relatives (M4-2, énoncé C8, spec § 2.4.5) : activation du **shift**
//! (encodage relatif porté par l'ancre), **ancres composées** (décalages
//! nommés par fragment) et **transformations de clés** désignées par le
//! contrôleur (groupe d'action + section canonique des orbites).
//!
//! Une seule idée : la forme est relative, le contexte porte l'absolu.
//! Côté `algebra/` uniquement — le pool reste aveugle et inchangé (C1) :
//! l'ancre (position) vit dans le handle, le terme de transformation
//! (valeur) est un argument sur l'arc entrant, porté par l'appelant.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

use crate::interface::{Anchor, Handle, NodeId};

/// Fragment nommé d'un réseau factorisé : l'ancre composée porte un
/// décalage par fragment. Ordre canonique : lexicographique des noms.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Fragment(pub String);

/// Ancre composée (spec § 2.4.5) : enregistrement nommé de décalages, un
/// par fragment (`{machine : +i, convoyeur : +j}`). Zéro = début de
/// fragment. Reste dans le handle (C1 inchangé), le pool reste aveugle.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct AncreComposee {
    decalages: BTreeMap<Fragment, i64>,
}

impl AncreComposee {
    /// Ancre nulle : tous les fragments en début (zéro).
    pub fn nouveau() -> Self {
        Self::default()
    }

    /// Construit une ancre avec le décalage donné pour un fragment.
    pub fn avec(mut self, fragment: Fragment, decalage: i64) -> Self {
        self.decalages.insert(fragment, decalage);
        self
    }

    /// Décalage d'un fragment ; 0 si absent (zéro = début de fragment).
    pub fn decalage(&self, fragment: &Fragment) -> i64 {
        self.decalages.get(fragment).copied().unwrap_or(0)
    }

    /// Nouvelle ancre, un fragment décalé de `delta` (la glisse).
    pub fn decalee(&self, fragment: &Fragment, delta: i64) -> Self {
        let mut resultat = self.clone();
        let courant = resultat.decalage(fragment);
        resultat.decalages.insert(fragment.clone(), courant + delta);
        resultat
    }

    /// Composition point par point des décalages (lecture absolue =
    /// composition des arguments, O(1) par pas).
    pub fn composer(&self, autre: &AncreComposee) -> Self {
        let mut resultat = self.clone();
        for (fragment, decalage) in &autre.decalages {
            let courant = resultat.decalage(fragment);
            resultat
                .decalages
                .insert(fragment.clone(), courant + *decalage);
        }
        resultat
    }

    /// Vrai si tous les décalages sont nuls (début de chaque fragment).
    pub fn est_nulle(&self) -> bool {
        self.decalages.values().all(|&d| d == 0)
    }

    /// Itérateur sur les (fragment, décalage), ordre lexicographique des noms.
    pub fn fragments(&self) -> impl Iterator<Item = (&Fragment, i64)> {
        self.decalages.iter().map(|(f, &d)| (f, d))
    }
}

/// Ancre relative : activation du shift (encodage relatif). La position
/// absolue de la racine se lit `position + decalage` ; le décalage est
/// l'argument accumulé (shift +1 par cellule, l'ancre borne le dépliage).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AncreRelative {
    /// Position de base (ancre absolue d'origine).
    pub position: u32,
    /// Décalage accumulé (shift), peut être négatif.
    pub decalage: i64,
}

/// Décalage nul : position de base.
const DECALAGE_NUL: i64 = 0;

impl AncreRelative {
    /// Ancre relative en `position`, de décalage nul.
    pub fn nouvelle(position: u32) -> Self {
        AncreRelative {
            position,
            decalage: DECALAGE_NUL,
        }
    }

    /// Depuis une ancre absolue (M1) : décalage nul.
    pub fn depuis_ancre(ancre: Anchor) -> Self {
        Self::nouvelle(ancre.0)
    }

    /// Nouvelle ancre relative, décalée de `delta` (la glisse de la
    /// fenêtre glissante avance la base de f(δ)).
    pub fn decalee(&self, delta: i64) -> Self {
        AncreRelative {
            position: self.position,
            decalage: self.decalage + delta,
        }
    }

    /// Position absolue = position de base + décalage accumulé.
    pub fn absolue(&self) -> i64 {
        self.position as i64 + self.decalage
    }

    /// Conversion en ancre absolue (saturée dans `u32`).
    pub fn vers_ancre(&self) -> Anchor {
        Anchor(self.absolue().clamp(0, u32::MAX as i64) as u32)
    }
}

/// Groupe de transformations de clés désigné par le contrôleur (spec §
/// 2.4.5). Contrat d'algèbre : (i) l'action est décidable et **inversible**
/// (un groupe : lecture absolue et produit exigent t⁻¹) ; (ii) les
/// transformations sont des **bijections** ; (iii) la section canonique est
/// **confluente** (forme normale, sinon pas de hash-consing) ; (v) la
/// famille f est close par composition avec une loi calculable :
/// f(n₁)∘f(n₂) = f(n₁ ⊕ n₂) ; (vi) l'argument est stocké **modulo le noyau
/// de f**. La fonction vit dans le contrôleur, l'arc entrant porte
/// l'argument ; le pool reste aveugle.
pub trait GroupeCles {
    /// Clé stockée (domaine sur lequel G agit).
    type Cle: Clone + Ord + Hash + Debug;
    /// Argument de la transformation (le terme de l'arc entrant).
    type Argument: Clone + Eq + Hash + Debug;

    /// Action : clé effective = f(n)(clé stockée).
    fn appliquer(&self, cle: &Self::Cle, argument: &Self::Argument) -> Self::Cle;

    /// Loi de composition des arguments : f(n₁)∘f(n₂) = f(n₁ ⊕ n₂).
    fn composer(&self, premier: &Self::Argument, second: &Self::Argument) -> Self::Argument;

    /// Argument neutre (transformation identité).
    fn neutre(&self) -> Self::Argument;

    /// Inverse t⁻¹ de la transformation d'argument donné.
    fn inverser(&self, argument: &Self::Argument) -> Self::Argument;

    /// Section canonique des orbites (confluente) : normalise les arcs
    /// d'une forme modulo G. Retourne `(argument, arcs normalisés)` —
    /// l'argument vit sur l'arc entrant, la forme stockée est la section
    /// de l'orbite.
    fn section_arcs(
        &self,
        arcs: &BTreeMap<Self::Cle, NodeId>,
    ) -> (Self::Argument, BTreeMap<Self::Cle, NodeId>);
}

/// Instance canonique : le décalage +n, G = (ℤ, +) agissant sur les clés
/// `u64`, **section « min = 0 »** — l'arc le plus petit est étiqueté 0 et
/// l'argument `n` est le minimum des clés (Wireshark : numéros de séquence
/// relatifs, précédent industriel de la clause (vi)).
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupeShift;

impl GroupeCles for GroupeShift {
    type Cle = u64;
    type Argument = i64;

    fn appliquer(&self, cle: &u64, argument: &i64) -> u64 {
        // Lecture effective saturée : la clé absolue reste dans ℕ.
        cle.saturating_add_signed(*argument)
    }

    fn composer(&self, premier: &i64, second: &i64) -> i64 {
        premier.saturating_add(*second)
    }

    fn neutre(&self) -> i64 {
        DECALAGE_NUL
    }

    fn inverser(&self, argument: &i64) -> i64 {
        argument.saturating_neg()
    }

    fn section_arcs(&self, arcs: &BTreeMap<u64, NodeId>) -> (i64, BTreeMap<u64, NodeId>) {
        match arcs.keys().next() {
            // Forme sans arc : argument neutre, forme inchangée.
            None => (self.neutre(), arcs.clone()),
            // Section min = 0 : on retranche le minimum à toutes les clés.
            Some(&min) => (
                min as i64,
                arcs.iter()
                    .map(|(cle, cible)| (cle - min, *cible))
                    .collect(),
            ),
        }
    }
}

/// Arc d'entrée : la valeur d'un DD est un **arc** (terme f(n), handle),
/// pas un nœud nu (spec § 2.4.5 « L'arc de racine »). Deux DD partageant
/// tous leurs nœuds mais différant d'entrée sont deux DD distincts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ArcEntree<Argument> {
    /// Argument de la transformation (terme f(n) porté par l'arc).
    pub argument: Argument,
    /// Handle de la racine.
    pub handle: Handle,
}

impl<Argument: Clone> ArcEntree<Argument> {
    /// Nouvel arc d'entrée (argument, handle).
    pub fn nouvelle(argument: Argument, handle: Handle) -> Self {
        ArcEntree { argument, handle }
    }

    /// Lecture absolue d'une clé stockée : clé effective = f(n)(clé).
    pub fn lire_cle<Cle>(
        &self,
        groupe: &impl GroupeCles<Cle = Cle, Argument = Argument>,
        cle: &Cle,
    ) -> Cle {
        groupe.appliquer(cle, &self.argument)
    }

    /// Composition de deux lectures : l'argument s'accumule (n₁ ⊕ n₂),
    /// O(1) par pas (invariant de Q6) ; le handle final est celui du second.
    pub fn composer(
        &self,
        groupe: &impl GroupeCles<Argument = Argument>,
        autre: &ArcEntree<Argument>,
    ) -> ArcEntree<Argument> {
        ArcEntree {
            argument: groupe.composer(&self.argument, &autre.argument),
            handle: autre.handle,
        }
    }
}
