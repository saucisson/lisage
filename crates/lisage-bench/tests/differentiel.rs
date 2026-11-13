//! Test charter (M0-4) + differential A/B/C wired in from M1-7.
//!
//! Triptych: lane A = reference interpreter (truth), lane B = fixed-order
//! DD, lane C = Lisage + planner. Naming convention `test_c<n>_*`.
//!
//! The real differential tests against lanes B/C are written **red**
//! before the core is implemented (M1-1..M1-6): they are `#[ignore]` as
//! long as the targeted lane is a mock, and one-ignored in the issue that
//! delivers it.

use std::collections::{BTreeMap, BTreeSet};

use lisage_bench::differential::{lane_a_count, lane_b_count, lane_c_count};
use lisage_bench::{lane_a, lane_b, lane_c};
use lisage_lang::interpreter::{count, filter, lookup};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

proptest! {
    /// C0 (mock): the three lanes agree on any integer.
    #[test]
    fn test_c0_triptych_agreement(entier in 0u32..1000) {
        let a = lane_a(entier);
        let b = lane_b(entier);
        let c = lane_c(entier);
        prop_assert_eq!(a, b, "lane A (reference) != lane B");
        prop_assert_eq!(a, c, "lane A (reference) != lane C");
    }
}

#[test]
fn test_c0_example_mock() {
    assert_eq!(lane_a(41), 42);
    assert_eq!(lane_b(41), lane_a(41));
    assert_eq!(lane_c(41), lane_a(41));
}

fn atom(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_of(pairs: &[(u64, u64)]) -> Value {
    Value::Map(
        pairs
            .iter()
            .map(|(key, value)| (atom(*key), atom(*value)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Small set of maps generated randomly (universe {0,1} × keys {0,1}).
fn set_random(maps: Vec<(u64, u64, u64, u64)>) -> BTreeSet<Value> {
    maps.into_iter()
        .map(|(a, b, c, d)| {
            let mut map = BTreeMap::new();
            if a == 1 {
                map.insert(atom(0), atom(b));
            }
            if c == 1 {
                map.insert(atom(1), atom(d));
            }
            Value::Map(map)
        })
        .collect()
}

#[test]
#[ignore = "red (M1-7): lane B mock until M1-6 (#13)"]
fn test_c1_differential_count_lane_a_vs_b() {
    let set = set_random(vec![(1, 1, 1, 0), (1, 0, 1, 0)]);
    assert_eq!(lane_a_count(&set), lane_b_count(&set));
}

#[test]
#[ignore = "red (M1-7): lane C mock until M1-6 (#13)"]
fn test_c1_differential_count_lane_a_vs_c() {
    let set = set_random(vec![(1, 1, 1, 0), (1, 0, 1, 0)]);
    assert_eq!(lane_a_count(&set), lane_c_count(&set));
}

proptest! {
    /// Lane A (truth): the explicit count equals the cardinality; the
    /// differential against B/C will be one-ignored when the DD lanes exist.
    #[test]
    fn test_lane_a_count_equal_cardinality(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let set = set_random(maps);
        prop_assert_eq!(lane_a_count(&set), count(&set));
        prop_assert_eq!(lane_a_count(&set), set.len() as u64);
    }

    /// Lane A: `filter` only keeps a subset (monotone count).
    #[test]
    fn test_lane_a_filter_count(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let set = set_random(maps);
        let keeps = filter(&set, |m| m != &map_of(&[(0, 1), (1, 1)]));
        prop_assert!(count(&keeps) <= count(&set));
    }

    /// Lane A: `lookup(k)` of a member m equals m(k) (None if absent).
    #[test]
    fn test_lane_a_lookup(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let set = set_random(maps);
        let result = lookup(&set, &atom(0));
        prop_assert_eq!(result.len(), set.len());
        for membre in &set {
            let expected = match membre {
                Value::Map(map) => map.get(&atom(0)).cloned(),
                _ => None,
            };
            prop_assert_eq!(&result[membre], &expected);
        }
    }
}
