//! Conversions between orders and controller kingdoms (M3-2, statement C3,
//! spec § 2.4.3 and § 10 Q1): **reorder = convert** — a conversion between
//! two [`Order<T>`] of the same type is compiled into a transduction
//! (sequence of adjacent level transpositions) at **predictable cost**
//! (number of inversions, Kendall tau distance, computed from the orders
//! alone). The **typed contract extends from orders to encodings**: a value
//! only exists in a [`Kingdom`] typed by (type `T`, order mark `O`,
//! encoding mark `E`), and leaving the kingdom goes through an explicit
//! conversion.

use std::marker::PhantomData;

use crate::order::{MarkedOrder, Order, TypeOrder};

/// Encoding of a type (controller kingdom, M3-2): compilation of the
/// semantic decomposition into physical levels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    /// Universe sweep: one presence level per key, δ = absent,
    /// no marker (spec § 2.1, M1 compilation).
    SweepUnivers,
    /// Enumerative compilation: positions + end marker, bounded by the
    /// length (constructor `Seq`, M2-1).
    Enumeration,
}

/// Encoding mark: zero-sized type carrying an [`Encoding`]. The typed
/// contract of kingdoms goes through this mark, like the order through
/// [`MarkedOrder`].
pub trait MarkedEncoding: Sized + Send + Sync {
    /// The encoding associated with the mark.
    fn encoding() -> Encoding;
}

/// Typed conversion error (cause preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorConversion {
    /// The two orders do not have the same canonical key set: no conversion
    /// possible between those types.
    OrdersUnconvertible,
    /// The conversion does not start from the source kingdom's order or
    /// does not arrive at the target kingdom's order.
    KingdomIncompatible,
}

impl std::fmt::Display for ErrorConversion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorConversion::OrdersUnconvertible => {
                write!(formatter, "unconvertible orders (different canonical keys)")
            }
            ErrorConversion::KingdomIncompatible => {
                write!(
                    formatter,
                    "conversion incompatible with the source/target kingdoms"
                )
            }
        }
    }
}

impl std::error::Error for ErrorConversion {}

/// Conversion between two orders of the same type `T`: **reorder =
/// convert**. Compiled into a transduction — sequence of adjacent level
/// transpositions — at predictable cost: the number of inversions between
/// the two π (Kendall tau distance), computed from the orders alone,
/// before any execution.
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
        if source.canonical_keys() != target.canonical_keys() {
            return Err(ErrorConversion::OrdersUnconvertible);
        }
        // Bubble sort of the target sequence towards the source order,
        // recording each adjacent transposition: the number of
        // transpositions equals the number of inversions.
        let mut sequence = target.sequence().to_vec();
        let mut transpositions = Vec::new();
        let mut sorted = false;
        while !sorted {
            sorted = true;
            for index in 0..sequence.len().saturating_sub(1) {
                let position_current = source.position(&sequence[index]);
                let position_next = source.position(&sequence[index + 1]);
                if position_current > position_next {
                    sequence.swap(index, index + 1);
                    transpositions.push((index, index + 1));
                    sorted = false;
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
    pub fn cost(&self) -> usize {
        self.transpositions.len()
    }

    /// Adjacent transpositions (positions) composing the transduction.
    pub fn transpositions(&self) -> &[(usize, usize)] {
        &self.transpositions
    }

    /// The conversion is the identity (zero cost) iff the two orders are
    /// aligned.
    pub fn is_identity(&self) -> bool {
        self.cost() == 0
    }
}

/// Variance marker of `Kingdom`: no bounds on `T`, `O`, `E`.
type MarkerKingdom<T, O, E> = PhantomData<fn() -> (T, O, E)>;

/// Controller kingdom: typed contract (type `T`, order mark `O`, encoding
/// mark `E`) — extension of the typed contract from orders to encodings
/// (M3-2, C3). A value only exists in a kingdom; leaving the kingdom =
/// explicit conversion.
pub struct Kingdom<T: TypeOrder, O: MarkedOrder<T>, E: MarkedEncoding, V> {
    value: V,
    marker: MarkerKingdom<T, O, E>,
}

impl<T: TypeOrder, O: MarkedOrder<T>, E: MarkedEncoding, V> Kingdom<T, O, E, V> {
    /// New value in the kingdom `(T, O, E)`.
    pub fn new(value: V) -> Self {
        Kingdom {
            value,
            marker: PhantomData,
        }
    }

    /// Value carried by the kingdom.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Order of the kingdom, from its mark.
    pub fn order(descripteur: &T) -> Order<T> {
        O::order(descripteur)
    }

    /// Encoding of the kingdom, from its mark.
    pub fn encoding() -> Encoding {
        E::encoding()
    }
}

impl<T: TypeOrder, O: MarkedOrder<T>, E: MarkedEncoding, V: Clone> Kingdom<T, O, E, V> {
    /// **Reorder = convert**: changes the kingdom's order mark via an
    /// explicit conversion at predictable cost (`conversion.cost()`).
    /// Typed contract checked by value: the conversion must start from the
    /// source kingdom's order (`O`) and arrive at the target kingdom's
    /// order (`O2`).
    pub fn reorder<O2: MarkedOrder<T>>(
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
