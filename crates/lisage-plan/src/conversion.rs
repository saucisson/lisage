//! Conversions between orders and controller realms (M3-2, claim C3,
//! spec § 2.4.3 and § 10 Q1): **reorder = convert** — a conversion
//! between two [`Order<T>`] of the same type is compiled into a transduction
//! (sequence of adjacent level transpositions) at **predictable
//! cost** (number of inversions, Kendall tau distance, computed
//! from the orders alone). The **typed contract extends from orders to
//! encodings**: a value exists only in a [`Royaume`] typed by
//! (type `T`, order mark `O`, encoding mark `E`), and leaving the
//! realm goes through an explicit conversion.

use std::marker::PhantomData;

use crate::ordre::{MarqueOrdre, Order, TypeOrdre};

/// Encoding of a type (controller realm, M3-2): compilation of the
/// semantic decomposition into physical levels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encodage {
    /// Universe sweep: one presence level per key, δ = absent,
    /// no marker (spec § 2.1, M1 compilation).
    BalayageUnivers,
    /// Enumerative compilation: positions + end marker, bounded by
    /// length (constructor `Seq`, M2-1).
    Enumeration,
}

/// Encoding mark: zero-sized type carrying an [`Encodage`]. The typed
/// contract of realms goes through this mark, like the order through
/// [`MarqueOrdre`].
pub trait MarqueEncodage: Sized + Send + Sync {
    /// The encoding associated with the mark.
    fn encodage() -> Encodage;
}

/// Typed conversion error (cause preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurConversion {
    /// The two orders do not have the same canonical key set:
    /// no conversion possible between those types.
    OrdresInconvertibles,
    /// The conversion does not start from the source realm's order or
    /// does not arrive at the target realm's order.
    RoyaumeIncompatible,
}

impl std::fmt::Display for ErreurConversion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurConversion::OrdresInconvertibles => {
                write!(
                    formatter,
                    "inconvertible orders (different canonical keys)"
                )
            }
            ErreurConversion::RoyaumeIncompatible => {
                write!(
                    formatter,
                    "conversion incompatible with source/target realms"
                )
            }
        }
    }
}

impl std::error::Error for ErreurConversion {}

/// Conversion between two orders of the same type `T`: **reorder =
/// convert**. Compiled into a transduction — sequence of adjacent level
/// transpositions — at predictable cost: the number of inversions
/// between the two π (Kendall tau distance), computed from the orders
/// alone, before any execution.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Conversion<T: TypeOrdre> {
    source: Order<T>,
    cible: Order<T>,
    /// Adjacent transpositions (positions in the sequence) composing
    /// π_source → π_cible, in application order.
    transpositions: Vec<(usize, usize)>,
}

impl<T: TypeOrdre> Conversion<T> {
    /// Conversion from `source` to `cible`: both orders must have the same
    /// canonical key set (same instantiated type).
    pub fn nouveau(source: &Order<T>, cible: &Order<T>) -> Result<Self, ErreurConversion> {
        if source.canoniques() != cible.canoniques() {
            return Err(ErreurConversion::OrdresInconvertibles);
        }
        // Bubble sort of the target sequence toward the source order,
        // recording each adjacent transposition: the number of
        // transpositions equals the number of inversions.
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

    /// Starting order of the conversion.
    pub fn source(&self) -> &Order<T> {
        &self.source
    }

    /// Arrival order of the conversion.
    pub fn cible(&self) -> &Order<T> {
        &self.cible
    }

    /// Predictable cost of the transduction: number of inversions between
    /// the two π = number of adjacent transpositions.
    pub fn cout(&self) -> usize {
        self.transpositions.len()
    }

    /// Adjacent transpositions (positions) composing the transduction.
    pub fn transpositions(&self) -> &[(usize, usize)] {
        &self.transpositions
    }

    /// The conversion is the identity (zero cost) iff both orders are
    /// aligned.
    pub fn est_identite(&self) -> bool {
        self.cout() == 0
    }
}

/// Variance marker of `Royaume`: no bound on `T`, `O`, `E`.
type MarqueurRoyaume<T, O, E> = PhantomData<fn() -> (T, O, E)>;

/// Controller realm: typed contract (type `T`, order mark `O`,
/// encoding mark `E`) — extension of the typed contract from orders to
/// encodings (M3-2, C3). A value exists only in a realm;
/// leaving the realm = explicit conversion.
pub struct Royaume<T: TypeOrdre, O: MarqueOrdre<T>, E: MarqueEncodage, V> {
    valeur: V,
    marqueur: MarqueurRoyaume<T, O, E>,
}

impl<T: TypeOrdre, O: MarqueOrdre<T>, E: MarqueEncodage, V> Royaume<T, O, E, V> {
    /// New value in the realm `(T, O, E)`.
    pub fn nouveau(valeur: V) -> Self {
        Royaume {
            valeur,
            marqueur: PhantomData,
        }
    }

    /// Value carried by the realm.
    pub fn valeur(&self) -> &V {
        &self.valeur
    }

    /// Order of the realm, from its mark.
    pub fn ordre(descripteur: &T) -> Order<T> {
        O::ordre(descripteur)
    }

    /// Encoding of the realm, from its mark.
    pub fn encodage() -> Encodage {
        E::encodage()
    }
}

impl<T: TypeOrdre, O: MarqueOrdre<T>, E: MarqueEncodage, V: Clone> Royaume<T, O, E, V> {
    /// **Reorder = convert**: change the realm's order mark via an explicit
    /// conversion at predictable cost (`conversion.cout()`).
    /// Typed contract checked by value: the conversion must start from the
    /// source realm's order (`O`) and arrive at the target realm's order
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
