//! PBT of rank/unrank (MEM-4, ADR-0103): round-trip `rank ∘ unrank` on
//! the bounded domains of the reference instance, and the declared
//! absence of rank on an unbounded domain (DDD path unchanged).

use lisage_algebra::interface::{KeyAlgebra, Level};
use lisage_core::bdd::BddKeys;
use lisage_core::schema::{Domain, Schema, SchemaKeys};
use lisage_lang::value::Atom;
use proptest::prelude::*;

fn schema_keys(size: u64) -> SchemaKeys {
    let values: Vec<Atom> = (0..size).map(Atom).collect();
    SchemaKeys::new(Schema::Atom(Domain::new(values)))
}

proptest! {
    /// Round-trip on a bounded atom domain of the reference instance:
    /// `rank(level, unrank(level, r)) = r` for every ordinal in range,
    /// and both directions return `None` out of range.
    #[test]
    fn test_rank_unrank_roundtrip_reference_instance(size in 1u64..64, rang in 0u64..80) {
        let keys = schema_keys(size);
        let level = Level(0);
        if rang < size {
            let key = keys.unrank(level, rang).expect("ordinal in range");
            prop_assert_eq!(keys.rank(level, &key), Some(rang));
        } else {
            prop_assert_eq!(keys.unrank(level, rang), None);
        }
        let key = rang;
        if key < size {
            prop_assert_eq!(keys.rank(level, &key), Some(key));
        } else {
            prop_assert_eq!(keys.rank(level, &key), None);
        }
    }

    /// Round-trip on the BDD algebra (bounded domain of cardinality 2).
    #[test]
    fn test_rank_unrank_roundtrip_bdd(rang in 0u64..4) {
        let keys = BddKeys::new(8);
        let level = Level(3);
        if rang < 2 {
            let key = keys.unrank(level, rang).expect("ordinal in range");
            prop_assert_eq!(keys.rank(level, &key), Some(rang));
        } else {
            prop_assert_eq!(keys.unrank(level, rang), None);
            prop_assert_eq!(keys.rank(level, &rang), None);
        }
    }
}

/// Unbounded domain: the absence of rank is properly declared (the
/// default `None`), the DDD path is unchanged.
#[test]
fn test_rank_absent_declared_on_unbounded_domain() {
    struct UnboundedKeys;
    impl KeyAlgebra for UnboundedKeys {
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
    let keys = UnboundedKeys;
    assert_eq!(keys.rank(Level(0), &0), None);
    assert_eq!(keys.unrank(Level(0), 0), None);
}
