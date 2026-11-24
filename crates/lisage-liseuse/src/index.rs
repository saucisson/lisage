//! Inverse indexes over a trace (D2-3): node → event ranges, round →
//! interval, op → interval. Built in one O(n) pass; `explain`/`goto`
//! queries answer in O(log n + k) by binary search, never by linear scan.

use std::collections::BTreeMap;
use std::ops::Range;

use lisage_core::NodeId;
use lisage_core::observer::TraceEvent;

/// Inverse index of one trace: built once from the event vector, then
/// queried by node / round / op.
pub struct IndexTrace {
    /// Number of indexed events.
    length: usize,
    /// Node → ranges of event positions mentioning it (merged when
    /// contiguous).
    by_node: BTreeMap<NodeId, Vec<Range<usize>>>,
    /// Round number → interval of event positions of the round.
    by_round: BTreeMap<u32, Range<usize>>,
    /// Op index → interval of event positions of the operation (from
    /// `OpStart` to `OpEnd` inclusive).
    by_op: Vec<Range<usize>>,
}

/// Node carried by an event, if any.
fn node_of(event: &TraceEvent) -> Option<NodeId> {
    match event {
        TraceEvent::NodeCreated { node, .. }
        | TraceEvent::NodeShared { node, .. }
        | TraceEvent::Attr { node, .. }
        | TraceEvent::MemoHit { node, .. }
        | TraceEvent::MemoMiss { node, .. }
        | TraceEvent::VisitEntry { node, .. }
        | TraceEvent::VisitExit { node, .. }
        | TraceEvent::MemoryRead { node, .. }
        | TraceEvent::StructureChange { node, .. } => Some(*node),
        _ => None,
    }
}

/// Pushes `position` into the node's range list, merging with the last
/// range when contiguous.
fn push_position(ranges: &mut Vec<Range<usize>>, position: usize) {
    if let Some(last_range) = ranges.last_mut()
        && last_range.end == position
    {
        last_range.end = position + 1;
        return;
    }
    ranges.push(position..position + 1);
}

impl IndexTrace {
    /// Builds the index from the trace events, in one pass (O(n)).
    pub fn build(events: &[TraceEvent]) -> Self {
        let mut by_node: BTreeMap<NodeId, Vec<Range<usize>>> = BTreeMap::new();
        let mut by_round: BTreeMap<u32, Range<usize>> = BTreeMap::new();
        let mut by_op: Vec<Range<usize>> = Vec::new();
        let mut round_start: Option<(u32, usize)> = None;
        let mut op_stack: Vec<usize> = Vec::new();

        for (position, event) in events.iter().enumerate() {
            if let Some(node) = node_of(event) {
                push_position(by_node.entry(node).or_default(), position);
            }
            match event {
                TraceEvent::Round { number, .. } => {
                    // The previous round ends where this one starts.
                    if let Some((prev_number, start)) = round_start.take() {
                        by_round.insert(prev_number, start..position);
                    }
                    round_start = Some((*number, position));
                }
                TraceEvent::OpStart { .. } => {
                    // The op index is assigned in start order.
                    by_op.push(position..position);
                    op_stack.push(by_op.len() - 1);
                }
                TraceEvent::OpEnd { .. } => {
                    if let Some(index) = op_stack.pop() {
                        by_op[index].end = position + 1;
                    }
                }
                _ => {}
            }
        }
        if let Some((number, start)) = round_start {
            by_round.insert(number, start..events.len());
        }
        IndexTrace {
            length: events.len(),
            by_node,
            by_round,
            by_op,
        }
    }

    /// Number of indexed events.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Number of distinct indexed nodes.
    pub fn node_count(&self) -> usize {
        self.by_node.len()
    }

    /// Number of indexed rounds.
    pub fn round_count(&self) -> usize {
        self.by_round.len()
    }

    /// Number of indexed operations.
    pub fn op_count(&self) -> usize {
        self.by_op.len()
    }

    /// `explain(node)`: positions of the events mentioning `node`, by
    /// index lookup — O(log n + k), no linear scan.
    pub fn explain(&self, node: NodeId) -> Vec<usize> {
        self.by_node
            .get(&node)
            .map(|ranges| ranges.iter().flat_map(|range| range.clone()).collect())
            .unwrap_or_default()
    }

    /// Ranges of event positions mentioning `node` (merged form).
    pub fn node_ranges(&self, node: NodeId) -> &[Range<usize>] {
        self.by_node
            .get(&node)
            .map(|ranges| ranges.as_slice())
            .unwrap_or(&[])
    }

    /// Interval of event positions of round `number` — O(log n).
    pub fn round_range(&self, number: u32) -> Option<Range<usize>> {
        self.by_round.get(&number).cloned()
    }

    /// Interval of event positions of the `index`-th operation — O(1).
    pub fn op_range(&self, index: usize) -> Option<Range<usize>> {
        self.by_op.get(index).cloned()
    }

    /// `goto` a round by number: first event position of the round —
    /// O(log n) via the round map.
    pub fn round_position(&self, number: u32) -> Option<usize> {
        self.by_round.get(&number).map(|range| range.start)
    }

    /// `goto` an op by index: first event position of the op — O(1).
    pub fn op_position(&self, index: usize) -> Option<usize> {
        self.by_op.get(index).map(|range| range.start)
    }
}
