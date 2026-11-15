//! Test charter (M0-4) + A/B/C differential plugged in from M1-7.
//!
//! Triptych: path A = reference interpreter (truth), path B = fixed-order
//! DD, path C = Lisage + planner. Naming convention `test_c<n>_*`.
//!
//! The real differential tests against paths B/C are written **red**
//! before the core implementation (M1-1..M1-6): they are `#[ignore]` as long
//! as the targeted path is a placeholder, and un-ignored in the issue that
//! delivers it.

use std::collections::{BTreeMap, BTreeSet};

use lisage_bench::differentiel::{lane_a_count, lane_b_count, lane_c_count};
use lisage_bench::{lane_a, lane_b, lane_c};
use lisage_lang::interpreter::{count, filter, lookup};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

proptest! {
    /// C0 (placeholder): the three paths agree on any integer.
    #[test]
    fn test_c0_triptyque_accord(entier in 0u32..1000) {
        let a = lane_a(entier);
        let b = lane_b(entier);
        let c = lane_c(entier);
        prop_assert_eq!(a, b, "path A (reference) != path B");
        prop_assert_eq!(a, c, "path A (reference) != path C");
    }
}

#[test]
fn test_c0_exemple_factice() {
    assert_eq!(lane_a(41), 42);
    assert_eq!(lane_b(41), lane_a(41));
    assert_eq!(lane_c(41), lane_a(41));
}

fn atome(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(key, value)| (atome(*key), atome(*value)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Small set of maps generated randomly (universe {0,1} × keys {0,1}).
fn set_aleatoire(maps: Vec<(u64, u64, u64, u64)>) -> BTreeSet<Value> {
    maps.into_iter()
        .map(|(a, b, c, d)| {
            let mut map = BTreeMap::new();
            if a == 1 {
                map.insert(atome(0), atome(b));
            }
            if c == 1 {
                map.insert(atome(1), atome(d));
            }
            Value::Map(map)
        })
        .collect()
}

#[test]
#[ignore = "red (M1-7): path B placeholder until M1-6 (#13)"]
fn test_c1_differentiel_count_lane_a_vs_b() {
    let set = set_aleatoire(vec![(1, 1, 1, 0), (1, 0, 1, 0)]);
    assert_eq!(lane_a_count(&set), lane_b_count(&set));
}

#[test]
#[ignore = "red (M1-7): path C placeholder until M1-6 (#13)"]
fn test_c1_differentiel_count_lane_a_vs_c() {
    let set = set_aleatoire(vec![(1, 1, 1, 0), (1, 0, 1, 0)]);
    assert_eq!(lane_a_count(&set), lane_c_count(&set));
}

proptest! {
    /// Path A (truth): the explicit count equals the cardinality; the
    /// differential against B/C will be un-ignored when the DD paths exist.
    #[test]
    fn test_lane_a_count_egale_cardinality(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let set = set_aleatoire(maps);
        prop_assert_eq!(lane_a_count(&set), count(&set));
        prop_assert_eq!(lane_a_count(&set), set.len() as u64);
    }

    /// Path A: `filter` keeps only a subset (monotone count).
    #[test]
    fn test_lane_a_filter_count(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let set = set_aleatoire(maps);
        let garde = filter(&set, |m| m != &map_de(&[(0, 1), (1, 1)]));
        prop_assert!(count(&garde) <= count(&set));
    }

    /// Path A: `lookup(k)` of a member m equals m(k) (None if absent).
    #[test]
    fn test_lane_a_lookup(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let set = set_aleatoire(maps);
        let result = lookup(&set, &atome(0));
        prop_assert_eq!(result.len(), set.len());
        for membre in &set {
            let expected = match membre {
                Value::Map(map) => map.get(&atome(0)).cloned(),
                _ => None,
            };
            prop_assert_eq!(&result[membre], &expected);
        }
    }
}
