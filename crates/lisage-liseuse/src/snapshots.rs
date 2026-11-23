//! Snapshots and O(log n) seek over a trace (D2-2).
//!
//! Extends the step-by-step scrubber with periodic snapshots of the
//! replayed state: `seek(k)` finds the last snapshot at or before `k`
//! by binary search (O(log n)) and replays only the events between that
//! snapshot and `k`. Snapshots are throttled by a budget (minimum number
//! of events between two snapshots) so the memory overhead stays bounded.

use std::collections::BTreeSet;

use lisage_core::observer::TraceEvent;

use crate::step_by_step::StateTrace;

/// Default minimum number of events between two snapshots.
pub const DEFAULT_SNAPSHOT_BUDGET: usize = 1_000;

/// A snapshot of the replayed state at a given position.
#[derive(Clone, Debug)]
struct Snapshot {
    /// Position in the trace (number of events replayed).
    position: usize,
    /// Replayed state at that position.
    state: StateTrace,
}

/// Scrubber with snapshots: seek answers in O(log n + replay tail).
pub struct SnapshotScrubber {
    /// The underlying event trace.
    events: Vec<TraceEvent>,
    /// Snapshots at frontier positions, sorted by position.
    snapshots: Vec<Snapshot>,
    /// Minimum events between two snapshots (budget).
    budget: usize,
    /// Current position.
    position: usize,
}

impl SnapshotScrubber {
    /// New snapshot scrubber on the given trace, with the default budget.
    pub fn new(events: Vec<TraceEvent>) -> Self {
        Self::with_budget(events, DEFAULT_SNAPSHOT_BUDGET)
    }

    /// New snapshot scrubber with a custom snapshot budget.
    pub fn with_budget(events: Vec<TraceEvent>, budget: usize) -> Self {
        let mut scrubber = SnapshotScrubber {
            events,
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
        let mut state = StateTrace::default();
        let mut last_snapshot_position = 0usize;
        for (index, event) in self.events.iter().enumerate() {
            apply_event(&mut state, event);
            if index - last_snapshot_position >= self.budget {
                self.snapshots.push(Snapshot {
                    position: index + 1,
                    state: state.clone(),
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
                    state: StateTrace::default(),
                },
            );
        }
    }

    /// Number of events in the trace.
    pub fn length(&self) -> usize {
        self.events.len()
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
    pub fn seek(&mut self, position: usize) -> &StateTrace {
        let target = position.min(self.events.len());
        self.position = target;
        // Binary search: last snapshot with position <= target.
        let snapshot_index = self
            .snapshots
            .partition_point(|s| s.position <= target)
            .saturating_sub(1);
        let snapshot = &self.snapshots[snapshot_index];
        let mut state = snapshot.state.clone();
        for event in &self.events[snapshot.position..target] {
            apply_event(&mut state, event);
        }
        // Cache the computed state at the current position for repeated calls.
        // (Stored as the last snapshot if it is at the target, else recomputed.)
        state_cache_set(&mut self.snapshots, target, state);
        &self.snapshots.last().expect("at least one snapshot").state
    }

    /// State at the current position (replays from the nearest snapshot).
    pub fn state(&self) -> StateTrace {
        let target = self.position;
        let snapshot_index = self
            .snapshots
            .partition_point(|s| s.position <= target)
            .saturating_sub(1);
        let snapshot = &self.snapshots[snapshot_index];
        let mut state = snapshot.state.clone();
        for event in &self.events[snapshot.position..target] {
            apply_event(&mut state, event);
        }
        state
    }
}

/// Applies one event to the replayed state (same logic as Scrubber::state).
fn apply_event(state: &mut StateTrace, event: &TraceEvent) {
    match event {
        TraceEvent::NodeCreated { node, .. } | TraceEvent::NodeShared { node, .. } => {
            state.nodes.insert(*node);
        }
        TraceEvent::Attr {
            channel,
            node,
            value,
            ..
        } => {
            state.attributes.push((*channel, *node, value.clone()));
        }
        TraceEvent::MemoHit { .. } => state.memo_hits += 1,
        TraceEvent::MemoMiss { .. } => state.memo_misses += 1,
        TraceEvent::Round { number, .. } => state.rounds = state.rounds.max(*number),
        _ => {}
    }
}

/// Stores the computed state as a snapshot at `position` if not present.
fn state_cache_set(snapshots: &mut Vec<Snapshot>, position: usize, state: StateTrace) {
    if let Some(last) = snapshots.last_mut()
        && last.position == position
    {
        last.state = state;
        return;
    }
    snapshots.push(Snapshot { position, state });
}

/// Set of live nodes at a position (convenience for tests).
pub fn live_nodes_at(
    scrubber: &SnapshotScrubber,
    position: usize,
) -> BTreeSet<lisage_core::NodeId> {
    let mut s = SnapshotScrubber::new(scrubber.events.clone());
    s.seek(position);
    s.state().nodes.clone()
}

/// Re-export Scrubber for migration callers.
pub use crate::step_by_step::Scrubber as LegacyScrubber;
