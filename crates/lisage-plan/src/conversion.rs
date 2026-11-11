//! Conversions entre ordres et royaumes de contrôleurs (M3-2, énoncé C3,
//! spec § 2.4.3 et § 10 Q1) : **réordonner = convertir** — une conversion
//! entre deux [`Order<T>`] d'un même type est compilée en transduction
//! (séquence de transpositions adjacentes de niveaux) à **coût
//! prévisible** (nombre d'inversions, distance de Kendall tau, calculée
//! depuis les seuls ordres). Le **contrat typé s'étend des ordres aux
//! encodages** : une valeur n'existe que dans un [`Royaume`] typé par
//! (type `T`, marque d'ordre `O`, marque d'encodage `E`), et quitter le
//! royaume passe par une conversion explicite.

use std::marker::PhantomData;

use crate::ordre::{MarqueOrdre, Order, TypeOrdre};

/// Encodage d'un type (royaume de contrôleur, M3-2) : compilation de la
/// décomposition sémantique en niveaux physiques.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encodage {
    /// Balayage d'univers : un niveau de présence par clé, δ = absent,
    /// sans marqueur (spec § 2.1, compilation de M1).
    BalayageUnivers,
    /// Compilation énumérative : positions + marqueur de fin, bornée par
    /// la longueur (constructeur `Seq`, M2-1).
    Enumeration,
}

/// Marque d'encodage : type zéro-taille portant un [`Encodage`]. Le
/// contrat typé des royaumes passe par cette marque, comme l'ordre par
/// [`MarqueOrdre`].
pub trait MarqueEncodage: Sized + Send + Sync {
    /// L'encodage associé à la marque.
    fn encodage() -> Encodage;
}

/// Erreur typée de conversion (cause préservée).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurConversion {
    /// Les deux ordres n'ont pas le même ensemble canonique de clés :
    /// pas de conversion possible entre ces types-là.
    OrdresInconvertibles,
    /// La conversion ne part pas de l'ordre du royaume source ou
    /// n'arrive pas à l'ordre du royaume cible.
    RoyaumeIncompatible,
}

impl std::fmt::Display for ErreurConversion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurConversion::OrdresInconvertibles => {
                write!(
                    formatter,
                    "ordres inconvertibles (clés canoniques différentes)"
                )
            }
            ErreurConversion::RoyaumeIncompatible => {
                write!(
                    formatter,
                    "conversion incompatible avec les royaumes source/cible"
                )
            }
        }
    }
}

impl std::error::Error for ErreurConversion {}

/// Conversion entre deux ordres d'un même type `T` : **réordonner =
/// convertir**. Compilée en transduction — séquence de transpositions
/// adjacentes de niveaux — à coût prévisible : le nombre d'inversions
/// entre les deux π (distance de Kendall tau), calculé depuis les seuls
/// ordres, avant toute exécution.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Conversion<T: TypeOrdre> {
    source: Order<T>,
    cible: Order<T>,
    /// Transpositions adjacentes (positions dans la séquence) composant
    /// π_source → π_cible, dans l'ordre d'application.
    transpositions: Vec<(usize, usize)>,
}

impl<T: TypeOrdre> Conversion<T> {
    /// Conversion de `source` vers `cible` : les deux ordres doivent avoir
    /// le même ensemble canonique de clés (même type instancié).
    pub fn nouveau(source: &Order<T>, cible: &Order<T>) -> Result<Self, ErreurConversion> {
        if source.canoniques() != cible.canoniques() {
            return Err(ErreurConversion::OrdresInconvertibles);
        }
        // Tri à bulles de la séquence cible vers l'ordre source, en
        // enregistrant chaque transposition adjacente : le nombre de
        // transpositions égale le nombre d'inversions.
        let mut sequence = cible.sequence().to_vec();
        let mut transpositions = Vec::new();
        let mut triee = false;
        while !triee {
            triee = true;
            for index in 0..sequence.len().saturating_sub(1) {
                let position_courante = source.position(&sequence[index]);
                let position_suivante = source.position(&sequence[index + 1]);
                if position_courante > position_suivante {
                    sequence.swap(index, index + 1);
                    transpositions.push((index, index + 1));
                    triee = false;
                }
            }
        }
        debug_assert_eq!(sequence, source.sequence());
        Ok(Conversion {
            source: source.clone(),
            cible: cible.clone(),
            transpositions,
        })
    }

    /// Ordre de départ de la conversion.
    pub fn source(&self) -> &Order<T> {
        &self.source
    }

    /// Ordre d'arrivée de la conversion.
    pub fn cible(&self) -> &Order<T> {
        &self.cible
    }

    /// Coût prévisible de la transduction : nombre d'inversions entre les
    /// deux π = nombre de transpositions adjacentes.
    pub fn cout(&self) -> usize {
        self.transpositions.len()
    }

    /// Transpositions adjacentes (positions) composant la transduction.
    pub fn transpositions(&self) -> &[(usize, usize)] {
        &self.transpositions
    }

    /// La conversion est l'identité (coût nul) ssi les deux ordres sont
    /// alignés.
    pub fn est_identite(&self) -> bool {
        self.cout() == 0
    }
}

/// Marqueur de variance de `Royaume` : aucune borne sur `T`, `O`, `E`.
type MarqueurRoyaume<T, O, E> = PhantomData<fn() -> (T, O, E)>;

/// Royaume de contrôleur : contrat typé (type `T`, marque d'ordre `O`,
/// marque d'encodage `E`) — extension du contrat typé des ordres aux
/// encodages (M3-2, C3). Une valeur n'existe que dans un royaume ;
/// quitter le royaume = conversion explicite.
pub struct Royaume<T: TypeOrdre, O: MarqueOrdre<T>, E: MarqueEncodage, V> {
    valeur: V,
    marqueur: MarqueurRoyaume<T, O, E>,
}

impl<T: TypeOrdre, O: MarqueOrdre<T>, E: MarqueEncodage, V> Royaume<T, O, E, V> {
    /// Valeur neuf dans le royaume `(T, O, E)`.
    pub fn nouveau(valeur: V) -> Self {
        Royaume {
            valeur,
            marqueur: PhantomData,
        }
    }

    /// Valeur portée par le royaume.
    pub fn valeur(&self) -> &V {
        &self.valeur
    }

    /// Ordre du royaume, d'après sa marque.
    pub fn ordre(descripteur: &T) -> Order<T> {
        O::ordre(descripteur)
    }

    /// Encodage du royaume, d'après sa marque.
    pub fn encodage() -> Encodage {
        E::encodage()
    }
}

impl<T: TypeOrdre, O: MarqueOrdre<T>, E: MarqueEncodage, V: Clone> Royaume<T, O, E, V> {
    /// **Réordonner = convertir** : change la marque d'ordre du royaume
    /// via une conversion explicite à coût prévisible (`conversion.cout()`).
    /// Contrat typé vérifié en valeur : la conversion doit partir de
    /// l'ordre du royaume source (`O`) et arriver à celui du royaume cible
    /// (`O2`).
    pub fn reordonner<O2: MarqueOrdre<T>>(
        &self,
        descripteur: &T,
        conversion: &Conversion<T>,
    ) -> Result<Royaume<T, O2, E, V>, ErreurConversion> {
        if conversion.source() != &O::ordre(descripteur)
            || conversion.cible() != &O2::ordre(descripteur)
        {
            return Err(ErreurConversion::RoyaumeIncompatible);
        }
        Ok(Royaume::nouveau(self.valeur.clone()))
    }
}
