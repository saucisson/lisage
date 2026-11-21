//! Snapshots and O(log n) seek over a trace (D2-2).
//!
//! Extends the step-by-step scrubber with periodic snapshots of the
//! replayed state: `seek(k)` finds the last snapshot at or before `k`
//! by binary search (O(log n)) and replays only the events between that
//! snapshot and `k`. Snapshots are throttled by a budget (minimum number
//! of events between two snapshots) so the memory overhead stays bounded.

use std::collections::BTreeSet;

use lisage_core::observer::TraceEvent;

use crate::step_by_step::EtatTrace;

/// Default minimum number of events between two snapshots.
pub const DEFAULT_SNAPSHOT_BUDGET: usize = 1_000;

/// A snapshot of the replayed state at a given position.
#[derive(Clone, Debug)]
struct Snapshot {
    /// Position in the trace (number of events replayed).
    position: usize,
    /// Replayed state at that position.
    etat: EtatTrace,
}

/// Scrubber with snapshots: seek answers in O(log n + replay tail).
pub struct SnapshotScrubber {
    /// The underlying event trace.
    evenements: Vec<TraceEvent>,
    /// Snapshots at frontier positions, sorted by position.
    snapshots: Vec<Snapshot>,
    /// Minimum events between two snapshots (budget).
    budget: usize,
    /// Current position.
    position: usize,
}

impl SnapshotScrubber {
    /// New snapshot scrubber on the given trace, with the default budget.
    pub fn new(evenements: Vec<TraceEvent>) -> Self {
        Self::with_budget(evenements, DEFAULT_SNAPSHOT_BUDGET)
    }

    /// New snapshot scrubber with a custom snapshot budget.
    pub fn with_budget(evenements: Vec<TraceEvent>, budget: usize) -> Self {
        let mut scrubber = SnapshotScrubber {
            evenements,
            snapshots: Vec::new(),
            budget: budget.max(1),
            position: 0,
        };
        scrubber.rebuild_snapshots();
        scrubber
    }

    /// Rebuilds all snapshots from scratch (one pass over the trace).
    fn rebuild_snapshots(&mut self) {
        self.snapshots.clear();
        let mut etat = EtatTrace::default();
        let mut last_snapshot_position = 0usize;
        for (index, event) in self.evenements.iter().enumerate() {
            apply_event(&mut etat, event);
            if index - last_snapshot_position >= self.budget {
                self.snapshots.push(Snapshot {
                    position: index + 1,
                    etat: etat.clone(),
                });
                last_snapshot_position = index + 1;
            }
        }
        // Always keep a snapshot at position 0 for the binary search base.
        if self.snapshots.first().is_none_or(|s| s.position != 0) {
            self.snapshots.insert(
                0,
                Snapshot {
                    position: 0,
                    etat: EtatTrace::default(),
                },
            );
        }
    }

    /// Number of events in the trace.
    pub fn longueur(&self) -> usize {
        self.evenements.len()
    }

    /// Number of snapshots.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Current position.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Moves to `position` (clamped) and returns the replayed state there.
    /// Seek = binary search for the last snapshot <= position, then replay
    /// the tail. O(log n + tail).
    pub fn seek(&mut self, position: usize) -> &EtatTrace {
        let target = position.min(self.evenements.len());
        self.position = target;
        // Binary search: last snapshot with position <= target.
        let snapshot_index = self
            .snapshots
            .partition_point(|s| s.position <= target)
            .saturating_sub(1);
        let snapshot = &self.snapshots[snapshot_index];
        let mut etat = snapshot.etat.clone();
        for event in &self.evenements[snapshot.position..target] {
            apply_event(&mut etat, event);
        }
        // Cache the computed state at the current position for repeated calls.
        // (Stored as the last snapshot if it is at the target, else recomputed.)
        etat_cache_set(&mut self.snapshots, target, etat);
        &self.snapshots.last().expect("at least one snapshot").etat
    }

    /// State at the current position (replays from the nearest snapshot).
    pub fn etat(&self) -> EtatTrace {
        let target = self.position;
        let snapshot_index = self
            .snapshots
            .partition_point(|s| s.position <= target)
            .saturating_sub(1);
        let snapshot = &self.snapshots[snapshot_index];
        let mut etat = snapshot.etat.clone();
        for event in &self.evenements[snapshot.position..target] {
            apply_event(&mut etat, event);
        }
        etat
    }
}

/// Applies one event to the replayed state (same logic as Scrubber::etat).
fn apply_event(etat: &mut EtatTrace, event: &TraceEvent) {
    match event {
        TraceEvent::NodeCreated { node, .. } | TraceEvent::NodeShared { node, .. } => {
            etat.nodes.insert(*node);
        }
        TraceEvent::Attr {
            channel,
            node,
            value,
            ..
        } => {
            etat.attributes.push((*channel, *node, value.clone()));
        }
        TraceEvent::MemoHit { .. } => etat.memo_hits += 1,
        TraceEvent::MemoMiss { .. } => etat.memo_misses += 1,
        TraceEvent::Round { numero, .. } => etat.rounds = etat.rounds.max(*numero),
        _ => {}
    }
}

/// Stores the computed state as a snapshot at `position` if not present.
fn etat_cache_set(snapshots: &mut Vec<Snapshot>, position: usize, etat: EtatTrace) {
    if let Some(last) = snapshots.last_mut()
        && last.position == position
    {
        last.etat = etat;
        return;
    }
    snapshots.push(Snapshot { position, etat });
}

/// Set of live nodes at a position (convenience for tests).
pub fn live_nodes_at(
    scrubber: &SnapshotScrubber,
    position: usize,
) -> BTreeSet<lisage_core::NodeId> {
    let mut s = SnapshotScrubber::new(scrubber.evenements.clone());
    s.seek(position);
    s.etat().nodes.clone()
}

/// Re-export Scrubber for migration callers.
pub use crate::step_by_step::Scrubber as LegacyScrubber;
