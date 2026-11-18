//! Class and tag bits of the `NodeId` budget (MEM-8, MEM-13, ADR-0103):
//! `NodeId(u32)` = **tag bits (highest) + class bits + index bits (low)**.
//! The tag field (MEM-13 arbitration) is [`TAG_BITS`] wide: bit 0 is the
//! CUDD-style complement bit ([`TAG_COMPLEMENT`]), bits 1..=3 the shift
//! nibble of M4-2 ([`TAG_SHIFT_MASK`]). The class field width `b` is
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

/// Width of the tag field (MEM-13 arbitration of the tripartite budget,
/// ADR-0103): 4 bits, taken from the top of the identifier. Bit 0 is the
/// CUDD-style complement bit, bits 1..=3 the shift nibble of M4-2.
pub const TAG_BITS: u32 = 4;

/// Complement bit of the tag field (bit 0, CUDD-style edge transformer).
pub const TAG_COMPLEMENT: u32 = 0b0001;

/// Shift nibble of the tag field (bits 1..=3, per-arc shift amount of
/// M4-2, 0..=7).
pub const TAG_SHIFT_MASK: u32 = 0b1110;

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
    /// Number of index bits for a class field of `class_bits` bits: the
    /// tag field ([`TAG_BITS`]) sits on top of the identifier, so the
    /// index gets `32 − TAG_BITS − class_bits` low bits.
    pub fn index_bits(class_bits: u32) -> u32 {
        assert!(
            class_bits + TAG_BITS <= 32,
            "class field wider than the 32-bit identifier"
        );
        32 - TAG_BITS - class_bits
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

    /// Encode `(class, index)` into a `NodeId` with a zero tag: the tag
    /// in the highest [`TAG_BITS`] bits, the class in the next
    /// `class_bits` bits, the index in the low bits. Fields out of their
    /// window are rejected by assertion.
    pub fn encode(class: u32, index: u32, class_bits: u32) -> NodeId {
        Self::encode_tagged(0, class, index, class_bits)
    }

    /// Encode `(tag, class, index)` into a `NodeId` (MEM-13): tag in the
    /// highest [`TAG_BITS`] bits, class in the next `class_bits` bits,
    /// index in the low `32 − TAG_BITS − class_bits` bits.
    pub fn encode_tagged(tag: u32, class: u32, index: u32, class_bits: u32) -> NodeId {
        let index_bits = Self::index_bits(class_bits);
        assert!(
            (tag as u64) < (1u64 << TAG_BITS),
            "tag {tag} out of the {TAG_BITS}-bit tag window"
        );
        assert!(
            (class as u64) < (1u64 << class_bits),
            "class {class} out of the {class_bits}-bit class window"
        );
        assert!(
            (index as u64) < (1u64 << index_bits),
            "index {index} out of the {index_bits}-bit index window"
        );
        NodeId(
            (((tag as u64) << (32 - TAG_BITS)) | ((class as u64) << index_bits) | index as u64)
                as u32,
        )
    }

    /// Decode with the effective class width `b`: `(class, index)`, the
    /// tag field masked out (see [`NodeId::decode_tagged`]).
    pub fn decode(self, class_bits: u32) -> (u32, u32) {
        let (tag, class, index) = self.decode_tagged(class_bits);
        let _ = tag;
        (class, index)
    }

    /// Decode with the effective class width `b`: `(tag, class, index)`.
    pub fn decode_tagged(self, class_bits: u32) -> (u32, u32, u32) {
        let index_bits = Self::index_bits(class_bits);
        let tag = (self.0 as u64 >> (32 - TAG_BITS)) as u32;
        let class_mask = if class_bits == 32 {
            u32::MAX
        } else {
            (1u32 << class_bits) - 1
        };
        let class = ((self.0 as u64 >> index_bits) as u32) & class_mask;
        let index_mask = if index_bits == 32 {
            u32::MAX
        } else {
            (1u32 << index_bits) - 1
        };
        (tag, class, self.0 & index_mask)
    }

    /// Tag field of the identifier (MEM-13).
    pub fn tag(self, class_bits: u32) -> u32 {
        self.decode_tagged(class_bits).0
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
