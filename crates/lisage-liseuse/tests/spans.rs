//! D2-4 tests — span tree (nested intervals) and folded balances
//! (prefix sums) over a trace, convergence marker, and round diff.
//! DoD: a saturation presented as N balance lines; diff round 11 vs 12
//! operational.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_core::arena::Arena;
use lisage_core::observer::{Observer, SpanKind, TraceEvent};
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{
    ABSENT, Domain, PRESENT, ReferenceController, Schema, SchemaKeys, compiler,
};
use lisage_core::transduction::{PhiOutContext, Transduction};
use lisage_core::{Anchor, Form, Level, NodeId, Pool};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::index::IndexTrace;
use lisage_liseuse::spans::{
    Balances, SpanTree, convergence_round, diff_rounds, render_balance_lines, round_intervals,
    round_lines,
};

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collector {
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

/// Step transduction `τ(X) = {{m}}` where `m = max atom of X + 1`: an
/// increasing chain of singletons — starting from `{{0}}`, round `k`
/// adds the singleton `{k}`, so a domain of size `D` saturates in
/// exactly `D` rounds (the last one detects the fixpoint, growth 0).
///
/// The synthesized attribute carries `max atom present + 1` (`0` =
/// none); the root's `phi_out` interns the singleton chain `{m}`.
#[derive(Clone)]
struct Successor {
    /// Number of levels of the `Set` schema (domain size).
    number_levels: u32,
}

impl Transduction for Successor {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        4163
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}
    fn phi_aggr(&self, level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        // Synthesized: `max atom present in the subtree + 1`.
        let mut max = 0u64;
        for (arc, (_, seen_below)) in arcs {
            max = max.max(*seen_below);
            if *arc == PRESENT {
                max = max.max(level.0 as u64 + 1);
            }
        }
        max
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<()>) {
        // `m = max atom of this subtree + 1` (recomputed from the
        // children's synthesized attributes, as at the root).
        let mut m = 0u64;
        for (arc, (_, seen_below)) in &ctx.arcs {
            m = m.max(*seen_below);
            if *arc == PRESENT {
                m = m.max(ctx.level.0 as u64 + 1);
            }
        }
        let node = if m >= self.number_levels as u64 {
            // The full domain is present: emit nothing.
            pool.intern(ctx.anchor, Form::Terminal(false))
        } else {
            // Intern the singleton chain `{m}`: PRESENT at level `m`,
            // ABSENT (default) everywhere else, down to the terminal.
            let mut current = pool.intern(ctx.anchor, Form::Terminal(true));
            for level in (0..self.number_levels).rev() {
                let arcs: BTreeMap<u64, NodeId> = if level as u64 == m {
                    BTreeMap::from([(PRESENT, current)])
                } else {
                    BTreeMap::from([(ABSENT, current)])
                };
                current = pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: Level(level),
                        arcs: arcs.into(),
                    },
                );
            }
            current
        };
        (node, lisage_core::memory::EffectMemory::new())
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), 0)
    }
}

/// Runs a saturation of the chain `{{0}} → {{0},{0,D-1}} → …` over a
/// `Set` domain of size `n`, with an observer: a real trace with
/// exactly `n` rounds.
fn saturation_trace(n: u64) -> Vec<TraceEvent> {
    let schema = Schema::Set(Domain::new((0..n).map(Atom).collect()));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let initial = compiler(
        &mut pool,
        anchor,
        &schema,
        &Value::Set([Value::Atom(Atom(0))].into_iter().collect()),
    )
    .unwrap();

    let collector = Collector::default();
    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy = PolicySaturation::Static(levels);
    let step = Successor {
        number_levels: schema.number_levels(),
    };
    let mut params = SaturationParameters {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(Arc::new(collector.clone())),
    };
    let result = saturate(&mut params).unwrap();
    assert_eq!(
        result.iterations, n as u32,
        "the chain saturates in n rounds"
    );
    collector.take()
}

/// D2-4: the span tree of a saturation — one `Round` root per round,
/// each holding one `Op` span, itself holding the `Visit` spans; the
/// children intervals are nested in the parent interval.
#[test]
fn test_span_tree_saturation_n4() {
    let events = saturation_trace(4);
    let tree = SpanTree::build(&events);

    // One root per round, all of kind Round.
    assert_eq!(tree.roots().len(), 4, "one Round span per round");
    for root in tree.roots() {
        assert_eq!(root.kind, SpanKind::Round);
        assert_eq!(root.children.len(), 1, "one Op span per round");
        assert_eq!(root.children[0].kind, SpanKind::Op);
        assert!(
            !root.children[0].children.is_empty(),
            "the Op span holds the Visite spans"
        );
        for child in &root.children[0].children {
            assert_eq!(child.kind, SpanKind::Visit);
        }
    }
    // Nesting: every child interval is inside its parent's.
    fn check_nested(node: &lisage_liseuse::spans::SpanNode) {
        for child in &node.children {
            assert!(node.range.start <= child.range.start);
            assert!(child.range.end <= node.range.end);
            check_nested(child);
        }
    }
    for root in tree.roots() {
        check_nested(root);
    }
    // Depth: Round > Op > Visit, and the Visit spans nest by
    // recursion (depth >= 3). Lookup by id finds a known span.
    assert!(tree.depth() >= 3);
    let first = tree.roots()[0].span;
    assert_eq!(tree.find(first).map(|node| node.span), Some(first));
    assert!(tree.find(lisage_core::observer::SpanId(u64::MAX)).is_none());
}

/// D2-4: folded balances by prefix sums — the balance of any interval
/// answers in O(1) and matches a test-side linear scan; the whole-trace
/// balance totals the events, created nodes, hits and growth.
#[test]
fn test_balances_prefix_sums() {
    let events = saturation_trace(4);
    let index = IndexTrace::build(&events);
    let balances = Balances::build(&events);
    assert_eq!(balances.length(), events.len());

    // Whole-trace balance == linear scan totals.
    let whole = balances.balance(0..events.len());
    let scan_created = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::NodeCreated { .. }))
        .count();
    let scan_hits = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::MemoHit { .. }))
        .count();
    let scan_growth: i64 = events
        .iter()
        .map(|e| match e {
            TraceEvent::Round { growth, .. } => *growth,
            _ => 0,
        })
        .sum();
    assert_eq!(whole.events, events.len());
    assert_eq!(whole.nodes_created, scan_created);
    assert_eq!(whole.hits, scan_hits);
    assert_eq!(whole.growth, scan_growth);

    // Per-round balances (via the index intervals) sum to the whole.
    let mut sum = lisage_liseuse::spans::Balance {
        events: 0,
        nodes_created: 0,
        hits: 0,
        growth: 0,
    };
    let tree = SpanTree::build(&events);
    let intervals = round_intervals(&events, &index, &tree);
    for number in 1..=4u32 {
        let range = intervals.get(&number).cloned().expect("round present");
        let balance = balances.balance(range.clone());
        // Cross-check against a linear scan of the interval (test-side).
        let slice = &events[range.clone()];
        assert_eq!(
            balance.nodes_created,
            slice
                .iter()
                .filter(|e| matches!(e, TraceEvent::NodeCreated { .. }))
                .count()
        );
        assert_eq!(
            balance.hits,
            slice
                .iter()
                .filter(|e| matches!(e, TraceEvent::MemoHit { .. }))
                .count()
        );
        sum.events += balance.events;
        sum.nodes_created += balance.nodes_created;
        sum.hits += balance.hits;
        sum.growth += balance.growth;
    }
    assert_eq!(sum, whole, "the round intervals partition the trace");

    // Inverted or out-of-trace ranges answer empty, never underflow.
    let (high, low) = (5, 2);
    let inverted = balances.balance(high..low);
    assert_eq!(inverted.events, 0);
    assert_eq!(inverted.growth, 0);
    assert_eq!(balances.balance(events.len()..events.len() + 10).events, 0);
}

/// DoD: saturation n=4 presented as N balance lines — one line per
/// round, the last one carrying the convergence marker.
#[test]
fn test_balance_lines_saturation_n4() {
    let events = saturation_trace(4);
    let index = IndexTrace::build(&events);
    let balances = Balances::build(&events);
    let tree = SpanTree::build(&events);
    let intervals = round_intervals(&events, &index, &tree);
    let lines = round_lines(&events, &balances, &intervals);
    assert_eq!(lines.len(), 4, "N balance lines for the n=4 saturation");
    assert_eq!(
        lines.iter().map(|line| line.number).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    // The fixpoint round (zero growth) is the convergence marker.
    assert_eq!(convergence_round(&events), Some(4));
    assert!(lines[3].converged, "the last round is the fixpoint");
    assert!(!lines[0].converged);
    let rendered = render_balance_lines(&lines);
    assert_eq!(rendered.len(), 4);
    assert!(rendered[0].starts_with("round 1: events="));
    assert!(rendered[3].contains("converged"));
    assert!(!rendered[0].contains("converged"));
}

/// DoD: diff round 11 vs 12 operational — on a real 13-round saturation,
/// the diff answers by index + prefix sums and matches the per-round
/// balances; a round diffed with itself is zero; an absent round is
/// `None`.
#[test]
fn test_diff_rounds_11_vs_12() {
    let events = saturation_trace(13);
    let index = IndexTrace::build(&events);
    let balances = Balances::build(&events);
    assert_eq!(convergence_round(&events), Some(13));

    let tree = SpanTree::build(&events);
    let intervals = round_intervals(&events, &index, &tree);
    let delta = diff_rounds(11, 12, &balances, &intervals).expect("rounds 11 and 12 exist");
    let range_11 = intervals.get(&11).unwrap().clone();
    let range_12 = intervals.get(&12).unwrap().clone();
    let expected = balances
        .balance(range_12.clone())
        .delta(&balances.balance(range_11));
    assert_eq!(delta, expected);
    // The growth delta equals the difference of the two Round growths.
    let growth_of = |target: u32| {
        events
            .iter()
            .find_map(|e| match e {
                TraceEvent::Round { number, growth, .. } if *number == target => Some(*growth),
                _ => None,
            })
            .unwrap()
    };
    // Pinned on the deterministic chain: round k grows by k+1 nodes
    // (measured on the trace), so round 12 grows more than round 11.
    assert_eq!(growth_of(11), 12);
    assert_eq!(growth_of(12), 13);
    assert_eq!(delta.growth, growth_of(12) - growth_of(11));
    assert_eq!(delta.growth, 1, "round 12 grows more than round 11");

    // A round diffed with itself is zero; an absent round is None.
    let zero = diff_rounds(11, 11, &balances, &intervals).unwrap();
    assert_eq!(zero.events, 0);
    assert_eq!(zero.growth, 0);
    assert!(diff_rounds(11, 14, &balances, &intervals).is_none());

    // Display renders the signed components.
    assert!(format!("{delta}").contains("growth=+"));
}
