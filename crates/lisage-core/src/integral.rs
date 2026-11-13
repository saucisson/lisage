//! **Integral at the fold** (backlog § 9 of the roadmap, open
//! question 9 of the spec): over 1D region keys (intervals), the
//! discrete fold — sum of cardinalities — becomes the **Lebesgue integral**:
//! the measure of an interval `[a, b]` in the continuous interpretation is
//! its length `b − a` (the isolated point has zero measure), and the integral
//! of a constant density 1 over the diagram is the measure of the denoted
//! set.
//!
//! The integral is computed with the current region as context:
//! `integral(node, region) = Σ_{edges I → child} integral(child, region ∩ I)`,
//! the terminal `⊤` returning the measure of the current region, `⊥` returning 0.
//! On a single-level diagram, it is the sum of the lengths of the
//! intervals leading to `⊤` — the differential against the cardinal fold
//! (sum of sizes) shows the count → measure passage.
//!
//! **DBM** variant (spec Q9 (ii), "polyhedral regions"): over
//! 2-variable DBM region keys, the Lebesgue measure is the **area** of
//! the zone ∩ box (`DbmKeys::measure_zone`, returned doubled in `i128`), and
//! the integral of constant density 1 over the diagram is the area of
//! the denoted set — `integral(node, region) = Σ_{edges Z → child}
//! integral(child, region ∩ Z)`.
//!
//! Narration (ADR-0100 decision 6): each recursive call emits an
//! `Attribute` event (channel `Synthesize`) to the optional observer,
//! with the computed measure; the test pins these events.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{Form, NodeId, Pool, PoolError};

use crate::dbm::{Dbm, DbmKeys};
use crate::observer::{ChannelAttribute, Observer, TraceEvent};
use crate::region::Interval;

/// Lebesgue measure of an interval in the continuous interpretation:
/// `length([a, b]) = b − a`; the empty interval and the isolated point have
/// zero measure.
pub fn measure_interval(interval: &Interval) -> u64 {
    if interval.is_empty() {
        0
    } else {
        interval.bound_sup().saturating_sub(interval.bound_inf())
    }
}

/// Intersection of two intervals (set-theoretic meet): the interval
/// empty if disjoint.
pub fn intersection(a: &Interval, b: &Interval) -> Interval {
    if a.is_empty() || b.is_empty() {
        return crate::region::EMPTY;
    }
    Interval::new(
        a.bound_inf().max(b.bound_inf()),
        a.bound_sup().min(b.bound_sup()),
    )
}

/// Integral of the constant density 1 over the set denoted by the
/// 1D region-key diagram, restricted to the current `region`
/// (continuous interpretation: Lebesgue measure, in saturated `u64`).
///
/// `observer`: narration channel (ADR-0100) — each visitd node emits
/// an `Attribute`/`Synthesize` event with the computed measure.
pub fn integral_constant<P: Pool<Key = Interval, Payload = bool>>(
    pool: &P,
    root: NodeId,
    region: &Interval,
    observer: Option<&dyn Observer>,
) -> Result<u64, PoolError> {
    let mut memo: HashMap<(NodeId, Interval), u64> = HashMap::new();
    integral(pool, root, region, observer, &mut memo)
}

fn integral<P: Pool<Key = Interval, Payload = bool>>(
    pool: &P,
    node: NodeId,
    region: &Interval,
    observer: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Interval), u64>,
) -> Result<u64, PoolError> {
    if let Some(total) = memo.get(&(node, *region)) {
        if let Some(obs) = observer {
            obs.on_event(&TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                node,
                value: format!("measure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(node)?.clone() {
        Form::Terminal(true) => measure_interval(region),
        Form::Terminal(false) => 0,
        Form::Node { edges, .. } => {
            let mut sum = 0u64;
            for (interval, target) in &edges {
                let under_region = intersection(region, interval);
                if under_region.is_empty() {
                    continue;
                }
                let contribution = integral(pool, *target, &under_region, observer, memo)?;
                sum = sum.saturating_add(contribution);
            }
            sum
        }
    };
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node,
            value: format!("measure={total}"),
        });
    }
    memo.insert((node, *region), total);
    Ok(total)
}

/// Integral of the constant density 1 over the set denoted by the
/// DBM region-key diagram (2 variables), restricted to the current `region`,
/// intersected with the bounded box of `keys`. The measure is
/// the **doubled** area (`i128`, exact) — see [`DbmKeys::measure_zone`].
///
/// `observer`: narration channel (ADR-0100) — each visitd node emits
/// an `Attribute`/`Synthesize` event with the computed measure.
pub fn integral_dbm_constant<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    root: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observer: Option<&dyn Observer>,
) -> Result<i128, PoolError> {
    let mut memo: HashMap<(NodeId, Dbm), i128> = HashMap::new();
    integral_dbm(pool, root, region, keys, observer, &mut memo)
}

fn integral_dbm<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    node: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observer: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Dbm), i128>,
) -> Result<i128, PoolError> {
    if let Some(total) = memo.get(&(node, region.clone())) {
        if let Some(obs) = observer {
            obs.on_event(&TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                node,
                value: format!("measure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(node)?.clone() {
        Form::Terminal(true) => keys.measure_zone(region),
        Form::Terminal(false) => 0,
        Form::Node { edges, .. } => {
            let mut sum = 0i128;
            for (zone, target) in &edges {
                let under_region = region.intersection(zone);
                if under_region.is_empty() {
                    continue;
                }
                let contribution =
                    integral_dbm(pool, *target, &under_region, keys, observer, memo)?;
                sum += contribution;
            }
            sum
        }
    };
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node,
            value: format!("measure={total}"),
        });
    }
    memo.insert((node, region.clone()), total);
    Ok(total)
}

/// Builds a single-level diagram denoting the union of the given
/// `intervals` (edges to `⊤`), the complement going to
/// `⊥` — integral instance-test: the measure of the union is the sum
/// of the lengths. **Precondition**: the intervals are pairwise
/// disjoint (debug assertion) — otherwise the integral would sum the
/// individual lengths and would over-count the measure of the union.
pub fn compile_union<P: Pool<Key = Interval, Payload = bool>>(
    pool: &mut P,
    anchor: lisage_algebra::interface::Anchor,
    intervals: &[Interval],
) -> NodeId {
    debug_assert!(
        intervals.windows(2).all(|pair| {
            let a = &pair[0];
            let b = &pair[1];
            a.is_empty() || b.is_empty() || a.bound_sup() < b.bound_inf()
        }),
        "compile_union requires disjoint intervals (precondition)"
    );
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let edges: BTreeMap<Interval, NodeId> = intervals
        .iter()
        .map(|interval| (*interval, true_node))
        .collect();
    pool.intern(
        anchor,
        Form::Node {
            level: lisage_algebra::interface::Level(0),
            edges,
        },
    )
}
