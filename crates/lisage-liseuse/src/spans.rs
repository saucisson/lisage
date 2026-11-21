//! D2-4: span tree and folded balances over a trace.
//!
//! - [`SpanTree`] reconstructs the nested span intervals
//!   (`SpanOpen`/`SpanClose`) in one O(n) pass.
//! - [`Balances`] keeps prefix sums over the event vector — events,
//!   nodes created, memo hits, pool growth — so the balance of any
//!   interval answers in O(1), never by linear scan.
//! - [`round_lines`] / [`render_balance_lines`] present a saturation as
//!   one balance line per round, with the convergence marker (a round
//!   of zero growth = fixpoint).
//! - [`diff_rounds`] compares two rounds (DoD: diff round 11 vs 12) by
//!   index lookup + prefix sums, in O(log n).

use std::fmt;
use std::ops::Range;

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
}

impl SpanTree {
    /// Builds the tree in one O(n) pass with a stack. Unclosed spans at
    /// the end of the trace are closed at the last position; a
    /// `SpanClose` without matching `SpanOpen` is ignored.
    pub fn build(events: &[TraceEvent]) -> Self {
        let mut stack: Vec<SpanNode> = Vec::new();
        let mut roots: Vec<SpanNode> = Vec::new();
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
                    if let Some(mut node) = stack.pop()
                        && node.span == *span
                    {
                        node.range.end = position + 1;
                        match stack.last_mut() {
                            Some(parent) => parent.children.push(node),
                            None => roots.push(node),
                        }
                    }
                }
                _ => {}
            }
        }
        // Defensive: close the spans still open at the end of the trace.
        while let Some(mut node) = stack.pop() {
            node.range.end = events.len();
            match stack.last_mut() {
                Some(parent) => parent.children.push(node),
                None => roots.push(node),
            }
        }
        SpanTree { roots }
    }

    /// Top-level spans, in opening order.
    pub fn roots(&self) -> &[SpanNode] {
        &self.roots
    }

    /// Finds a span by identifier — DFS over the tree.
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
                TraceEvent::Round { croissance, .. } => (0, 0, *croissance),
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

    /// Folded balance of the interval `range` — O(1).
    pub fn balance(&self, range: Range<usize>) -> Balance {
        let start = range.start.min(self.length);
        let end = range.end.min(self.length);
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
    /// Convergence marker: the round changed nothing (`apres == avant`,
    /// fixpoint reached).
    pub converged: bool,
}

/// Event interval of one round: the range of its `Round` span in the
/// span tree (the span covers the whole round, `SpanOpen..=SpanClose`);
/// falls back to the index's round interval when the span is absent.
pub fn round_interval(
    events: &[TraceEvent],
    index: &IndexTrace,
    tree: &SpanTree,
    number: u32,
) -> Option<Range<usize>> {
    let span = events.iter().find_map(|event| match event {
        TraceEvent::Round { numero, span, .. } if *numero == number => *span,
        _ => None,
    });
    if let Some(span) = span
        && let Some(node) = tree.find(span)
    {
        return Some(node.range.clone());
    }
    index.round_range(number)
}

/// First round that changed nothing (`apres == avant`) — the
/// convergence marker of the saturation.
pub fn convergence_round(events: &[TraceEvent]) -> Option<u32> {
    events.iter().find_map(|event| match event {
        TraceEvent::Round {
            numero,
            avant,
            apres,
            ..
        } if avant == apres => Some(*numero),
        _ => None,
    })
}

/// Balance lines of every round of the trace, in round order. The round
/// intervals come from the span tree (O(depth) each), the balances from
/// the prefix sums (O(1) each).
pub fn round_lines(
    events: &[TraceEvent],
    index: &IndexTrace,
    balances: &Balances,
    tree: &SpanTree,
) -> Vec<RoundLine> {
    let mut rounds: Vec<(u32, bool)> = events
        .iter()
        .filter_map(|event| match event {
            TraceEvent::Round {
                numero,
                avant,
                apres,
                ..
            } => Some((*numero, avant == apres)),
            _ => None,
        })
        .collect();
    rounds.sort_unstable();
    rounds.dedup();
    rounds
        .iter()
        .map(|&(number, converged)| {
            let range = round_interval(events, index, tree, number).unwrap_or(0..0);
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

/// Diff of two rounds by number: `balance(second) - balance(first)`
/// over their span intervals — O(depth) by span-tree lookup, O(1) by
/// prefix sums. `None` when either round is absent from the trace.
pub fn diff_rounds(
    first: u32,
    second: u32,
    events: &[TraceEvent],
    index: &IndexTrace,
    balances: &Balances,
    tree: &SpanTree,
) -> Option<BalanceDelta> {
    let range_first = round_interval(events, index, tree, first)?;
    let range_second = round_interval(events, index, tree, second)?;
    Some(
        balances
            .balance(range_second)
            .delta(&balances.balance(range_first)),
    )
}
