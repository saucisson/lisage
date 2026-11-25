//! D1-3 tests for `TraceVal` / `Traceable`: bounds of the mini-ADT,
//! elision (lazy fragments are computed only when forced), the Debug
//! fallback, and the structured impls of the H/S/M types of the existing
//! transductions.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Level, NodeId};
use lisage_core::fold::{BoolOr, Natural};
use lisage_core::lookup::Mode;
use lisage_core::observer::{
    TRACE_VAL_MAX_DEPTH, TRACE_VAL_MAX_TEXT, TRACE_VAL_MAX_TOKENS, TraceVal, Traceable,
};
use lisage_core::operations::PresenceKeys;
use lisage_core::transfer::StateTransfer;

/// D1-3 DoD: the ADT is bounded — texts are truncated, collections capped
/// with an `Elided` marker.
#[test]
fn test_trace_val_bounds() {
    // Text bound.
    let long = "x".repeat(TRACE_VAL_MAX_TEXT * 2);
    let value = TraceVal::text(long);
    let TraceVal::Text(text) = &value else {
        panic!("expected text")
    };
    assert_eq!(text.chars().count(), TRACE_VAL_MAX_TEXT + 1);
    assert!(text.ends_with('…'));

    // Set bound: cap + Elided marker.
    let values: Vec<TraceVal> = (0..TRACE_VAL_MAX_TOKENS as i64 + 10)
        .map(TraceVal::integer)
        .collect();
    let TraceVal::Set(set) = TraceVal::set(values) else {
        panic!("expected set")
    };
    assert_eq!(set.len(), TRACE_VAL_MAX_TOKENS + 1);
    assert_eq!(set.last(), Some(&TraceVal::Elided));

    // Map bound: cap + Elided pair.
    let pairs: Vec<(TraceVal, TraceVal)> = (0..TRACE_VAL_MAX_TOKENS as i64 + 5)
        .map(|i| (TraceVal::integer(i), TraceVal::integer(i)))
        .collect();
    let TraceVal::Map(map) = TraceVal::map(pairs) else {
        panic!("expected map")
    };
    assert_eq!(map.len(), TRACE_VAL_MAX_TOKENS + 1);
    assert_eq!(map.last(), Some(&(TraceVal::Elided, TraceVal::Elided)));

    // Small collections are untouched.
    let petite = TraceVal::set(vec![TraceVal::integer(1), TraceVal::integer(2)]);
    assert_eq!(petite.render(), "{1, 2}");
}

/// D1-3 DoD: elision — a lazy fragment is **not** computed until forced.
#[test]
fn test_trace_val_elision_paresseuse() {
    let counter = Arc::new(AtomicU64::new(0));
    let counter_clone = counter.clone();
    let fragment = TraceVal::Fragment(Arc::new(move || {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        TraceVal::text("calcule")
    }));

    // Not forced: the payload is not computed, equality is by allocation.
    assert_eq!(counter.load(Ordering::Relaxed), 0);
    assert_eq!(fragment.clone(), fragment);
    assert_ne!(fragment, TraceVal::text("calcule"));
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    // Forced: computed exactly once per force call.
    assert_eq!(fragment.render(), "calcule");
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    assert_eq!(fragment.force().render(), "calcule");
    assert_eq!(counter.load(Ordering::Relaxed), 2);

    // Fragment chains are bounded in depth: beyond the bound, elision.
    let mut profond = TraceVal::integer(0);
    for _ in 0..TRACE_VAL_MAX_DEPTH + 5 {
        let next = profond;
        profond = TraceVal::Fragment(Arc::new(move || next.clone()));
    }
    assert_eq!(profond.render(), "…");
}

/// D1-3: the `Traceable` default is the lazy Debug fallback; the H/S/M
/// types of the existing transductions have structured impls.
#[test]
fn test_traceable_impls() {
    // Debug fallback (lazy): a custom type without a specific impl.
    #[derive(Clone, Debug)]
    struct Quelconque(#[allow(dead_code)] u64);
    impl Traceable for Quelconque {}
    let v = Quelconque(7).trace_val();
    assert!(matches!(v, TraceVal::Fragment(_)));
    assert_eq!(v.render(), "Quelconque(7)");

    // H of lookup: Mode.
    assert_eq!(Mode::Seek.trace_val().render(), "Cherche");
    assert_eq!(Mode::Extract.trace_val().render(), "Extrait");
    assert_eq!(Mode::Stop(3).trace_val().render(), "{Stop: 3}");

    // H of exists/filter: PresenceKeys.
    let mut keys: PresenceKeys = BTreeSet::new();
    keys.insert(0);
    keys.insert(2);
    assert_eq!(keys.trace_val().render(), "{0, 2}");

    // S of fold: Natural / BoolOr; H/S/M of image: unit elides.
    assert_eq!(Natural(42).trace_val().render(), "42");
    assert_eq!(BoolOr(true).trace_val().render(), "1");
    assert_eq!(().trace_val().render(), "…");
    assert_eq!(true.trace_val().render(), "1");

    // S of lookup: fragment map.
    let mut fragment = BTreeMap::new();
    fragment.insert(1u64, NodeId(3));
    assert_eq!(fragment.trace_val().render(), "{1: 3}");

    // M of transfer: bool; H of transfer: StateTransfer.
    assert_eq!(StateTransfer::Normal.trace_val().render(), "Normal");
    assert_eq!(
        StateTransfer::Transfer(1).trace_val().render(),
        "{Transfert: 1}"
    );

    // Memory writes: Vec<(NodeId, Level, M)>.
    let writes = vec![(NodeId(3), Level(1), true)];
    assert_eq!(writes.trace_val().render(), "{{3: 1, …: 1}}");

    // NodeId / Level / u64.
    assert_eq!(NodeId(9).trace_val().render(), "9");
    assert_eq!(Level(2).trace_val().render(), "2");
    assert_eq!(5u64.trace_val().render(), "5");
}
