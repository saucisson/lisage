//! D2-4: span tree and folded balances over a trace.
//!
//! - [`SpanTree`] reconstructs the nested span intervals
//!   (`SpanOpen`/`SpanClose`) in one O(n) pass.
//! - [`Balances`] keeps prefix sums over the event vector — events,
//!   nodes created, memo hits, pool growth — so the balance of any
//!   interval answers in O(1), never by linear scan.
//! - [`round_lines`] / [`render_balance_lines`] present a saturation as
//!   one balance line per round, with the convergence marker (a round
//!   that changed nothing, `after == before` = fixpoint).
//! - [`diff_rounds`] compares two rounds (DoD: diff round 11 vs 12) by
//!   round-interval lookup + prefix sums, in O(1) after one O(n) pass.
//! - [`fire_lines`] / [`render_fire_lines`] present the in-place kernel
//!   (ADR-0109) as one balance line per local fire, in firing order,
//!   with the fixpoint marker on the sterile fire (the fire that
//!   changed nothing closes the local fixpoint of the visited node).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops::Range;

use lisage_core::NodeId;
use lisage_core::observer::{SpanId, SpanKind, TraceEvent};

use crate::index::IndexTrace;

/// One node of the span tree.
#[derive(Clone, Debug)]
pub struct SpanNode {
    /// Identifier of the span.
    pub span: SpanId,
    /// Kind of the span.
    pub kind: SpanKind,
    /// Lane number (0 for sequential execution).
    pub lane: u32,
    /// Event positions of the span: `open..=close`.
    pub range: Range<usize>,
    /// Nested spans, in opening order.
    pub children: Vec<SpanNode>,
}

/// Span tree of a trace: the nested intervals reconstructed from the
/// `SpanOpen`/`SpanClose` events in one pass.
#[derive(Clone, Debug)]
pub struct SpanTree {
    roots: Vec<SpanNode>,
    by_id: HashMap<SpanId, Range<usize>>,
}

impl SpanTree {
    /// Builds the tree in one O(n) pass with a stack. Unclosed spans at
    /// the end of the trace are closed at the last position; a
    /// `SpanClose` that does not match the innermost open span is
    /// ignored (the span stays open).
    pub fn build(events: &[TraceEvent]) -> Self {
        let mut stack: Vec<SpanNode> = Vec::new();
        let mut roots: Vec<SpanNode> = Vec::new();
        let mut by_id: HashMap<SpanId, Range<usize>> = HashMap::new();
        let close = |stack: &mut Vec<SpanNode>,
                     roots: &mut Vec<SpanNode>,
                     by_id: &mut HashMap<SpanId, Range<usize>>,
                     mut node: SpanNode,
                     end: usize| {
            node.range.end = end;
            by_id.insert(node.span, node.range.clone());
            match stack.last_mut() {
                Some(parent) => parent.children.push(node),
                None => roots.push(node),
            }
        };
        for (position, event) in events.iter().enumerate() {
            match event {
                TraceEvent::SpanOpen { span, kind, lane } => stack.push(SpanNode {
                    span: *span,
                    kind: *kind,
                    lane: *lane,
                    range: position..position,
                    children: Vec::new(),
                }),
                TraceEvent::SpanClose { span } => {
                    // Only the innermost open span may be closed; a
                    // mismatched `SpanClose` is ignored and the span
                    // stays open.
                    if let Some(node) = stack.pop_if(|node| node.span == *span) {
                        close(&mut stack, &mut roots, &mut by_id, node, position + 1);
                    }
                }
                _ => {}
            }
        }
        // Defensive: close the spans still open at the end of the trace.
        while let Some(node) = stack.pop() {
            close(&mut stack, &mut roots, &mut by_id, node, events.len());
        }
        SpanTree { roots, by_id }
    }

    /// Top-level spans, in opening order.
    pub fn roots(&self) -> &[SpanNode] {
        &self.roots
    }

    /// Event interval of a span by identifier — O(1) hash lookup.
    pub fn span_range(&self, span: SpanId) -> Option<Range<usize>> {
        self.by_id.get(&span).cloned()
    }

    /// Finds a span node by identifier — DFS over the tree, O(n).
    pub fn find(&self, span: SpanId) -> Option<&SpanNode> {
        fn search(node: &SpanNode, span: SpanId) -> Option<&SpanNode> {
            if node.span == span {
                return Some(node);
            }
            node.children.iter().find_map(|child| search(child, span))
        }
        self.roots.iter().find_map(|root| search(root, span))
    }

    /// Depth of the tree (a single root span with no child has depth 1).
    pub fn depth(&self) -> usize {
        fn depth_of(node: &SpanNode) -> usize {
            1 + node.children.iter().map(depth_of).max().unwrap_or(0)
        }
        self.roots.iter().map(depth_of).max().unwrap_or(0)
    }
}

/// Folded balance of an event interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Balance {
    /// Number of events in the interval.
    pub events: usize,
    /// `NodeCreated` events in the interval.
    pub nodes_created: usize,
    /// `MemoHit` events in the interval.
    pub hits: usize,
    /// Sum of the `Round` growths in the interval.
    pub growth: i64,
}

impl Balance {
    /// `self - other`, component-wise (signed).
    pub fn delta(&self, other: &Balance) -> BalanceDelta {
        BalanceDelta {
            events: self.events as i64 - other.events as i64,
            nodes_created: self.nodes_created as i64 - other.nodes_created as i64,
            hits: self.hits as i64 - other.hits as i64,
            growth: self.growth - other.growth,
        }
    }
}

/// Signed difference between two balances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalanceDelta {
    /// Delta of the event counts.
    pub events: i64,
    /// Delta of the created nodes.
    pub nodes_created: i64,
    /// Delta of the memo hits.
    pub hits: i64,
    /// Delta of the pool growth.
    pub growth: i64,
}

impl fmt::Display for BalanceDelta {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "events={:+} nodes_created={:+} hits={:+} growth={:+}",
            self.events, self.nodes_created, self.hits, self.growth
        )
    }
}

/// Prefix sums over the trace: the balance of any interval `[a, b)`
/// answers in O(1) as `prefix[b] - prefix[a]`.
pub struct Balances {
    length: usize,
    nodes_created: Vec<usize>,
    hits: Vec<usize>,
    growth: Vec<i64>,
}

impl Balances {
    /// Builds the prefix sums in one O(n) pass. `prefix[i]` counts the
    /// events `events[..i]`.
    pub fn build(events: &[TraceEvent]) -> Self {
        let mut nodes_created: Vec<usize> = Vec::with_capacity(events.len() + 1);
        let mut hits: Vec<usize> = Vec::with_capacity(events.len() + 1);
        let mut growth: Vec<i64> = Vec::with_capacity(events.len() + 1);
        nodes_created.push(0);
        hits.push(0);
        growth.push(0);
        for event in events {
            let (created, hit, grown) = match event {
                TraceEvent::NodeCreated { .. } => (1, 0, 0),
                TraceEvent::MemoHit { .. } => (0, 1, 0),
                TraceEvent::Round { growth, .. } => (0, 0, *growth),
                _ => (0, 0, 0),
            };
            nodes_created.push(nodes_created.last().expect("prefix non-empty") + created);
            hits.push(hits.last().expect("prefix non-empty") + hit);
            growth.push(growth.last().expect("prefix non-empty") + grown);
        }
        Balances {
            length: events.len(),
            nodes_created,
            hits,
            growth,
        }
    }

    /// Number of indexed events.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Folded balance of the interval `range` — O(1). The bounds are
    /// clamped to the trace length; an inverted range answers an empty
    /// balance instead of underflowing.
    pub fn balance(&self, range: Range<usize>) -> Balance {
        let start = range.start.min(self.length);
        let end = range.end.min(self.length).max(start);
        Balance {
            events: end - start,
            nodes_created: self.nodes_created[end] - self.nodes_created[start],
            hits: self.hits[end] - self.hits[start],
            growth: self.growth[end] - self.growth[start],
        }
    }
}

/// One balance line of a saturation: a round with its folded balance
/// and the convergence marker.
#[derive(Clone, Copy, Debug)]
pub struct RoundLine {
    /// Round number (1-based).
    pub number: u32,
    /// Folded balance of the round's event interval.
    pub balance: Balance,
    /// Convergence marker: the round changed nothing (`after == before`,
    /// fixpoint reached).
    pub converged: bool,
}

/// Round number → event interval, built in one O(n) pass: the range of
/// the round's `Round` span in the span tree (the span covers the whole
/// round, `SpanOpen..=SpanClose`); falls back to the index's round
/// interval when the span is absent. Lookups in the returned map are
/// O(1).
pub fn round_intervals(
    events: &[TraceEvent],
    index: &IndexTrace,
    tree: &SpanTree,
) -> BTreeMap<u32, Range<usize>> {
    let mut intervals: BTreeMap<u32, Range<usize>> = BTreeMap::new();
    for event in events {
        if let TraceEvent::Round { number, span, .. } = event {
            let range = span
                .and_then(|span| tree.span_range(span))
                .or_else(|| index.round_range(*number))
                .unwrap_or(0..0);
            intervals.insert(*number, range);
        }
    }
    intervals
}

/// First round that changed nothing (`after == before`) — the
/// convergence marker of the saturation.
pub fn convergence_round(events: &[TraceEvent]) -> Option<u32> {
    events.iter().find_map(|event| match event {
        TraceEvent::Round {
            number,
            before,
            after,
            ..
        } if before == after => Some(*number),
        _ => None,
    })
}

/// Balance lines of every round of the trace, in round order. The round
/// intervals come from the prebuilt map (O(1) each), the balances from
/// the prefix sums (O(1) each).
pub fn round_lines(
    events: &[TraceEvent],
    balances: &Balances,
    intervals: &BTreeMap<u32, Range<usize>>,
) -> Vec<RoundLine> {
    let mut rounds: Vec<(u32, bool)> = events
        .iter()
        .filter_map(|event| match event {
            TraceEvent::Round {
                number,
                before,
                after,
                ..
            } => Some((*number, before == after)),
            _ => None,
        })
        .collect();
    rounds.sort_unstable();
    rounds.dedup();
    rounds
        .iter()
        .map(|&(number, converged)| {
            let range = intervals.get(&number).cloned().unwrap_or(0..0);
            RoundLine {
                number,
                balance: balances.balance(range),
                converged,
            }
        })
        .collect()
}

/// Renders the balance lines: one line per round, with the convergence
/// marker on the fixpoint round.
pub fn render_balance_lines(lines: &[RoundLine]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let marker = if line.converged { " converged" } else { "" };
            format!(
                "round {}: events={} nodes_created={} hits={} growth={:+}{marker}",
                line.number,
                line.balance.events,
                line.balance.nodes_created,
                line.balance.hits,
                line.balance.growth
            )
        })
        .collect()
}

/// One balance line of the firing-order view (ADR-0109): a local fire
/// of the in-place kernel with its folded balance and its marker — the
/// sterile fire (`merged == before`) closes the local fixpoint of the
/// visited node.
#[derive(Clone, Copy, Debug)]
pub struct FireLine {
    /// Fire number, 1-based (the firing order of the kernel).
    pub number: u32,
    /// Level at which the fire happened.
    pub level: u32,
    /// Node before the fire (the visited node).
    pub before: NodeId,
    /// Node after the fire (`before ⊕ image`).
    pub merged: NodeId,
    /// Folded balance of the fire's event interval.
    pub balance: Balance,
    /// `true` when the fire changed nothing (`merged == before`): the
    /// visited node reached its local fixpoint.
    pub sterile: bool,
}

/// Fire number → event interval, built in one O(n) pass: the range of
/// the fire's `Fire` span in the span tree (the span covers the whole
/// fire, `SpanOpen..=SpanClose`); falls back to the position of the
/// `LocalFire` event itself when the span is absent. Lookups in the
/// returned map are O(1).
pub fn fire_intervals(events: &[TraceEvent], tree: &SpanTree) -> BTreeMap<u32, Range<usize>> {
    let mut intervals: BTreeMap<u32, Range<usize>> = BTreeMap::new();
    for (position, event) in events.iter().enumerate() {
        if let TraceEvent::LocalFire { number, span, .. } = event {
            intervals.entry(*number).or_insert_with(|| {
                span.and_then(|span| tree.span_range(span))
                    .unwrap_or(position..position + 1)
            });
        }
    }
    intervals
}

/// Fire lines of the trace, in firing order (ADR-0109): the local
/// fires of the in-place kernel, with their folded balance — same
/// shape as [`round_lines`], one line per fire instead of one line
/// per round.
pub fn fire_lines(
    events: &[TraceEvent],
    balances: &Balances,
    intervals: &BTreeMap<u32, Range<usize>>,
) -> Vec<FireLine> {
    let mut fires: Vec<(u32, u32, NodeId, NodeId)> = events
        .iter()
        .filter_map(|event| match event {
            TraceEvent::LocalFire {
                number,
                level,
                before,
                merged,
                ..
            } => Some((*number, *level, *before, *merged)),
            _ => None,
        })
        .collect();
    fires.sort_unstable();
    fires.dedup();
    fires
        .iter()
        .map(|&(number, level, before, merged)| {
            let range = intervals.get(&number).cloned().unwrap_or(0..0);
            FireLine {
                number,
                level,
                before,
                merged,
                balance: balances.balance(range),
                sterile: before == merged,
            }
        })
        .collect()
}

/// Renders the fire lines: one line per local fire, in firing order,
/// with the fixpoint marker on the sterile fire.
pub fn render_fire_lines(lines: &[FireLine]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let marker = if line.sterile { " fixpoint" } else { "" };
            format!(
                "fire {} at level {}: before {} -> merged {} events={} nodes_created={} hits={} growth={:+}{marker}",
                line.number,
                line.level,
                line.before.0,
                line.merged.0,
                line.balance.events,
                line.balance.nodes_created,
                line.balance.hits,
                line.balance.growth
            )
        })
        .collect()
}

/// Diff of two rounds by number: `balance(second) - balance(first)`
/// over their span intervals — O(1) by map lookup, O(1) by prefix sums.
/// `None` when either round is absent from the trace.
pub fn diff_rounds(
    first: u32,
    second: u32,
    balances: &Balances,
    intervals: &BTreeMap<u32, Range<usize>>,
) -> Option<BalanceDelta> {
    let range_first = intervals.get(&first)?;
    let range_second = intervals.get(&second)?;
    Some(
        balances
            .balance(range_second.clone())
            .delta(&balances.balance(range_first.clone())),
    )
}
