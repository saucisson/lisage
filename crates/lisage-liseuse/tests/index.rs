//! D2-3 tests — inverse indexes over a trace: node → event ranges,
//! round → interval, op → interval; `explain`/`goto` by index query.
//! DoD: `explain n42` on the philosophers trace answers without linear
//! scan.

use std::sync::{Arc, Mutex};

use lisage_core::MemoryPolicy;
use lisage_core::observer::{Observer, TraceEvent};
use lisage_core::pli::{Naturel, Pli};
use lisage_core::transduction::Transducteur;
use lisage_core::{Anchor, NodeId};
use lisage_liseuse::index::IndexTrace;
use lisage_liseuse::philosophers::{build_r_n, pool_neuf, schema_anneau};

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Observer for Collector {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

/// Runs the fold `(ℕ,+,×)` on R_5 with an observer: a real philosophers
/// trace (hundreds of nodes, one op).
fn philosophers_trace() -> Vec<TraceEvent> {
    let n = 5;
    let schema = schema_anneau(n);
    let mut input = pool_neuf(&schema);
    let mut output = pool_neuf(&schema);
    let anchor = Anchor(0);
    let root = build_r_n(&mut input, anchor, n);
    let collector = Collector::default();
    let mut runner = Transducteur::with_observer(
        Pli::<Naturel>::new(schema),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, root).unwrap();
    collector.events.lock().expect("mutex poisoned").clone()
}

/// DoD: `explain n42` on the philosophers trace answers by index query —
/// the returned positions are exactly the events mentioning node 42
/// (cross-checked against a linear scan in the test only), and a node
/// absent from the trace answers empty.
#[test]
fn test_explain_n42_on_philosophers_trace() {
    let events = philosophers_trace();
    assert!(events.len() > 100, "a real trace, not a toy");
    let index = IndexTrace::build(&events);
    assert_eq!(index.length(), events.len());

    // n42: the 42nd distinct node of the philosophers trace (node ids are
    // class-encoded, so "n42" is the 42nd node, not the raw id 42).
    let mut distinct_nodes: Vec<NodeId> = events
        .iter()
        .filter_map(|e| match e {
            TraceEvent::NodeCreated { node, .. }
            | TraceEvent::NodeShared { node, .. }
            | TraceEvent::Attr { node, .. }
            | TraceEvent::MemoHit { node, .. }
            | TraceEvent::MemoMiss { node, .. }
            | TraceEvent::VisiteEntree { node, .. }
            | TraceEvent::VisiteSortie { node, .. }
            | TraceEvent::StructureChange { node, .. } => Some(*node),
            _ => None,
        })
        .collect();
    distinct_nodes.sort();
    distinct_nodes.dedup();
    assert_eq!(
        distinct_nodes.len(),
        73,
        "the R_5 fold trace has 73 distinct nodes"
    );
    assert!(
        distinct_nodes.len() >= 42,
        "at least 42 nodes in the R_5 trace"
    );
    let target = distinct_nodes[41];
    let positions = index.explain(target);
    // The node exists in the philosophers trace and the index finds it.
    assert!(
        !positions.is_empty(),
        "n42 ({target:?}) appears in the R_5 trace (positions via index)"
    );
    // Every returned position mentions the node…
    for position in &positions {
        let mentions = match &events[*position] {
            TraceEvent::NodeCreated { node, .. }
            | TraceEvent::NodeShared { node, .. }
            | TraceEvent::Attr { node, .. }
            | TraceEvent::MemoHit { node, .. }
            | TraceEvent::MemoMiss { node, .. }
            | TraceEvent::VisiteEntree { node, .. }
            | TraceEvent::VisiteSortie { node, .. }
            | TraceEvent::StructureChange { node, .. } => *node == target,
            _ => false,
        };
        assert!(mentions, "position {position} mentions n42");
    }
    // …and the index is complete: the linear scan (test-side cross-check)
    // finds exactly the same positions.
    let scan: Vec<usize> = (0..events.len())
        .filter(|&i| match &events[i] {
            TraceEvent::NodeCreated { node, .. }
            | TraceEvent::NodeShared { node, .. }
            | TraceEvent::Attr { node, .. }
            | TraceEvent::MemoHit { node, .. }
            | TraceEvent::MemoMiss { node, .. }
            | TraceEvent::VisiteEntree { node, .. }
            | TraceEvent::VisiteSortie { node, .. }
            | TraceEvent::StructureChange { node, .. } => *node == target,
            _ => false,
        })
        .collect();
    assert_eq!(positions, scan, "explain == linear scan (completeness)");

    // Ranges are merged when contiguous.
    for range in index.node_ranges(target) {
        assert!(range.end > range.start);
    }

    // A node absent from the trace answers empty, by lookup.
    let absent = NodeId(u32::MAX);
    assert!(index.explain(absent).is_empty());
    assert!(index.node_ranges(absent).is_empty());
}

/// Round → interval and op → interval, with `goto` by number.
#[test]
fn test_index_rounds_and_ops() {
    let events = vec![
        TraceEvent::OpStart {
            fingerprint: 1,
            span: None,
        },
        TraceEvent::Round {
            numero: 1,
            niveaux_actifs: vec![],
            niveaux_modifies: vec![],
            avant: NodeId(0),
            image: NodeId(1),
            apres: NodeId(1),
            croissance: 0,
            span: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: None,
            forme: lisage_core::observer::TraceVal::Elide,
            span: None,
        },
        TraceEvent::Round {
            numero: 2,
            niveaux_actifs: vec![],
            niveaux_modifies: vec![],
            avant: NodeId(1),
            image: NodeId(2),
            apres: NodeId(2),
            croissance: 1,
            span: None,
        },
        TraceEvent::OpEnd { span: None },
    ];
    let index = IndexTrace::build(&events);

    // Op interval: OpStart..=OpEnd.
    assert_eq!(index.op_range(0), Some(0..5));
    assert_eq!(index.op_position(0), Some(0));
    assert!(index.op_range(1).is_none());

    // Round intervals: round 1 = [1, 3), round 2 = [3, 5).
    assert_eq!(index.round_range(1), Some(1..3));
    assert_eq!(index.round_range(2), Some(3..5));
    assert_eq!(index.round_position(2), Some(3));
    assert!(index.round_range(3).is_none());

    // Node ranges: node 1 at position 2 only.
    assert_eq!(index.explain(NodeId(1)), vec![2]);
}
