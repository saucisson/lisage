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
    longueur: usize,
    /// Node → ranges of event positions mentioning it (merged when
    /// contiguous).
    par_noeud: BTreeMap<NodeId, Vec<Range<usize>>>,
    /// Round number → interval of event positions of the round.
    par_round: BTreeMap<u32, Range<usize>>,
    /// Op index → interval of event positions of the operation (from
    /// `OpStart` to `OpEnd` inclusive).
    par_op: Vec<Range<usize>>,
}

/// Node carried by an event, if any.
fn noeud_de(evenement: &TraceEvent) -> Option<NodeId> {
    match evenement {
        TraceEvent::NodeCreated { node, .. }
        | TraceEvent::NodeShared { node, .. }
        | TraceEvent::Attr { node, .. }
        | TraceEvent::MemoHit { node, .. }
        | TraceEvent::MemoMiss { node, .. }
        | TraceEvent::VisiteEntree { node, .. }
        | TraceEvent::VisiteSortie { node, .. }
        | TraceEvent::StructureChange { node, .. } => Some(*node),
        _ => None,
    }
}

/// Pushes `position` into the node's range list, merging with the last
/// range when contiguous.
fn pousser_position(ranges: &mut Vec<Range<usize>>, position: usize) {
    if let Some(dernier) = ranges.last_mut()
        && dernier.end == position
    {
        dernier.end = position + 1;
        return;
    }
    ranges.push(position..position + 1);
}

impl IndexTrace {
    /// Builds the index from the trace events, in one pass (O(n)).
    pub fn construire(evenements: &[TraceEvent]) -> Self {
        let mut par_noeud: BTreeMap<NodeId, Vec<Range<usize>>> = BTreeMap::new();
        let mut par_round: BTreeMap<u32, Range<usize>> = BTreeMap::new();
        let mut par_op: Vec<Range<usize>> = Vec::new();
        let mut debut_round: Option<(u32, usize)> = None;
        let mut pile_ops: Vec<usize> = Vec::new();

        for (position, evenement) in evenements.iter().enumerate() {
            if let Some(node) = noeud_de(evenement) {
                pousser_position(par_noeud.entry(node).or_default(), position);
            }
            match evenement {
                TraceEvent::Round { numero, .. } => {
                    // The previous round ends where this one starts.
                    if let Some((numero_avant, debut)) = debut_round.take() {
                        par_round.insert(numero_avant, debut..position);
                    }
                    debut_round = Some((*numero, position));
                }
                TraceEvent::OpStart { .. } => pile_ops.push(position),
                TraceEvent::OpEnd { .. } => {
                    if let Some(debut) = pile_ops.pop() {
                        par_op.push(debut..position + 1);
                    }
                }
                _ => {}
            }
        }
        if let Some((numero, debut)) = debut_round {
            par_round.insert(numero, debut..evenements.len());
        }
        IndexTrace {
            longueur: evenements.len(),
            par_noeud,
            par_round,
            par_op,
        }
    }

    /// Number of indexed events.
    pub fn longueur(&self) -> usize {
        self.longueur
    }

    /// `explain(node)`: positions of the events mentioning `node`, by
    /// index lookup — O(log n + k), no linear scan.
    pub fn explain(&self, noeud: NodeId) -> Vec<usize> {
        self.par_noeud
            .get(&noeud)
            .map(|ranges| ranges.iter().flat_map(|plage| plage.clone()).collect())
            .unwrap_or_default()
    }

    /// Ranges of event positions mentioning `node` (merged form).
    pub fn plages_noeud(&self, noeud: NodeId) -> &[Range<usize>] {
        self.par_noeud
            .get(&noeud)
            .map(|ranges| ranges.as_slice())
            .unwrap_or(&[])
    }

    /// Interval of event positions of round `numero` — O(log n).
    pub fn plage_round(&self, numero: u32) -> Option<Range<usize>> {
        self.par_round.get(&numero).cloned()
    }

    /// Interval of event positions of the `index`-th operation — O(1).
    pub fn plage_op(&self, index: usize) -> Option<Range<usize>> {
        self.par_op.get(index).cloned()
    }

    /// `goto` a round by number: first event position of the round —
    /// O(log n) via the round map.
    pub fn position_round(&self, numero: u32) -> Option<usize> {
        self.par_round.get(&numero).map(|plage| plage.start)
    }

    /// `goto` an op by index: first event position of the op — O(1).
    pub fn position_op(&self, index: usize) -> Option<usize> {
        self.par_op.get(index).map(|plage| plage.start)
    }
}
