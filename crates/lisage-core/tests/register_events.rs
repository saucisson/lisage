//! D1-2 register events: the runner emits `OpStart`/`OpEnd`,
//! `VisitEntry`/`VisitExit`, `MemoConsult`/`MemoInsert`,
//! `MemoryWrite` and a `StructureChange` per node visit, classified by
//! comparison of the input/output forms. Dedicated pins: the T-EXPAND of
//! `lookup` φ_out (restructuring of the `present` arc) and the A2 collapse
//! of `Filter` φ_out (node reduced to ε(false)).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Pool};
use lisage_core::MemoryPolicy;
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::{Observer, StructureChangeKind, TraceEvent};
use lisage_core::operations::{Filter, PresenceKeys};
use lisage_core::schema::{
    ABSENT, Domain, PRESENT, ReferenceController, Schema, SchemaKeys, compiler,
};
use lisage_core::transduction::Transducer;
use lisage_lang::value::{Atom, Value};

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collector {
    fn new() -> Self {
        Self::default()
    }
    fn take(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("mutex poisoned").clone()
    }
}

impl Observer for Collector {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

fn atom(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_from(pairs: &[(u64, u64)]) -> Value {
    Value::Map(
        pairs
            .iter()
            .map(|(key, value)| (atom(*key), atom(*value)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// D1-2: one full register sequence on a small run — `OpStart` first,
/// `OpEnd` last, each visit bracketed by `VisitEntry`/`VisitExit`, each
/// miss followed by a `MemoConsult(hit=false)` + `MemoInsert` pair.
#[test]
fn test_sequence_register_per_visit() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = compiler(
        &mut input,
        anchor,
        &schema,
        &Value::Set(BTreeSet::from([atom(0), atom(1)])),
    )
    .unwrap();

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        lisage_core::fold::Fold::<lisage_core::fold::Natural>::new(schema.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, root).unwrap();
    let events = collector.take();

    assert!(
        matches!(
            events.first(),
            Some(TraceEvent::SpanOpen {
                kind: lisage_core::observer::SpanKind::Op,
                ..
            })
        ),
        "the trace opens with the Op span"
    );
    assert!(
        matches!(events.get(1), Some(TraceEvent::OpStart { .. })),
        "OpStart follows the opening of the Op span"
    );
    assert!(
        matches!(events.get(events.len() - 2), Some(TraceEvent::OpEnd { .. })),
        "OpEnd precedes the closing of the Op span"
    );
    assert!(
        matches!(events.last(), Some(TraceEvent::SpanClose { .. })),
        "the trace closes with the Op SpanClose"
    );
    let entries = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::VisitEntry { .. }))
        .count();
    let exits = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::VisitExit { .. }))
        .count();
    assert_eq!(entries, exits, "one VisiteSortie per VisiteEntree");
    assert!(entries >= 2, "the root and its children are visited");
    // Every MemoConsult(hit=false) is followed by a MemoInsert with the
    // same full key (fingerprint, node, h, version).
    for (index, event) in events.iter().enumerate() {
        if let TraceEvent::MemoConsult {
            fingerprint,
            node,
            h,
            version,
            hit: false,
            ..
        } = event
        {
            let insert = events[index + 1..].iter().find_map(|e| match e {
                TraceEvent::MemoInsert {
                    fingerprint: f,
                    node: n,
                    h: hh,
                    version: v,
                    ..
                } if f == fingerprint && n == node && hh == h && v == version => Some(()),
                _ => None,
            });
            assert!(
                insert.is_some(),
                "MemoInsert follows the MemoConsult of node {node:?}"
            );
        }
    }
    // Every visit ends with a StructureChange carrying the visited node.
    for event in &events {
        if let TraceEvent::VisitExit { node, .. } = event {
            assert!(
                events.iter().any(|e| matches!(
                    e,
                    TraceEvent::StructureChange { node: n, .. } if n == node
                )),
                "a StructureChange is emitted for the visited node {node:?}"
            );
        }
    }
}

/// D1-2 DoD: on the T-EXPAND of `lookup` φ_out (collapsed value level), the
/// presence-level node is restructured — its `present` arc is re-pointed
/// from the collapsed terminal to the re-explicit value-level node.
#[test]
fn test_structure_change_t_expand_lookup() {
    let list_keys = vec![Atom(0)];
    let domain_values = Domain::new(vec![Atom(0)]);
    let schema = Schema::Map(list_keys.clone(), domain_values.clone());
    let value = map_from(&[(0, 0)]);

    let mut input = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let mut output = Arena::new(
        LookupController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = compiler(&mut input, anchor, &schema, &value).unwrap();

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        Lookup::new(Atom(0), list_keys.clone(), domain_values.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, root).unwrap();
    let events = collector.take();

    let restructurations: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::StructureChange {
                    classification: StructureChangeKind::Restructuration,
                    ..
                }
            )
        })
        .collect();
    assert!(
        restructurations.iter().any(|e| {
            if let TraceEvent::StructureChange {
                arcs_before,
                arcs_after,
                ..
            } = e
            {
                // T-EXPAND: the `present` arc changes target (collapsed
                // terminal → re-explicit value-level node), the `absent`
                // arc is preserved.
                let before_present = arcs_before.iter().find(|(key, _)| *key == PRESENT);
                let after_present = arcs_after.iter().find(|(key, _)| *key == PRESENT);
                let before_absent = arcs_before.iter().find(|(key, _)| *key == ABSENT);
                let after_absent = arcs_after.iter().find(|(key, _)| *key == ABSENT);
                before_present.is_some()
                    && after_present.is_some()
                    && before_present != after_present
                    && before_absent == after_absent
            } else {
                false
            }
        }),
        "T-EXPAND: a StructureChange restructures the present arc: {events:?}"
    );
}

/// D1-2 DoD: on the A2 collapse of `Filter` φ_out (all children rejected ⇒
/// ε(false)), the visited node collapses from a node form to a terminal.
#[test]
fn test_structure_change_collapse_filter_a2() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let keys = SchemaKeys::new(schema.clone());
    let set = BTreeSet::from([map_from(&[(0, 1)]), map_from(&[(1, 0)])]);
    let mut root = input.intern(anchor, lisage_algebra::interface::Form::Terminal(false));
    for membre in &set {
        let membre_node = compiler(&mut input, anchor, &schema, membre).unwrap();
        root = lisage_core::fold::union(&mut input, &keys, anchor, root, membre_node).unwrap();
    }

    // Guard rejecting every member: A2 collapses each map node, then the
    // root, to ε(false).
    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        Filter::new(&schema, Box::new(|_keys: &PresenceKeys| false)).unwrap(),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    let (result, _) = runner.run(&input, &mut output, anchor, root).unwrap();
    assert!(
        matches!(
            output.resolve(result),
            Ok(lisage_algebra::interface::Form::Terminal(false))
        ),
        "A2: the whole set collapses to ε(false)"
    );

    let events = collector.take();
    let collapses: Vec<&TraceEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::StructureChange {
                    classification: StructureChangeKind::Collapse,
                    ..
                }
            )
        })
        .collect();
    assert!(
        collapses.iter().any(|e| {
            if let TraceEvent::StructureChange {
                node,
                arcs_before,
                arcs_after,
                ..
            } = e
            {
                *node == root && !arcs_before.is_empty() && arcs_after.is_empty()
            } else {
                false
            }
        }),
        "A2: the root node collapses (arcs before, no arcs after): {events:?}"
    );
}
