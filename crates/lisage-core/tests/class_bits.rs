//! Tests of the class bits of `NodeId` (MEM-8, ADR-0103): runtime
//! derivation of `b` from `cardinality`, the compile-time window, the
//! decoder parameterized by the effective `b`, the catch-all class beyond
//! the window, and the assertion rejection of invalid identifiers.

use lisage_algebra::class_bits::{
    CLASS_BITS_MAX, CLASS_BITS_OPEN_DEFAULT, TERMINAL_CLASS, class_bits_of, derive_class_bits,
};
use lisage_algebra::interface::{KeyAlgebra, Level, NodeId};
use lisage_core::bdd::BddKeys;
use lisage_core::schema::{Domain, Schema, SchemaKeys};
use lisage_lang::value::Atom;

/// Key algebra of an open (unbounded) domain: the continuum case,
/// `C(ℓ) = u64::MAX`.
struct OpenKeys;

impl KeyAlgebra for OpenKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        u64::MAX
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

/// Same open domain, with a declarative overload of the class width.
struct OpenKeysOverload {
    declared: u32,
}

impl KeyAlgebra for OpenKeysOverload {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        u64::MAX
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }

    fn declared_class_bits(&self) -> Option<u32> {
        Some(self.declared)
    }
}

#[test]
fn test_derive_bdd_is_2_with_30_index_bits() {
    // BDD: C(ℓ) = 2, smallest b with 2^b − 2 ≥ 2 is b = 2.
    let keys = BddKeys::new(8);
    assert_eq!(keys.cardinality(Level(0)), 2);
    assert_eq!(derive_class_bits(2), 2);
    assert_eq!(class_bits_of(&keys, 8), 2);
    assert_eq!(NodeId::index_bits(2), 30);
}

#[test]
fn test_derive_philosophers_is_3_or_4() {
    // Philosophers ring: Map over a value domain of size 3 → max C(ℓ) = 3
    // → b = 3 (2^3 − 2 = 6 ≥ 3, 2^2 − 2 = 2 < 3).
    let schema = Schema::Map(
        (0..5).map(Atom).collect(),
        Domain::new(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    assert_eq!(class_bits_of(&keys, schema.number_levels()), 3);
    // A value domain of size 14 → max C(ℓ) = 14 → b = 4 (2^4 − 2 = 14).
    let schema_large = Schema::Map(
        (0..5).map(Atom).collect(),
        Domain::new((0..14).map(Atom).collect()),
    );
    let keys_large = SchemaKeys::new(schema_large.clone());
    assert_eq!(class_bits_of(&keys_large, schema_large.number_levels()), 4);
}

#[test]
fn test_open_domain_low_default_with_declarative_overload() {
    // Open domain: cardinality beyond the window → low default.
    assert_eq!(derive_class_bits(u64::MAX), CLASS_BITS_OPEN_DEFAULT);
    assert_eq!(class_bits_of(&OpenKeys, 4), CLASS_BITS_OPEN_DEFAULT);
    // Declarative overload wins, clamped to the compile-time window.
    assert_eq!(class_bits_of(&OpenKeysOverload { declared: 4 }, 4), 4);
    assert_eq!(
        class_bits_of(&OpenKeysOverload { declared: 12 }, 4),
        CLASS_BITS_MAX
    );
}

#[test]
fn test_encode_decode_roundtrip() {
    for class_bits in [0, 1, 2, 3, 8] {
        let index_bits = NodeId::index_bits(class_bits);
        let max_class = if class_bits == 0 {
            0
        } else {
            (1u32 << class_bits) - 1
        };
        let max_index = if index_bits == 32 {
            u32::MAX
        } else {
            (1u32 << index_bits) - 1
        };
        for (class, index) in [
            (0, 0),
            (max_class, 0),
            (0, max_index),
            (max_class, max_index),
        ] {
            let node = NodeId::encode(class, index, class_bits);
            assert_eq!(node.decode(class_bits), (class, index));
        }
    }
    // Layout: class in the high bits, index in the low bits.
    assert_eq!(NodeId::encode(1, 0, 2).0, 1 << 30);
    assert_eq!(NodeId::encode(0, 5, 2).0, 5);
    assert_eq!(NodeId::encode(3, 7, 2).0, (3 << 30) | 7);
}

#[test]
fn test_degree_beyond_window_lands_in_catch_all_class() {
    // b = 3: exact classes 1..=6 (2^3 − 2), catch-all = 7 (2^3 − 1).
    assert_eq!(NodeId::class_of_degree(6, 3), 6);
    assert_eq!(NodeId::class_of_degree(7, 3), NodeId::catch_all_class(3));
    assert_eq!(NodeId::catch_all_class(3), 7);
    assert_eq!(NodeId::class_of_degree(254, 8), 254);
    assert_eq!(NodeId::class_of_degree(255, 8), 255);
    assert_eq!(NodeId::class_of_degree(0, 3), TERMINAL_CLASS);
    // The catch-all class is encodable: no loss of correctness.
    let node = NodeId::encode(NodeId::catch_all_class(3), 42, 3);
    assert_eq!(node.decode(3), (7, 42));
}

#[test]
#[should_panic(expected = "class 4 out of the 2-bit class window")]
fn test_encode_rejects_class_out_of_window() {
    let _ = NodeId::encode(4, 0, 2);
}

#[test]
#[should_panic(expected = "index 1073741824 out of the 30-bit index window")]
fn test_encode_rejects_index_out_of_window() {
    let _ = NodeId::encode(0, 1 << 30, 2);
}

#[test]
#[should_panic(expected = "class field wider than the 32-bit identifier")]
fn test_decode_rejects_class_field_wider_than_identifier() {
    let _ = NodeId(0).decode(33);
}
