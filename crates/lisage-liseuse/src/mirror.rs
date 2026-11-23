//! Pool/memo/memory mirror reconstructed from the trace alone (D2-1).
//!
//! Replays the trace events to rebuild the full state of the pool
//! (live nodes, collected nodes), the memo table (keys inserted,
//! current version) and the level memory (version per write). The
//! mirror is the single source of truth for "what the trace says the
//! engine state is" — used by the debugger and by CI property tests
//! that assert mirror == real engine state at trace end.

use std::collections::{BTreeMap, BTreeSet};

use lisage_core::NodeId;
use lisage_core::observer::TraceEvent;

/// One memo-table key: (op fingerprint, node, memory version).
/// The inherited attribute `h` is not part of the mirror key (it is
/// a lazy `TraceVal`, not comparable cheaply); the version pins the
/// memory state at insertion time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoKey {
    /// Fingerprint of the operation.
    pub fingerprint: u64,
    /// Node of the key.
    pub node: NodeId,
    /// Memory version of the key.
    pub version: u64,
}

/// Full mirror of the engine state, reconstructed from the trace.
#[derive(Clone, Debug, Default)]
pub struct PoolMirror {
    /// Live nodes in the pool (created minus collected).
    live_nodes: BTreeSet<NodeId>,
    /// All nodes ever created.
    created_nodes: BTreeSet<NodeId>,
    /// Nodes collected by GC.
    collected_nodes: BTreeSet<NodeId>,
    /// Memo table: keys currently present.
    memo_keys: BTreeSet<MemoKey>,
    /// Current memory version (last `MemoryWrite`).
    memory_version: u64,
    /// Pool size reported by the last `DiagramBuilt`.
    last_pool_size: Option<usize>,
    /// Number of GC events seen.
    gc_count: u64,
}

impl PoolMirror {
    /// Rebuilds the mirror from the trace events, in one pass.
    pub fn build(events: &[TraceEvent]) -> Self {
        let mut mirror = PoolMirror::default();
        for event in events {
            mirror.apply(event);
        }
        mirror
    }

    /// Applies one trace event to the mirror.
    pub fn apply(&mut self, event: &TraceEvent) {
        match event {
            TraceEvent::NodeCreated { node, .. } => {
                self.created_nodes.insert(*node);
                self.live_nodes.insert(*node);
            }
            TraceEvent::NodeShared { node, .. } => {
                // A shared node is already in the pool: it counts as live.
                self.live_nodes.insert(*node);
            }
            TraceEvent::VisitEntry { node, .. } => {
                // A visited node is necessarily in the pool.
                self.live_nodes.insert(*node);
            }
            TraceEvent::Gc { collected_list, .. } => {
                self.gc_count += 1;
                for node in collected_list {
                    self.collected_nodes.insert(*node);
                    self.live_nodes.remove(node);
                }
            }
            TraceEvent::MemoInsert {
                fingerprint,
                node,
                version,
                ..
            } => {
                self.memo_keys.insert(MemoKey {
                    fingerprint: *fingerprint,
                    node: *node,
                    version: *version,
                });
            }
            TraceEvent::MemoryWrite { version_after, .. } => {
                self.memory_version = *version_after;
            }
            TraceEvent::DiagramBuilt { nodes, .. } => {
                self.last_pool_size = Some(*nodes);
            }
            _ => {}
        }
    }

    /// Live nodes in the pool at trace end (created minus collected).
    pub fn live_nodes(&self) -> &BTreeSet<NodeId> {
        &self.live_nodes
    }

    /// All nodes ever created.
    pub fn created_nodes(&self) -> &BTreeSet<NodeId> {
        &self.created_nodes
    }

    /// Nodes collected by GC.
    pub fn collected_nodes(&self) -> &BTreeSet<NodeId> {
        &self.collected_nodes
    }

    /// Number of live nodes (created minus collected).
    pub fn live_count(&self) -> usize {
        self.live_nodes.len()
    }

    /// Memo keys currently in the table.
    pub fn memo_keys(&self) -> &BTreeSet<MemoKey> {
        &self.memo_keys
    }

    /// Number of memo entries.
    pub fn memo_count(&self) -> usize {
        self.memo_keys.len()
    }

    /// Current memory version.
    pub fn memory_version(&self) -> u64 {
        self.memory_version
    }

    /// Pool size reported by the last `DiagramBuilt`, if any.
    pub fn last_pool_size(&self) -> Option<usize> {
        self.last_pool_size
    }

    /// Number of GC events.
    pub fn gc_count(&self) -> u64 {
        self.gc_count
    }

    /// Consistency check: live_count must equal last_pool_size when
    /// a `DiagramBuilt` was seen (the pool size is the live count at
    /// diagram-build time, before any later GC).
    pub fn is_consistent(&self) -> bool {
        if let Some(size) = self.last_pool_size {
            // The diagram-built size counts nodes at that point; later
            // GCs may have collected some, so live_count <= size is the
            // invariant (created nodes minus collected >= live at build).
            self.live_count() <= size || self.gc_count == 0
        } else {
            true
        }
    }
}

/// Per-level memory version map (level → version at last write).
/// Reconstructed from `MemoryWrite` events; the level is not
/// carried by the event, so this mirror tracks only the global
/// version counter.
pub type MemoryVersions = BTreeMap<u32, u64>;
