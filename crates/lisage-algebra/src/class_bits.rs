//! Class bits of the `NodeId` budget (MEM-8, ADR-0103): `NodeId(u32)` =
//! **class bits (high) + index bits (low)**. The class field width `b` is
//! the smallest integer with `2^b − 2 ≥ max_ℓ C(ℓ)`, derived from
//! [`KeyAlgebra::cardinality`] at runtime and clamped to the compile-time
//! window [`CLASS_BITS_MAX`]. An open (unbounded) domain cannot be derived:
//! it falls back to the low default [`CLASS_BITS_OPEN_DEFAULT`] unless the
//! algebra declares an overload ([`KeyAlgebra::declared_class_bits`]).
//!
//! Class `0` is the terminal class; class `2^b − 1` is the **catch-all
//! class**: a degree beyond the window (`degree > 2^b − 2`) lands there
//! without loss of correctness. The decoder is parameterized by the
//! effective `b`; invalid identifiers are rejected by assertion.

use crate::interface::{KeyAlgebra, Level, NodeId};

/// Compile-time window of the class field: `b_max` (default 8).
pub const CLASS_BITS_MAX: u32 = 8;

/// Low default of the class field for an open (unbounded) domain: the
/// derivation is impossible, every non-terminal degree goes to the
/// catch-all class — correct, but with no exact class.
pub const CLASS_BITS_OPEN_DEFAULT: u32 = 1;

/// Terminal class: class `0` carries the terminals.
pub const TERMINAL_CLASS: u32 = 0;

/// Smallest `b` in `1..=CLASS_BITS_MAX` with `2^b − 2 ≥ max_cardinality`.
/// An open domain (cardinality beyond the window, e.g. `u64::MAX` for the
/// continuum) yields the low default [`CLASS_BITS_OPEN_DEFAULT`].
pub fn derive_class_bits(max_cardinality: u64) -> u32 {
    for bits in 1..=CLASS_BITS_MAX {
        if (1u64 << bits) - 2 >= max_cardinality {
            return bits;
        }
    }
    CLASS_BITS_OPEN_DEFAULT
}

/// Class width of a key algebra over `number_levels` levels: the
/// declarative overload wins when declared (clamped to the window),
/// otherwise the width is derived from the cardinalities at runtime.
pub fn class_bits_of<Keys>(keys: &Keys, number_levels: u32) -> u32
where
    Keys: KeyAlgebra,
{
    if let Some(declared) = keys.declared_class_bits() {
        return declared.min(CLASS_BITS_MAX);
    }
    let max_cardinality = (0..number_levels)
        .map(|level| keys.cardinality(Level(level)))
        .max()
        .unwrap_or(0);
    derive_class_bits(max_cardinality)
}

impl NodeId {
    /// Number of index bits for a class field of `class_bits` bits.
    pub fn index_bits(class_bits: u32) -> u32 {
        assert!(
            class_bits <= 32,
            "class field wider than the 32-bit identifier"
        );
        32 - class_bits
    }

    /// Catch-all class value for a class field of `class_bits` bits
    /// (`2^b − 1`): degrees beyond the window land here.
    pub fn catch_all_class(class_bits: u32) -> u32 {
        assert!(
            class_bits <= 32,
            "class field wider than the 32-bit identifier"
        );
        ((1u64 << class_bits) - 1) as u32
    }

    /// Encode `(class, index)` into a `NodeId`: the class in the high
    /// `class_bits` bits, the index in the low `32 − class_bits` bits.
    /// Fields out of their window are rejected by assertion.
    pub fn encode(class: u32, index: u32, class_bits: u32) -> NodeId {
        let index_bits = Self::index_bits(class_bits);
        assert!(
            (class as u64) < (1u64 << class_bits),
            "class {class} out of the {class_bits}-bit class window"
        );
        assert!(
            (index as u64) < (1u64 << index_bits),
            "index {index} out of the {index_bits}-bit index window"
        );
        NodeId((((class as u64) << index_bits) | index as u64) as u32)
    }

    /// Decode with the effective class width `b`: `(class, index)`.
    pub fn decode(self, class_bits: u32) -> (u32, u32) {
        let index_bits = Self::index_bits(class_bits);
        let class = (self.0 as u64 >> index_bits) as u32;
        let index_mask = if index_bits == 32 {
            u32::MAX
        } else {
            (1u32 << index_bits) - 1
        };
        (class, self.0 & index_mask)
    }

    /// Class of a node of the given `degree`: the exact class when the
    /// degree fits the window (`degree ≤ 2^b − 2`), the catch-all class
    /// beyond — without loss of correctness.
    pub fn class_of_degree(degree: u32, class_bits: u32) -> u32 {
        let max_exact = (1u64 << class_bits) - 2;
        if (degree as u64) <= max_exact {
            degree
        } else {
            Self::catch_all_class(class_bits)
        }
    }
}
