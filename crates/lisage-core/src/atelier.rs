//! **Didactic workshop** — reduction dashboard (backlog § 9 of the
//! roadmap, request of November 10, 2026): making the thesis "the diagram
//! is a function of the controller" tangible by enabling/disabling each
//! reduction one at a time and measuring.
//!
//! First fragment (backend, no web front):
//!
//! - [`Reductions`]: R-collapse / R-default switches;
//! - [`AtelierController`]: wrapping controller that toggles them (the pool
//!   reads the `collapse_active` / `default_active` hooks at interning time);
//! - [`measure`]: three didactic metrics — the **shared nodes**
//!   (R-share active, count of reachable handles), the **paths**
//!   (number of root → terminal paths) and the **unfolded size** (number
//!   of nodes in the unfolded tree, each arc duplicating the target subtree:
//!   what the absence of hash-consing would cost). Thread example: R₃ fits in
//!   33 shared nodes versus an unfolded size of 126, the 20 paths
//!   being one per reachable state.
//!
//! R-share itself is not switchable (it is the pool's single table):
//! its deactivation is **measured** by the unfolded size, not by a
//! degraded pool.
//!
//! Fragment 3 (ordering parameter, backlog § 9):
//!
//! - [`Order`]: level → variable permutation, applied at compile time;
//! - [`compiler_bdd_order`] / [`comparer_orders`]: the same boolean function
//!   compiled under two orderings — same paths (2^n, one per assignment),
//!   different shared nodes: "the diagram is a function of the
//!   controller", the ordering is visible to the naked eye.
//!
//! Fragment 4 (composite anchor parameter, backlog § 9, sliding-window
//! thread):
//!
//! - [`AnchorWindow`]: the composite anchor `{base, horizon}` of the
//!   SWP protocol — two named offsets, the slide is the movement of the base,
//!   carried by the incoming arc;
//! - [`build_window_anchored`] / [`comparer_anchors_window`]: the
//!   window read under two anchors — each anchor has its own root, the
//!   relative shape of the window is shared (`same_form`), only the
//!   absolute readings differ: "the anchor moves, the pool never does".

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Level, MemoryPolicy, NodeId, Pool, PoolError,
};
use lisage_algebra::relatif::{AnchorComposee, CycleGroup, Fragment};

use crate::observer::{CanalAttr, Observer, TraceEvent};

/// Controller-side reduction switches (R-share excluded: measured).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reductions {
    /// R-collapse active.
    pub collapse: bool,
    /// R-default active (absorption of the δ(ℓ) arc towards D_δ).
    pub defaut: bool,
}

impl Reductions {
    /// All reductions enabled (canonical configuration).
    pub const TOUTES: Reductions = Reductions {
        collapse: true,
        defaut: true,
    };
    /// No controller-side reduction (R-share always active, measured).
    pub const AUCUNE: Reductions = Reductions {
        collapse: false,
        defaut: false,
    };
    /// R-collapse toggled off, R-default active.
    pub fn without_collapse() -> Self {
        Reductions {
            collapse: false,
            defaut: true,
        }
    }
    /// R-default toggled off, R-collapse active.
    pub fn without_defaut() -> Self {
        Reductions {
            collapse: true,
            defaut: false,
        }
    }
}

/// Wrapping controller that toggles R-collapse and R-default according to
/// [`Reductions`]; everything else is delegated to the inner controller.
pub struct AtelierController<C: Controller> {
    inner: C,
    reductions: Reductions,
}

impl<C: Controller> AtelierController<C> {
    /// Wraps `inner` with the `reductions` switches.
    pub fn new(inner: C, reductions: Reductions) -> Self {
        AtelierController { inner, reductions }
    }

    /// The active switches.
    pub fn reductions(&self) -> Reductions {
        self.reductions
    }
}

impl<C: Controller> Controller for AtelierController<C> {
    type Key = C::Key;
    type Payload = C::Payload;
    type Keys = C::Keys;

    // Fingerprint specific to the wrapper (outside the native controllers 1-6 range).
    const FINGERPRINT: ControllerId = ControllerId(100);

    fn keys(&self) -> &Self::Keys {
        self.inner.keys()
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        if self.reductions.defaut {
            self.inner.default_payload(level)
        } else {
            None
        }
    }

    fn collapse_active(&self) -> bool {
        self.reductions.collapse
    }

    fn default_active(&self) -> bool {
        self.reductions.defaut
    }

    fn memory_policy(&self) -> MemoryPolicy {
        self.inner.memory_policy()
    }

    fn generalized_absorption(&self) -> bool {
        self.reductions.defaut && self.inner.generalized_absorption()
    }

    fn cycle_group(&self) -> Option<&dyn CycleGroup<Key = Self::Key, Argument = i64>> {
        self.inner.cycle_group()
    }
}

/// Didactic measurement report of a diagram.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportAtelier {
    /// Distinct reachable nodes (R-share active): the shared diagram.
    pub nodes_partages: u64,
    /// Root → terminal paths in the DAG.
    pub paths: u64,
    /// Nodes of the unfolded tree (each arc duplicating the target subtree):
    /// the cost of the absence of hash-consing.
    pub size_depliee: u64,
}

/// Measures a diagram: shared nodes, paths and unfolded size. Propagates
/// the pool error (an unknown `NodeId` is an error, not a silent value).
/// `observer`: narration channel (ADR-0100) — an `Attr`/`Synthesized`
/// event carries the full report.
pub fn measure<P: Pool>(
    pool: &P,
    root: NodeId,
    observer: Option<&dyn Observer>,
) -> Result<ReportAtelier, PoolError> {
    let report = ReportAtelier {
        nodes_partages: count_nodes(pool, root)?,
        paths: count_paths(pool, root)?,
        size_depliee: count_size_depliee(pool, root)?,
    };
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthesized,
            node: root,
            value: format!(
                "nodes_partages={} paths={} size_depliee={}",
                report.nodes_partages, report.paths, report.size_depliee
            ),
        });
    }
    Ok(report)
}

/// Counts the distinct nodes reachable from `root`, terminals included.
pub fn count_nodes<P: Pool>(pool: &P, root: NodeId) -> Result<u64, PoolError> {
    let mut visite = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !visite.insert(node) {
            continue;
        }
        if let Form::Node { arcs, .. } = pool.resolve(node)? {
            stack.extend(arcs.values().copied());
        }
    }
    Ok(visite.len() as u64)
}

/// Counts the root → terminal paths of the DAG (memoized per node, saturating).
/// This is not the unfolded size: that one is the sum of path lengths,
/// computed by [`count_size_depliee`].
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
            Form::Node { arcs, .. } => {
                let mut sum = 0u64;
                for cible in arcs.values() {
                    sum = sum.saturating_add(paths(pool, *cible, memo)?);
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

/// Number of nodes in the unfolded tree (memoized per node, saturating): each
/// arc duplicates the subtree of its target, what the absence of
/// R-share would cost.
pub fn count_size_depliee<P: Pool>(pool: &P, root: NodeId) -> Result<u64, PoolError> {
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
            Form::Node { arcs, .. } => {
                let mut sum = 1u64;
                for cible in arcs.values() {
                    sum = sum.saturating_add(size(pool, *cible, memo)?);
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

/// Level of the sliding-window root.
const LEVEL_ROOT: u32 = 0;
/// Level of the window node of the sliding window.
const LEVEL_WINDOW: u32 = 1;

/// **Encoding parameter** (didactic workshop, backlog § 9: "playing with the
/// controller parameters — ordering, absolute/relative encoding, realm,
/// composite anchor").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encodage {
    /// Absolute keys: each window has its own keys, one node per base.
    Absolu,
    /// Relative keys (min section = 0, M4-2): a single shared shape,
    /// the position argument lives on the incoming arc.
    Relatif,
}

/// Didactic comparison of the two encodings on the same logical object.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportComparaison {
    /// Metrics of the absolute encoding.
    pub absolu: ReportAtelier,
    /// Metrics of the relative encoding.
    pub relatif: ReportAtelier,
}

/// Builds the **sliding window** in absolute encoding: for each base
/// `b ∈ [0, nb_bases)`, its own window `{b..b+size-1}` (one node per
/// base), all linked to the root. The number of nodes grows with
/// `nb_bases` — this is the "infinity of the window" of the flat prototype.
pub fn build_window_absolue<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    nb_bases: u64,
) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let mut arcs = BTreeMap::new();
    for base in 0..nb_bases {
        let window = pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_WINDOW),
                arcs: (0..size).map(|k| (base + k, vrai)).collect(),
            },
        );
        arcs.insert(base, window);
    }
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_ROOT),
            arcs,
        },
    )
}

/// Interns the relative shape of the window (keys `0..size-1`, min
/// section = 0), shared by all bases.
fn interner_window_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_WINDOW),
            arcs: (0..size).map(|k| (k, vrai)).collect(),
        },
    )
}

/// Builds the same sliding window in **relative** encoding (M4-2,
/// min section = 0): a single shape `{0..size-1}` shared by all the
/// bases (the position argument lives on the incoming arc). The number of
/// nodes is constant — this is the "4 against infinity" of the window.
pub fn build_window_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    nb_bases: u64,
) -> NodeId {
    let window = interner_window_relative(pool, anchor, size);
    let arcs = (0..nb_bases).map(|base| (base, window)).collect();
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_ROOT),
            arcs,
        },
    )
}

/// Builds the sliding window in the requested `encodage`: single entry
/// point of the encoding parameter.
pub fn build_window<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    encodage: Encodage,
    size: u64,
    nb_bases: u64,
) -> NodeId {
    match encodage {
        Encodage::Absolu => build_window_absolue(pool, anchor, size, nb_bases),
        Encodage::Relatif => build_window_relative(pool, anchor, size, nb_bases),
    }
}

/// Compares the two encodings of the sliding window over `nb_bases` bases:
/// same paths (same logical object), different shared nodes — the
/// relative sharing is visible to the naked eye. `observer`: ADR-0100 narration.
pub fn comparer_encodages_window<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    nb_bases: u64,
    observer: Option<&dyn Observer>,
) -> Result<ReportComparaison, PoolError> {
    let root_absolue = build_window(pool, anchor, Encodage::Absolu, size, nb_bases);
    let root_relative = build_window(pool, anchor, Encodage::Relatif, size, nb_bases);
    let absolu = measure(pool, root_absolue, observer)?;
    let relatif = measure(pool, root_relative, observer)?;
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthesized,
            node: root_relative,
            value: format!(
                "encodage=relatif_vs_absolu nodes={} contre {}",
                relatif.nodes_partages, absolu.nodes_partages
            ),
        });
    }
    Ok(ReportComparaison { absolu, relatif })
}

/// **Ordering parameter** (didactic workshop, backlog § 9: "playing with the
/// controller parameters — ordering, absolute/relative encoding, realm,
/// composite anchor"): level → variable permutation. Level ℓ decides
/// variable `permutation[ℓ]`; the natural ordering is the identity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Order {
    /// `permutation[ℓ]` = variable decided at level ℓ.
    permutation: Vec<u32>,
}

impl Order {
    /// Natural ordering: level ℓ decides variable ℓ.
    pub fn naturel(number_variables: u32) -> Self {
        Order {
            permutation: (0..number_variables).collect(),
        }
    }

    /// Reversed ordering: level ℓ decides variable n-1-ℓ.
    pub fn inverse(number_variables: u32) -> Self {
        Order {
            permutation: (0..number_variables).rev().collect(),
        }
    }

    /// Custom ordering: `permutation[ℓ]` = variable of level ℓ. Must
    /// be a permutation of `0..n` (otherwise two levels would decide the
    /// same variable and one variable would be skipped).
    pub fn personnalise(permutation: Vec<u32>) -> Self {
        let mut triee = permutation.clone();
        triee.sort_unstable();
        assert_eq!(
            triee,
            (0..permutation.len() as u32).collect::<Vec<_>>(),
            "the ordering is a permutation of 0..n (internal invariant)"
        );
        Order { permutation }
    }

    /// Number of variables.
    pub fn number_variables(&self) -> u32 {
        self.permutation.len() as u32
    }

    /// Variable decided at level `level`.
    pub fn variable_au_level(&self, level: u32) -> u32 {
        self.permutation[level as usize]
    }

    /// Didactic label `[v₀, v₁, …]` for narration.
    pub fn label(&self) -> String {
        let variables: Vec<String> = self.permutation.iter().map(u32::to_string).collect();
        format!("[{}]", variables.join(","))
    }
}

/// Compiles a truth table into a classic BDD under the given `order`:
/// top-down Shannon expansion, level ℓ decides variable
/// `order.variable_au_level(ℓ)`. `table[a]` = value of the function on
/// assignment `a` (bit ℓ of `a` = variable ℓ); the length must be
/// `2^n`.
pub fn compiler_bdd_order<P: Pool<Key = u64, Payload = bool>>(
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
    // Permute the table: bit ℓ of the local index = bit `order[ℓ]` of
    // the original index, then the classic Shannon expansion applies
    // (at level ℓ we branch on bit ℓ of the local index).
    let table_locale: Vec<bool> = (0..table.len())
        .map(|indice| {
            let mut original = 0usize;
            for level in 0..number_variables {
                original |= ((indice >> level) & 1) << order.variable_au_level(level) as usize;
            }
            table[original]
        })
        .collect();
    shannon(pool, anchor, Level(LEVEL_ROOT), &table_locale)
}

/// Top-down Shannon expansion: `table` is indexed by the bits of the
/// remaining variables (bit ℓ of the index = variable of level ℓ); the
/// even positions (bit of the current level = 0) give arc 0, the
/// odd ones arc 1.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
) -> NodeId {
    if table.len() == 1 {
        return pool.intern(anchor, Form::Terminal(table[0]));
    }
    let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
    let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
    let bas = shannon(pool, anchor, Level(level.0 + 1), &bas_table);
    let haut = shannon(pool, anchor, Level(level.0 + 1), &haut_table);
    pool.intern(
        anchor,
        Form::Node {
            level,
            arcs: BTreeMap::from([(0, bas), (1, haut)]),
        },
    )
}

/// Didactic comparison of two orderings on the same boolean function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportComparaisonOrders {
    /// Metrics under the first ordering.
    pub first: ReportAtelier,
    /// Metrics under the second ordering.
    pub second: ReportAtelier,
}

/// Compiles the same truth table under two orderings and measures: same
/// paths (2^n, one per assignment — same logical object), different shared
/// nodes — the ordering changes the diagram, not the function.
/// `observer`: ADR-0100 narration.
pub fn comparer_orders<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    table: &[bool],
    order_first: &Order,
    order_second: &Order,
    observer: Option<&dyn Observer>,
) -> Result<ReportComparaisonOrders, PoolError> {
    assert_eq!(
        order_first.number_variables(),
        order_second.number_variables(),
        "both orderings cover the same number of variables (internal invariant)"
    );
    let root_first = compiler_bdd_order(pool, anchor, order_first, table);
    let root_second = compiler_bdd_order(pool, anchor, order_second, table);
    let first = measure(pool, root_first, observer)?;
    let second = measure(pool, root_second, observer)?;
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthesized,
            node: root_second,
            value: format!(
                "order={}_vs_{} nodes={} contre {}",
                order_first.label(),
                order_second.label(),
                first.nodes_partages,
                second.nodes_partages
            ),
        });
    }
    Ok(ReportComparaisonOrders { first, second })
}

/// Fragment named "base" of the composite anchor of the sliding window
/// (sequence position, SWP thread).
pub fn fragment_base() -> Fragment {
    Fragment("base".to_string())
}

/// Fragment named "horizon" of the composite anchor of the sliding window
/// (window size, SWP thread).
pub fn fragment_horizon() -> Fragment {
    Fragment("horizon".to_string())
}

/// **Composite anchor parameter** (didactic workshop, backlog § 9:
/// "playing with the controller parameters — ordering, absolute/relative
/// encoding, realm, composite anchor"; sliding-window thread § 4-5):
/// the window's anchor is a named record of two offsets,
/// `{base, horizon}`. The **slide** is the movement of the base; the anchor
/// stays in the handle (C1 unchanged), the pool stays blind.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnchorWindow {
    inner: AnchorComposee,
}

impl AnchorWindow {
    /// Anchor `{base, horizon}`: the two named offsets of the SWP protocol.
    pub fn new(base: i64, horizon: i64) -> Self {
        AnchorWindow {
            inner: AnchorComposee::new()
                .with(fragment_base(), base)
                .with(fragment_horizon(), horizon),
        }
    }

    /// Offset of the base (sequence position).
    pub fn base(&self) -> i64 {
        self.inner.decalage(&fragment_base())
    }

    /// Offset of the horizon (window size).
    pub fn horizon(&self) -> i64 {
        self.inner.decalage(&fragment_horizon())
    }

    /// New anchor, base shifted by `delta`: the slide of the window.
    pub fn glessee(&self, delta: i64) -> Self {
        AnchorWindow {
            inner: self.inner.decalee(&fragment_base(), delta),
        }
    }

    /// Absolute reading of the window: the interval `[base, base + horizon)`.
    pub fn window_absolue(&self) -> (i64, i64) {
        let base = self.base();
        (base, base + self.horizon())
    }

    /// Key of the incoming arc: the base of the anchor, saturated into `u64`.
    /// This is the position argument (spec § 2.4.5: the incoming arc carries
    /// the argument) — it lives on the arc, never in the interning key.
    pub fn key_input(&self) -> u64 {
        self.base().max(0) as u64
    }

    /// Didactic label `{base: b, horizon: k}` for narration.
    pub fn label(&self) -> String {
        format!("{{base: {}, horizon: {}}}", self.base(), self.horizon())
    }
}

/// Builds the sliding window read under the composite `anchor`: the base of
/// the anchor is the argument of the incoming arc (key `key_input`), the
/// shape of the window stays relative (keys `0..size-1`, min section = 0)
/// and shared. Returns `(root, window)`: the root specific to this base,
/// the window shape reached by its incoming arc.
pub fn build_window_anchored<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    window_anchor: &AnchorWindow,
) -> (NodeId, NodeId) {
    let window = interner_window_relative(pool, anchor, size);
    let root = pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_ROOT),
            arcs: BTreeMap::from([(window_anchor.key_input(), window)]),
        },
    );
    (root, window)
}

/// Didactic comparison of two composite anchors on the same window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReportComparaisonAnchors {
    /// Metrics of the window read under the first anchor.
    pub avant: ReportAtelier,
    /// Metrics of the window read under the second anchor.
    pub apres: ReportAtelier,
    /// True if both readings reach the same window shape
    /// (`NodeId`): moving the base does not duplicate the shape.
    pub same_form: bool,
    /// Node count of the pool after both readings (one root per
    /// anchor, a single window shape).
    pub nodes_pool: u64,
    /// Absolute reading `[base, base + horizon)` under the first anchor.
    pub lecture_avant: (i64, i64),
    /// Absolute reading `[base, base + horizon)` under the second anchor.
    pub lecture_apres: (i64, i64),
}

/// Compares the sliding window read under two composite anchors: each
/// anchor builds its own root (the incoming arc carries its base), but the
/// relative shape of the window (keys `0..size-1`, min section = 0) is
/// shared — `same_form` true, same nodes, same paths — only the
/// absolute readings differ. This is the slide of the SWP protocol:
/// "the anchor moves, the pool never does": moving the base does not
/// duplicate the window shape. `observer`: ADR-0100 narration.
pub fn comparer_anchors_window<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    size: u64,
    anchor_avant: &AnchorWindow,
    anchor_apres: &AnchorWindow,
    observer: Option<&dyn Observer>,
) -> Result<ReportComparaisonAnchors, PoolError> {
    // Each anchor enters the pool through its incoming arc (its base); the
    // window shape reached on both sides must be the same node.
    let (root_avant, window_avant) = build_window_anchored(pool, anchor, size, anchor_avant);
    let (root_apres, window_apres) = build_window_anchored(pool, anchor, size, anchor_apres);
    let nodes_pool = pool.node_count() as u64;
    let same_form = window_avant == window_apres;
    let avant = measure(pool, root_avant, observer)?;
    let apres = measure(pool, root_apres, observer)?;
    if let Some(obs) = observer {
        let (debut_avant, fin_avant) = anchor_avant.window_absolue();
        let (debut_apres, fin_apres) = anchor_apres.window_absolue();
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthesized,
            node: root_apres,
            value: format!(
                "anchor={}_vs_{} nodes={} contre {} lecture=[{}..{}) contre [{}..{}) pool={}",
                anchor_avant.label(),
                anchor_apres.label(),
                avant.nodes_partages,
                apres.nodes_partages,
                debut_avant,
                fin_avant,
                debut_apres,
                fin_apres,
                nodes_pool,
            ),
        });
    }
    Ok(ReportComparaisonAnchors {
        avant,
        apres,
        same_form,
        nodes_pool,
        lecture_avant: anchor_avant.window_absolue(),
        lecture_apres: anchor_apres.window_absolue(),
    })
}
