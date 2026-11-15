//! Conversions between orders and controller realms (M3-2, claim C3,
//! spec § 2.4.3 and § 10 Q1): **reorder = convert** — a conversion
//! between two [`Order<T>`] of the same type is compiled into a transduction
//! (sequence of adjacent level transpositions) at **predictable
//! cost** (number of inversions, Kendall tau distance, computed
//! from the orders alone). The **typed contract extends from orders to
//! encodings**: a value exists only in a [`Kingdom`] typed by
//! (type `T`, order mark `O`, encoding mark `E`), and leaving the
//! realm goes through an explicit conversion.

use std::marker::PhantomData;

use crate::order::{MarqueOrder, Order, TypeOrder};

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
/// [`MarqueOrder`].
pub trait MarqueEncodage: Sized + Send + Sync {
    /// The encoding associated with the mark.
    fn encodage() -> Encodage;
}

/// Typed conversion error (cause preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorConversion {
    /// The two orders do not have the same canonical key set:
    /// no conversion possible between those types.
    OrdersInconvertibles,
    /// The conversion does not start from the source realm's order or
    /// does not arrive at the target realm's order.
    KingdomIncompatible,
}

impl std::fmt::Display for ErrorConversion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorConversion::OrdersInconvertibles => {
                write!(formatter, "inconvertible orders (different canonical keys)")
            }
            ErrorConversion::KingdomIncompatible => {
                write!(
                    formatter,
                    "conversion incompatible with source/target realms"
                )
            }
        }
    }
}

impl std::error::Error for ErrorConversion {}

/// Conversion between two orders of the same type `T`: **reorder =
/// convert**. Compiled into a transduction — sequence of adjacent level
/// transpositions — at predictable cost: the number of inversions
/// between the two π (Kendall tau distance), computed from the orders
/// alone, before any execution.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Conversion<T: TypeOrder> {
    source: Order<T>,
    target: Order<T>,
    /// Adjacent transpositions (positions in the sequence) composing
    /// π_source → π_target, in application order.
    transpositions: Vec<(usize, usize)>,
}

impl<T: TypeOrder> Conversion<T> {
    /// Conversion from `source` to `target`: both orders must have the same
    /// canonical key set (same instantiated type).
    pub fn new(source: &Order<T>, target: &Order<T>) -> Result<Self, ErrorConversion> {
        if source.canoniques() != target.canoniques() {
            return Err(ErrorConversion::OrdersInconvertibles);
        }
        // Bubble sort of the target sequence toward the source order,
        // recording each adjacent transposition: the number of
        // transpositions equals the number of inversions.
        let mut sequence = target.sequence().to_vec();
        let mut transpositions = Vec::new();
        let mut triee = false;
        while !triee {
            triee = true;
            for index in 0..sequence.len().saturating_sub(1) {
                let position_courante = source.position(&sequence[index]);
                let position_nexte = source.position(&sequence[index + 1]);
                if position_courante > position_nexte {
                    sequence.swap(index, index + 1);
                    transpositions.push((index, index + 1));
                    triee = false;
                }
            }
        }
        debug_assert_eq!(sequence, source.sequence());
        Ok(Conversion {
            source: source.clone(),
            target: target.clone(),
            transpositions,
        })
    }

    /// Starting order of the conversion.
    pub fn source(&self) -> &Order<T> {
        &self.source
    }

    /// Arrival order of the conversion.
    pub fn target(&self) -> &Order<T> {
        &self.target
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

/// Variance marker of `Kingdom`: no bound on `T`, `O`, `E`.
type MarkerKingdom<T, O, E> = PhantomData<fn() -> (T, O, E)>;

/// Controller realm: typed contract (type `T`, order mark `O`,
/// encoding mark `E`) — extension of the typed contract from orders to
/// encodings (M3-2, C3). A value exists only in a realm;
/// leaving the realm = explicit conversion.
pub struct Kingdom<T: TypeOrder, O: MarqueOrder<T>, E: MarqueEncodage, V> {
    value: V,
    marker: MarkerKingdom<T, O, E>,
}

impl<T: TypeOrder, O: MarqueOrder<T>, E: MarqueEncodage, V> Kingdom<T, O, E, V> {
    /// New value in the realm `(T, O, E)`.
    pub fn new(value: V) -> Self {
        Kingdom {
            value,
            marker: PhantomData,
        }
    }

    /// Value carried by the realm.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Order of the realm, from its mark.
    pub fn order(descripteur: &T) -> Order<T> {
        O::order(descripteur)
    }

    /// Encoding of the realm, from its mark.
    pub fn encodage() -> Encodage {
        E::encodage()
    }
}

impl<T: TypeOrder, O: MarqueOrder<T>, E: MarqueEncodage, V: Clone> Kingdom<T, O, E, V> {
    /// **Reorder = convert**: change the realm's order mark via an explicit
    /// conversion at predictable cost (`conversion.cout()`).
    /// Typed contract checked by value: the conversion must start from the
    /// source realm's order (`O`) and arrive at the target realm's order
    /// (`O2`).
    pub fn reordonner<O2: MarqueOrder<T>>(
        &self,
        descripteur: &T,
        conversion: &Conversion<T>,
    ) -> Result<Kingdom<T, O2, E, V>, ErrorConversion> {
        if conversion.source() != &O::order(descripteur)
            || conversion.target() != &O2::order(descripteur)
        {
            return Err(ErrorConversion::KingdomIncompatible);
        }
        Ok(Kingdom::new(self.value.clone()))
    }
}
