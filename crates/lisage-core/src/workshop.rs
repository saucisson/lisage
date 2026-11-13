//! **Didactic workshop** — reduction dashboard (backlog § 9 of the
//! roadmap, request of 10 November 2026): make the thesis "the diagram is a
//! function of the controller" tangible by
//! enabling/disabling each reduction one at a time and measuring.
//!
//! First fragment (backend, no web front):
//!
//! - [`Reductions`]: R-collapse / R-default switches;
//! - [`WorkshopController`]: wrapper controller that toggles them (the pool
//!   reads the `collapse_active` / `default_active` hooks at interning);
//! - [`measure`]: three didactic metrics — the **shared nodes**
//!   (R-share active, count of reachable handles), the **paths**
//!   (number of root → terminal paths) and the **unfolded size** (number
//!   of nodes of the unfolded tree, each edge duplicating the target subtree:
//!   what the absence of hash-consing would cost). Golden thread example: R₃ fits in
//!   33 shared nodes against an unfolded size of 126, the 20 paths
//!   being one per reachable state.
//!
//! R-share itself is not switchable (it is the pool's unique table):
//! its deactivation is **measured** by the unfolded size, not by a
//! degraded pool.
//!
//! Fragment 3 (order parameter, backlog § 9):
//!
//! - [`Order`]: level → variable permutation, applied at compilation;
//! - [`compile_bdd_order`] / [`compare_orders`]: the same Boolean function
//!   compiled under two orders — same paths (2^n, one per assignment),
//!   different shared nodes: "the diagram is a function of the
//!   controller", the order is visible to the naked eye.
//!
//! Fragment 4 (composed anchor parameter, backlog § 9, sliding window
//! golden thread):
//!
//! - [`AnchorWindow`]: the composed anchor `{base, horizon}` of the
//!   SWP protocol — two named offsets, the slide is the displacement of the base,
//!   carried by the input edge;
//! - [`build_window_anchored`] / [`compare_anchors_window`]: the
//!   window read under two anchors — each anchor has its root, the
//!   relative form of the window is shared (`same_form`), only the
//!   absolute reads differ: "the anchor moves, the pool never does".

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Level, NodeId, PolicyMemory, Pool, PoolError,
};
use lisage_algebra::relative::{AnchorComposed, Fragment, GroupKeys};

use crate::observer::{ChannelAttribute, Observer, TraceEvent};

/// Reduction switches on the controller side (R-share excluded: measured).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reductions {
    /// R-collapse active.
    pub collapse: bool,
    /// R-default active (absorption of the δ(ℓ) edge towards D_δ).
    pub default: bool,
}

impl Reductions {
    /// All reductions enabled (canonical configuration).
    pub const ALL: Reductions = Reductions {
        collapse: true,
        default: true,
    };
    /// No controller-side reduction (R-share always active, measured).
    pub const NO: Reductions = Reductions {
        collapse: false,
        default: false,
    };
    /// R-collapse switched off, R-default active.
    pub fn without_collapse() -> Self {
        Reductions {
            collapse: false,
            default: true,
        }
    }
    /// R-default switched off, R-collapse active.
    pub fn without_default() -> Self {
        Reductions {
            collapse: true,
            default: false,
        }
    }
}

/// Wrapper controller that toggles R-collapse and R-default according to
/// [`Reductions`]; everything else is delegated to the inner controller.
pub struct WorkshopController<C: Controller> {
    inner: C,
    reductions: Reductions,
}

impl<C: Controller> WorkshopController<C> {
    /// Wraps `inner` with the `reductions` switches.
    pub fn new(inner: C, reductions: Reductions) -> Self {
        WorkshopController { inner, reductions }
    }

    /// The active switches.
    pub fn reductions(&self) -> Reductions {
        self.reductions
    }
}

impl<C: Controller> Controller for WorkshopController<C> {
    type Key = C::Key;
    type Payload = C::Payload;
    type Keys = C::Keys;

    // Fingerprint specific to the wrapper (outside the native controllers 1-6 range).
    const FINGERPRINT: ControllerId = ControllerId(100);

    fn keys(&self) -> &Self::Keys {
        self.inner.keys()
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        if self.reductions.default {
            self.inner.default_payload(level)
        } else {
            None
        }
    }

    fn collapse_active(&self) -> bool {
        self.reductions.collapse
    }

    fn default_active(&self) -> bool {
        self.reductions.default
    }

    fn policy_memory(&self) -> PolicyMemory {
        self.inner.policy_memory()
    }

    fn absorption_generalized(&self) -> bool {
        self.reductions.default && self.inner.absorption_generalized()
    }

    fn group_keys(&self) -> Option<&dyn GroupKeys<Key = Self::Key, Argument = i64>> {
        self.inner.group_keys()
    }
}

/// Didactic measurement report of a diagram.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportWorkshop {
    /// Distinct reachable nodes (R-share active): the shared diagram.
    pub nodes_shared: u64,
    /// Root → terminal paths in the DAG.
    pub paths: u64,
    /// Nodes of the unfolded tree (each edge duplicating the target subtree):
    /// the cost of the absence of hash-consing.
    pub size_unfolded: u64,
}

/// Measures a diagram: shared nodes, paths and unfolded size. Propagates
/// the pool error (an unknown `NodeId` is an error, not a
/// silent value). `observer`: narration channel (ADR-0100) — one
/// `Attribute`/`Synthesize` event carries the full report.
pub fn measure<P: Pool>(
    pool: &P,
    root: NodeId,
    observer: Option<&dyn Observer>,
) -> Result<ReportWorkshop, PoolError> {
    let report = ReportWorkshop {
        nodes_shared: count_nodes(pool, root)?,
        paths: count_paths(pool, root)?,
        size_unfolded: count_size_unfolded(pool, root)?,
    };
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root,
            value: format!(
                "nodes_shared={} paths={} size_unfolded={}",
                report.nodes_shared, report.paths, report.size_unfolded
            ),
        });
    }
    Ok(report)
}

/// Counts the distinct nodes reachable from `root`, terminals included.
pub fn count_nodes<P: Pool>(pool: &P, root: NodeId) -> Result<u64, PoolError> {
    let mut visit = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !visit.insert(node) {
            continue;
        }
        if let Form::Node { edges, .. } = pool.resolve(node)? {
            stack.extend(edges.values().copied());
        }
    }
    Ok(visit.len() as u64)
}

/// Counts the root → terminal paths of the DAG (memoized per node, saturated).
/// This is not the unfolded size: that one is the sum of the path
/// lengths, computed by [`count_size_unfolded`].
pub fn count_paths<P: Pool>(pool: &P, root: NodeId) -> Result<u64, PoolError> {
    fn paths<P: Pool>(
        pool: &P,
        node: NodeId,
        memo: &mut HashMap<NodeId, u64>,
    ) -> Result<u64, PoolError> {
        if let Some(total) = memo.get(&node) {
            return Ok(*total);
        }
        let total = match pool.resolve(node)? {
            Form::Terminal(_) => 1,
            Form::Node { edges, .. } => {
                let mut sum = 0u64;
                for target in edges.values() {
                    sum = sum.saturating_add(paths(pool, *target, memo)?);
                }
                sum
            }
        };
        memo.insert(node, total);
        Ok(total)
    }
    let mut memo = HashMap::new();
    paths(pool, root, &mut memo)
}

/// Number of nodes of the unfolded tree (memoized per node, saturated): each
/// edge duplicates the subtree of its target, what the absence of
/// R-share would cost.
pub fn count_size_unfolded<P: Pool>(pool: &P, root: NodeId) -> Result<u64, PoolError> {
    fn size<P: Pool>(
        pool: &P,
        node: NodeId,
        memo: &mut HashMap<NodeId, u64>,
    ) -> Result<u64, PoolError> {
        if let Some(total) = memo.get(&node) {
            return Ok(*total);
        }
        let total = match pool.resolve(node)? {
            Form::Terminal(_) => 1,
            Form::Node { edges, .. } => {
                let mut sum = 1u64;
                for target in edges.values() {
                    sum = sum.saturating_add(size(pool, *target, memo)?);
                }
                sum
            }
        };
        memo.insert(node, total);
        Ok(total)
    }
    let mut memo = HashMap::new();
    size(pool, root, &mut memo)
}

/// Level of the root of the sliding window.
const LEVEL_ROOT: u32 = 0;
/// Level of the window node of the sliding window.
const LEVEL_FENETRE: u32 = 1;

/// **Encoding parameter** (didactic workshop, backlog § 9: "play with the
/// controller parameters — order, absolute/relative encoding, kingdom,
/// composed anchor").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    /// Absolute keys: each window has its own keys, one node per base.
    Absolute,
    /// Relative keys (min section = 0, M4-2): a single shared form,
    /// the position argument lives on the input edge.
    Relative,
}

/// Didactic comparison of the two encodings on the same logical object.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportComparison {
    /// Metrics of the absolute encoding.
    pub absolute: ReportWorkshop,
    /// Metrics of the relative encoding.
    pub relative: ReportWorkshop,
}

/// Builds the **sliding window** in absolute encoding: for each base
/// `b ∈ [0, count_bases)`, a window of its own `{b..b+size-1}` (one node per
/// base), all linked to the root. The node count grows with
/// `count_bases` — this is the "infinity of the window" of the flat prototype.
pub fn build_window_absolute<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    count_bases: u64,
) -> NodeId {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let mut edges = BTreeMap::new();
    for base in 0..count_bases {
        let window = pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_FENETRE),
                edges: (0..size).map(|k| (base + k, true_node)).collect(),
            },
        );
        edges.insert(base, window);
    }
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_ROOT),
            edges,
        },
    )
}

/// Interns the relative form of the window (keys `0..size-1`, min
/// section = 0), shared by all bases.
fn intern_window_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
) -> NodeId {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_FENETRE),
            edges: (0..size).map(|k| (k, true_node)).collect(),
        },
    )
}

/// Builds the same sliding window in **relative** encoding (M4-2,
/// min section = 0): a single form `{0..size-1}` shared by all the
/// bases (the position argument lives on the input edge). The node
/// count is constant — this is the "4 against infinity" of the window.
pub fn build_window_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    count_bases: u64,
) -> NodeId {
    let window = intern_window_relative(pool, anchor, size);
    let edges = (0..count_bases).map(|base| (base, window)).collect();
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_ROOT),
            edges,
        },
    )
}

/// Builds the sliding window in the requested `encoding`: single entry
/// point of the encoding parameter.
pub fn build_window<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    encoding: Encoding,
    size: u64,
    count_bases: u64,
) -> NodeId {
    match encoding {
        Encoding::Absolute => build_window_absolute(pool, anchor, size, count_bases),
        Encoding::Relative => build_window_relative(pool, anchor, size, count_bases),
    }
}

/// Compares the two encodings of the sliding window over `count_bases` bases:
/// same paths (same logical object), different shared nodes — the
/// relative sharing is visible to the naked eye. `observer`: ADR-0100 narration.
pub fn compare_encodings_window<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    count_bases: u64,
    observer: Option<&dyn Observer>,
) -> Result<ReportComparison, PoolError> {
    let root_absolute = build_window(pool, anchor, Encoding::Absolute, size, count_bases);
    let root_relative = build_window(pool, anchor, Encoding::Relative, size, count_bases);
    let absolute = measure(pool, root_absolute, observer)?;
    let relative = measure(pool, root_relative, observer)?;
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root_relative,
            value: format!(
                "encoding=relative_vs_absolute nodes={} vs {}",
                relative.nodes_shared, absolute.nodes_shared
            ),
        });
    }
    Ok(ReportComparison { absolute, relative })
}

/// **Order parameter** (didactic workshop, backlog § 9: "play with the
/// controller parameters — order, absolute/relative encoding, kingdom,
/// composed anchor"): level → variable permutation. Level ℓ decides
/// variable `permutation[ℓ]`; the natural order is the identity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Order {
    /// `permutation[ℓ]` = variable decided at level ℓ.
    permutation: Vec<u32>,
}

impl Order {
    /// Natural order: level ℓ decides variable ℓ.
    pub fn naturel(number_variables: u32) -> Self {
        Order {
            permutation: (0..number_variables).collect(),
        }
    }

    /// Reversed order: level ℓ decides variable n-1-ℓ.
    pub fn inverse(number_variables: u32) -> Self {
        Order {
            permutation: (0..number_variables).rev().collect(),
        }
    }

    /// Custom order: `permutation[ℓ]` = variable of level ℓ. Must
    /// be a permutation of `0..n` (otherwise two levels would decide the
    /// same variable and one variable would be skipped).
    pub fn personnalise(permutation: Vec<u32>) -> Self {
        let mut sorted = permutation.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            (0..permutation.len() as u32).collect::<Vec<_>>(),
            "the order is a permutation of 0..n (internal invariant)"
        );
        Order { permutation }
    }

    /// Number of variables.
    pub fn number_variables(&self) -> u32 {
        self.permutation.len() as u32
    }

    /// Variable decided at level `level`.
    pub fn variable_at_level(&self, level: u32) -> u32 {
        self.permutation[level as usize]
    }

    /// Didactic label `[v₀, v₁, …]` for the narration.
    pub fn label(&self) -> String {
        let variables: Vec<String> = self.permutation.iter().map(u32::to_string).collect();
        format!("[{}]", variables.join(","))
    }
}

/// Compiles a truth table into a classic BDD under the given `order`:
/// top-down Shannon expansion, level ℓ decides variable
/// `order.variable_at_level(ℓ)`. `table[a]` = value of the function on
/// assignment `a` (bit ℓ of `a` = variable ℓ); the length must be
/// `2^n`.
pub fn compile_bdd_order<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    order: &Order,
    table: &[bool],
) -> NodeId {
    let number_variables = order.number_variables();
    assert_eq!(
        table.len(),
        1usize << number_variables,
        "truth table of size 2^n (internal invariant)"
    );
    // Permutes the table: bit ℓ of the local index = bit `order[ℓ]` of
    // the original index, then the classic Shannon expansion applies
    // (at level ℓ we branch on bit ℓ of the local index).
    let table_local: Vec<bool> = (0..table.len())
        .map(|indice| {
            let mut original = 0usize;
            for level in 0..number_variables {
                original |= ((indice >> level) & 1) << order.variable_at_level(level) as usize;
            }
            table[original]
        })
        .collect();
    shannon(pool, anchor, Level(LEVEL_ROOT), &table_local)
}

/// Top-down Shannon expansion: `table` is indexed by the bits of the
/// remaining variables (bit ℓ of the index = variable of level ℓ); the
/// even positions (bit of the current level = 0) give edge 0, the
/// odd ones edge 1.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
) -> NodeId {
    if table.len() == 1 {
        return pool.intern(anchor, Form::Terminal(table[0]));
    }
    let low_table: Vec<bool> = table.iter().step_by(2).copied().collect();
    let top_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
    let low = shannon(pool, anchor, Level(level.0 + 1), &low_table);
    let top = shannon(pool, anchor, Level(level.0 + 1), &top_table);
    pool.intern(
        anchor,
        Form::Node {
            level,
            edges: BTreeMap::from([(0, low), (1, top)]),
        },
    )
}

/// Didactic comparison of two orders on the same Boolean function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportComparisonOrders {
    /// Metrics under the first order.
    pub first: ReportWorkshop,
    /// Metrics under the second order.
    pub second: ReportWorkshop,
}

/// Compiles the same truth table under two orders and measures: same
/// paths (2^n, one per assignment — same logical object), different shared
/// nodes — the order changes the diagram, not the function.
/// `observer`: ADR-0100 narration.
pub fn compare_orders<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    table: &[bool],
    order_first: &Order,
    order_second: &Order,
    observer: Option<&dyn Observer>,
) -> Result<ReportComparisonOrders, PoolError> {
    assert_eq!(
        order_first.number_variables(),
        order_second.number_variables(),
        "the two orders cover the same number of variables (internal invariant)"
    );
    let root_first = compile_bdd_order(pool, anchor, order_first, table);
    let root_second = compile_bdd_order(pool, anchor, order_second, table);
    let first = measure(pool, root_first, observer)?;
    let second = measure(pool, root_second, observer)?;
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root_second,
            value: format!(
                "order={}_vs_{} nodes={} vs {}",
                order_first.label(),
                order_second.label(),
                first.nodes_shared,
                second.nodes_shared
            ),
        });
    }
    Ok(ReportComparisonOrders { first, second })
}

/// Fragment named "base" of the composed anchor of the sliding window
/// (sequence position, SWP golden thread).
pub fn fragment_base() -> Fragment {
    Fragment("base".to_string())
}

/// Fragment named "horizon" of the composed anchor of the sliding window
/// (window size, SWP golden thread).
pub fn fragment_horizon() -> Fragment {
    Fragment("horizon".to_string())
}

/// **Composed anchor parameter** (didactic workshop, backlog § 9:
/// "play with the controller parameters — order, absolute/relative encoding,
/// kingdom, composed anchor"; sliding window golden thread § 4-5):
/// the window anchor is a named record of two offsets,
/// `{base, horizon}`. The **slide** is the displacement of the base; the anchor
/// stays in the handle (C1 unchanged), the pool stays blind.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnchorWindow {
    inner: AnchorComposed,
}

impl AnchorWindow {
    /// Anchor `{base, horizon}`: the two named offsets of the SWP protocol.
    pub fn new(base: i64, horizon: i64) -> Self {
        AnchorWindow {
            inner: AnchorComposed::new()
                .with(fragment_base(), base)
                .with(fragment_horizon(), horizon),
        }
    }

    /// Offset of the base (sequence position).
    pub fn base(&self) -> i64 {
        self.inner.offset(&fragment_base())
    }

    /// Offset of the horizon (window size).
    pub fn horizon(&self) -> i64 {
        self.inner.offset(&fragment_horizon())
    }

    /// New anchor, base shifted by `delta`: the slide of the window.
    pub fn glessee(&self, delta: i64) -> Self {
        AnchorWindow {
            inner: self.inner.decalee(&fragment_base(), delta),
        }
    }

    /// Absolute read of the window: the interval `[base, base + horizon)`.
    pub fn window_absolute(&self) -> (i64, i64) {
        let base = self.base();
        (base, base + self.horizon())
    }

    /// Key of the input edge: the base of the anchor, saturated in `u64`.
    /// This is the position argument (spec § 2.4.5: the incoming edge carries
    /// the argument) — it lives on the edge, never in the interning key.
    pub fn key_input(&self) -> u64 {
        self.base().max(0) as u64
    }

    /// Didactic label `{base: b, horizon: k}` for the narration.
    pub fn label(&self) -> String {
        format!("{{base: {}, horizon: {}}}", self.base(), self.horizon())
    }
}

/// Builds the sliding window read under the composed `anchor`: the base of
/// the anchor is the argument of the input edge (key `key_input`), the form of
/// the window stays relative (keys `0..size-1`, min section = 0) and
/// shared. Returns `(root, window)`: the root specific to this base,
/// the window form reached by its input edge.
pub fn build_window_anchored<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    window_anchor: &AnchorWindow,
) -> (NodeId, NodeId) {
    let window = intern_window_relative(pool, anchor, size);
    let root = pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_ROOT),
            edges: BTreeMap::from([(window_anchor.key_input(), window)]),
        },
    );
    (root, window)
}

/// Didactic comparison of two composed anchors on the same window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportComparisonAnchors {
    /// Metrics of the window read under the first anchor.
    pub before: ReportWorkshop,
    /// Metrics of the window read under the second anchor.
    pub after: ReportWorkshop,
    /// True if the two reads reach the same window form
    /// (`NodeId`): moving the base does not duplicate the form.
    pub same_form: bool,
    /// Count of pool nodes after the two reads (one root per
    /// anchor, a single window form).
    pub nodes_pool: u64,
    /// Absolute read `[base, base + horizon)` under the first anchor.
    pub read_before: (i64, i64),
    /// Absolute read `[base, base + horizon)` under the second anchor.
    pub read_after: (i64, i64),
}

/// Compares the sliding window read under two composed anchors: each
/// anchor builds its own root (the input edge carries its base), but the
/// relative form of the window (keys `0..size-1`, min section = 0) is
/// shared — `same_form` true, same nodes, same paths — only the
/// absolute reads differ. This is the slide of the SWP protocol:
/// "the anchor moves, the pool never does": moving the base does not duplicate the
/// window form. `observer`: ADR-0100 narration.
pub fn compare_anchors_window<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    anchor_before: &AnchorWindow,
    anchor_after: &AnchorWindow,
    observer: Option<&dyn Observer>,
) -> Result<ReportComparisonAnchors, PoolError> {
    // Each anchor enters the pool through its input edge (its base); the
    // window form reached on both sides must be the same node.
    let (root_before, window_before) = build_window_anchored(pool, anchor, size, anchor_before);
    let (root_after, window_after) = build_window_anchored(pool, anchor, size, anchor_after);
    let nodes_pool = pool.node_count() as u64;
    let same_form = window_before == window_after;
    let before = measure(pool, root_before, observer)?;
    let after = measure(pool, root_after, observer)?;
    if let Some(obs) = observer {
        let (start_before, end_before) = anchor_before.window_absolute();
        let (start_after, end_after) = anchor_after.window_absolute();
        obs.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root_after,
            value: format!(
                "anchor={}_vs_{} nodes={} vs {} read=[{}..{}) vs [{}..{}) pool={}",
                anchor_before.label(),
                anchor_after.label(),
                before.nodes_shared,
                after.nodes_shared,
                start_before,
                end_before,
                start_after,
                end_after,
                nodes_pool,
            ),
        });
    }
    Ok(ReportComparisonAnchors {
        before,
        after,
        same_form,
        nodes_pool,
        read_before: anchor_before.window_absolute(),
        read_after: anchor_after.window_absolute(),
    })
}
