//! Ordres typés de première classe (M3-1, énoncé C3, spec § 10 Q1) :
//! [`Order<T>`] est une **valeur** — la permutation π des clés canoniques
//! du type `T`, avec les contraintes d'ordre du type embarquées et
//! vérifiées à la construction. **Alignement comme typage** : deux
//! opérandes ne se composent que sous un ordre commun, imposé par le
//! paramètre de type [`MarqueOrdre`] (et vérifié en valeur par
//! [`Order::alignement`]).
//!
//! C'est le point de jonction entre le calcul des transductions (T-EXPAND
//! consulte l'ordre : le niveau réinséré prend sa place déclarée) et le
//! plan de représentation.

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use lisage_lang::Atom;

/// Type ordonnable : clés canoniques du type + contraintes d'ordre
/// (paires `avant`/`après` exigées dans toute permutation valide).
pub trait TypeOrdre {
    /// Clé ordonnée d'un niveau du type.
    type Cle: Clone + Ord + Hash + Debug;

    /// Séquence canonique des clés (l'ordre par défaut du type).
    fn cles_canoniques(&self) -> Vec<Self::Cle>;

    /// Contraintes d'ordre du type : `avant` doit précéder `après` dans π.
    fn contraintes(&self) -> Vec<(Self::Cle, Self::Cle)> {
        Vec::new()
    }
}

/// Descripteur du type `Atom(D)` / `Set(D)` : ordre sur les atomes du
/// domaine borné (canonique = atomes triés, sans contrainte).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DomaineAtomes(pub Vec<Atom>);

impl TypeOrdre for DomaineAtomes {
    type Cle = Atom;

    fn cles_canoniques(&self) -> Vec<Atom> {
        let mut cles = self.0.clone();
        cles.sort();
        cles.dedup();
        cles
    }
}

/// Phase d'un niveau de `Map(K, V)` : présence de la clé, puis valeur.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Phase {
    /// Niveau de présence de la clé (δ = absent).
    Presence,
    /// Niveau valeur de la clé.
    Valeur,
}

/// Clé d'un niveau de `Map(K, V)` : (clé, phase).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CleMap {
    /// Clé de la map.
    pub cle: Atom,
    /// Phase du niveau : présence ou valeur.
    pub phase: Phase,
}

/// Descripteur du type `Map(K, V)` : contrainte par clé — la présence
/// précède la valeur (balayage d'univers, spec § 2.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClesMap(pub Vec<Atom>);

impl ClesMap {
    /// Clés canoniques triées (sans doublons).
    fn cles_triees(&self) -> Vec<Atom> {
        let mut cles = self.0.clone();
        cles.sort();
        cles.dedup();
        cles
    }
}

impl TypeOrdre for ClesMap {
    type Cle = CleMap;

    fn cles_canoniques(&self) -> Vec<CleMap> {
        let mut niveaux = Vec::new();
        for cle in self.cles_triees() {
            niveaux.push(CleMap {
                cle,
                phase: Phase::Presence,
            });
            niveaux.push(CleMap {
                cle,
                phase: Phase::Valeur,
            });
        }
        niveaux
    }

    fn contraintes(&self) -> Vec<(CleMap, CleMap)> {
        self.cles_triees()
            .into_iter()
            .map(|cle| {
                (
                    CleMap {
                        cle,
                        phase: Phase::Presence,
                    },
                    CleMap {
                        cle,
                        phase: Phase::Valeur,
                    },
                )
            })
            .collect()
    }
}

/// Erreur typée de construction ou d'alignement d'ordre (cause préservée).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurOrdre<Cle> {
    /// π n'est pas une permutation des clés canoniques (clé manquante, en
    /// double, ou étrangère au type).
    PasUnePermutation,
    /// Contrainte du type violée : `avant` doit précéder `après` dans π.
    ContrainteViolee {
        /// Clé qui devait précéder.
        avant: Cle,
        /// Clé qui devait suivre.
        apres: Cle,
    },
    /// Deux ordres du même type ne sont pas alignés (π différents).
    OrdresNonAlignes,
}

impl<Cle: Debug> std::fmt::Display for ErreurOrdre<Cle> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurOrdre::PasUnePermutation => {
                write!(formatter, "π n'est pas une permutation des clés canoniques")
            }
            ErreurOrdre::ContrainteViolee { avant, apres } => {
                write!(
                    formatter,
                    "contrainte violée: {avant:?} doit précéder {apres:?}"
                )
            }
            ErreurOrdre::OrdresNonAlignes => {
                write!(formatter, "ordres non alignés (π différents)")
            }
        }
    }
}

impl<Cle: Debug> std::error::Error for ErreurOrdre<Cle> {}

/// Ordre total sur les clés d'un type `T` — **valeur de première classe**
/// (M3-1, C3) : la permutation π des clés canoniques, les contraintes du
/// type étant embarquées et vérifiées à la construction.
#[derive(Debug)]
pub struct Order<T: TypeOrdre> {
    /// Clés canoniques du type (π = identité sur cette séquence).
    canoniques: Vec<T::Cle>,
    /// π : séquence ordonnée des clés (permutation des canoniques).
    sequence: Vec<T::Cle>,
    /// Contraintes d'ordre du type, embarquées.
    contraintes: Vec<(T::Cle, T::Cle)>,
    marqueur: PhantomData<fn() -> T>,
}

// Implémentations manuelles : les derives ajouteraient des bornes sur `T`
// lui-même, alors que seules les clés `T::Cle` participent aux comparaisons.
impl<T: TypeOrdre> Clone for Order<T> {
    fn clone(&self) -> Self {
        Order {
            canoniques: self.canoniques.clone(),
            sequence: self.sequence.clone(),
            contraintes: self.contraintes.clone(),
            marqueur: PhantomData,
        }
    }
}

impl<T: TypeOrdre> PartialEq for Order<T> {
    fn eq(&self, autre: &Self) -> bool {
        self.canoniques == autre.canoniques
            && self.sequence == autre.sequence
            && self.contraintes == autre.contraintes
    }
}

impl<T: TypeOrdre> Eq for Order<T> {}

impl<T: TypeOrdre> Order<T> {
    /// Ordre canonique du type (π = identité).
    pub fn canonique(descripteur: &T) -> Self {
        let canoniques = descripteur.cles_canoniques();
        Order {
            sequence: canoniques.clone(),
            canoniques,
            contraintes: descripteur.contraintes(),
            marqueur: PhantomData,
        }
    }

    /// Ordre donné par la permutation π, vérifiée : π doit être une
    /// permutation des clés canoniques et satisfaire les contraintes du
    /// type.
    pub fn nouveau(descripteur: &T, pi: Vec<T::Cle>) -> Result<Self, ErreurOrdre<T::Cle>> {
        let canoniques = descripteur.cles_canoniques();
        let mut pi_triee = pi.clone();
        pi_triee.sort();
        let mut canoniques_triees = canoniques.clone();
        canoniques_triees.sort();
        if pi_triee != canoniques_triees {
            return Err(ErreurOrdre::PasUnePermutation);
        }
        let ordre = Order {
            canoniques,
            sequence: pi,
            contraintes: descripteur.contraintes(),
            marqueur: PhantomData,
        };
        for (avant, apres) in &ordre.contraintes {
            let satisfait = match (ordre.position(avant), ordre.position(apres)) {
                (Some(position_avant), Some(position_apres)) => position_avant < position_apres,
                _ => false,
            };
            if !satisfait {
                return Err(ErreurOrdre::ContrainteViolee {
                    avant: <T::Cle as Clone>::clone(avant),
                    apres: <T::Cle as Clone>::clone(apres),
                });
            }
        }
        Ok(ordre)
    }

    /// Position (0-based) d'une clé dans π.
    pub fn position(&self, cle: &T::Cle) -> Option<usize> {
        self.sequence.iter().position(|c| c == cle)
    }

    /// Clé à une position de π.
    pub fn cle(&self, position: usize) -> Option<&T::Cle> {
        self.sequence.get(position)
    }

    /// Nombre de clés de l'ordre.
    pub fn taille(&self) -> usize {
        self.sequence.len()
    }

    /// Séquence ordonnée des clés (π lui-même).
    pub fn sequence(&self) -> &[T::Cle] {
        &self.sequence
    }

    /// Clés canoniques du type (π = identité sur cette séquence).
    pub fn canoniques(&self) -> &[T::Cle] {
        &self.canoniques
    }

    /// π comme tableau : indice dans la séquence canonique → position dans π.
    pub fn permutation(&self) -> Vec<usize> {
        self.canoniques
            .iter()
            .map(|cle| self.position(cle).expect("clé canonique présente dans π"))
            .collect()
    }

    /// Contraintes d'ordre embarquées dans `Order<T>`.
    pub fn contraintes(&self) -> &[(T::Cle, T::Cle)] {
        &self.contraintes
    }

    /// Deux ordres du même type sont alignés ssi même π.
    pub fn alignes(&self, autre: &Self) -> bool {
        self.sequence == autre.sequence
    }

    /// Alignement vérifié entre deux ordres du même type, erreur typée
    /// sinon (C3 : deux opérandes ne se composent que sous un ordre commun).
    pub fn alignement(&self, autre: &Self) -> Result<(), ErreurOrdre<T::Cle>> {
        if self.alignes(autre) {
            Ok(())
        } else {
            Err(ErreurOrdre::OrdresNonAlignes)
        }
    }
}

/// Marque d'ordre : type zéro-taille portant l'[`Order<T>`] d'un type `T`.
/// L'alignement devient un fait de typage : deux opérandes ne se composent
/// que sous la même marque `O` (C3).
pub trait MarqueOrdre<T: TypeOrdre>: Sized + Send + Sync {
    /// L'ordre (valeur de première classe) associé à la marque.
    fn ordre(descripteur: &T) -> Order<T>;
}

/// Opérande aligné sur l'ordre `O` du type `T`. La composition
/// ([`Aligne::composer`]) exige le même paramètre `O` des deux côtés :
/// des opérandes sous des ordres différents ne compilent pas.
pub struct Aligne<T: TypeOrdre, O: MarqueOrdre<T>, V> {
    valeur: V,
    marqueur: PhantomData<fn() -> (T, O)>,
}

impl<T: TypeOrdre, O: MarqueOrdre<T>, V> Aligne<T, O, V> {
    /// Opérande neuf, aligné sur la marque `O`.
    pub fn nouveau(valeur: V) -> Self {
        Aligne {
            valeur,
            marqueur: PhantomData,
        }
    }

    /// Valeur portée par l'opérande.
    pub fn valeur(&self) -> &V {
        &self.valeur
    }

    /// Composition sous ordre commun : `autre` partage le même `(T, O)` —
    /// l'alignement est imposé par le typage, pas vérifié à l'exécution.
    pub fn composer<U, W>(
        &self,
        autre: &Aligne<T, O, U>,
        combinaison: impl FnOnce(&V, &U) -> W,
    ) -> Aligne<T, O, W> {
        Aligne::nouveau(combinaison(&self.valeur, autre.valeur()))
    }
}
