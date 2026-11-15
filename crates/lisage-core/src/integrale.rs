//! **Integral at the fold** (backlog § 9 of the roadmap, open question 9
//! of the spec): on 1D region-keys (intervals), the discrete fold — sum of
//! cardinalities — becomes the **Lebesgue integral**: the measure of an
//! interval `[a, b]` in the continuous interpretation is its length
//! `b − a` (the isolated point has measure zero), and the integral of a
//! constant density 1 over the diagram is the measure of the denoted set.
//!
//! The integral is computed with the current region as context:
//! `integral(node, region) = Σ_{arcs I → child} integral(child, region ∩ I)`,
//! the terminal `⊤` returning the measure of the current region, `⊥`
//! returning 0. On a single-level diagram, this is the sum of the lengths
//! of the intervals leading to `⊤` — the differential against the cardinality
//! fold (sum of sizes) shows the count → measure passage.
//!
//! **DBM** variant (spec Q9 (ii), "polyhedral regions"): on DBM
//! region-keys with 2 variables, the Lebesgue measure is the **area** of
//! the zone ∩ box (`DbmKeys::measure_zone`, returned doubled in `i128`), and
//! the integral of constant density 1 over the diagram is the area of the
//! denoted set — `integral(node, region) = Σ_{arcs Z → child}
//! integral(child, region ∩ Z)`.
//!
//! Narration (ADR-0100 decision 6): each recursive call emits an `Attr`
//! event (channel `Synthesized`) to the optional observer, with the computed
//! measure; the test pins these events.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{Form, NodeId, Pool, PoolError};

use crate::dbm::{Dbm, DbmKeys};
use crate::observer::{ChannelAttr, Observer, TraceEvent};
use crate::region::Interval;

/// Lebesgue measure of an interval in the continuous interpretation:
/// `length([a, b]) = b − a`; the empty interval and the isolated point
/// have measure zero.
pub fn measure_interval(interval: &Interval) -> u64 {
    if interval.est_empty() {
        0
    } else {
        interval.bound_sup().saturating_sub(interval.bound_inf())
    }
}

/// Intersection of two intervals (set-theoretic meet): the empty interval
/// if disjoint.
pub fn intersection(a: &Interval, b: &Interval) -> Interval {
    if a.est_empty() || b.est_empty() {
        return crate::region::EMPTY;
    }
    Interval::new(
        a.bound_inf().max(b.bound_inf()),
        a.bound_sup().min(b.bound_sup()),
    )
}

/// Integral of the constant density 1 over the set denoted by the
/// 1D region-key diagram, restricted to the current `region`
/// (continuous interpretation: Lebesgue measure, saturated `u64`).
///
/// `observer`: narration channel (ADR-0100) — each visited node emits
/// an `Attr`/`Synthesized` event with the computed measure.
pub fn integrale_constante<P: Pool<Key = Interval, Payload = bool>>(
    pool: &P,
    root: NodeId,
    region: &Interval,
    observer: Option<&dyn Observer>,
) -> Result<u64, PoolError> {
    let mut memo: HashMap<(NodeId, Interval), u64> = HashMap::new();
    integrale(pool, root, region, observer, &mut memo)
}

fn integrale<P: Pool<Key = Interval, Payload = bool>>(
    pool: &P,
    node: NodeId,
    region: &Interval,
    observer: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Interval), u64>,
) -> Result<u64, PoolError> {
    if let Some(total) = memo.get(&(node, *region)) {
        if let Some(obs) = observer {
            obs.on_event(&TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
                node,
                value: format!("measure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(node)?.clone() {
        Form::Terminal(true) => measure_interval(region),
        Form::Terminal(false) => 0,
        Form::Node { arcs, .. } => {
            let mut sum = 0u64;
            for (interval, target) in &arcs {
                let sous_region = intersection(region, interval);
                if sous_region.est_empty() {
                    continue;
                }
                let contribution = integrale(pool, *target, &sous_region, observer, memo)?;
                sum = sum.saturating_add(contribution);
            }
            sum
        }
    };
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node,
            value: format!("measure={total}"),
        });
    }
    memo.insert((node, *region), total);
    Ok(total)
}

/// Integral of the constant density 1 over the set denoted by the DBM
/// region-key diagram (2 variables), restricted to the current `region`,
/// intersected with the bounded box of `keys`. The measure is the
/// **doubled** area (`i128`, exact) — see [`DbmKeys::measure_zone`].
///
/// `observer`: narration channel (ADR-0100) — each visited node emits
/// an `Attr`/`Synthesized` event with the computed measure.
pub fn integrale_dbm_constante<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    root: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observer: Option<&dyn Observer>,
) -> Result<i128, PoolError> {
    let mut memo: HashMap<(NodeId, Dbm), i128> = HashMap::new();
    integrale_dbm(pool, root, region, keys, observer, &mut memo)
}

fn integrale_dbm<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    node: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observer: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Dbm), i128>,
) -> Result<i128, PoolError> {
    if let Some(total) = memo.get(&(node, region.clone())) {
        if let Some(obs) = observer {
            obs.on_event(&TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
                node,
                value: format!("measure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(node)?.clone() {
        Form::Terminal(true) => keys.measure_zone(region),
        Form::Terminal(false) => 0,
        Form::Node { arcs, .. } => {
            let mut sum = 0i128;
            for (zone, target) in &arcs {
                let sous_region = region.intersection(zone);
                if sous_region.est_empty() {
                    continue;
                }
                let contribution =
                    integrale_dbm(pool, *target, &sous_region, keys, observer, memo)?;
                sum += contribution;
            }
            sum
        }
    };
    if let Some(obs) = observer {
        obs.on_event(&TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node,
            value: format!("measure={total}"),
        });
    }
    memo.insert((node, region.clone()), total);
    Ok(total)
}

/// Builds a single-level diagram denoting the union of the given
/// `intervals` (arcs to `⊤`), the complement going to `⊥` — integral
/// instance-test: the measure of the union is the sum of the lengths.
/// **Precondition**: the intervals are pairwise disjoint (debug assertion)
/// — otherwise the integral would sum the individual lengths and
/// over-count the measure of the union.
pub fn compiler_union<P: Pool<Key = Interval, Payload = bool>>(
    pool: &mut P,
    anchor: lisage_algebra::interface::Anchor,
    intervals: &[Interval],
) -> NodeId {
    debug_assert!(
        intervals.windows(2).all(|pair| {
            let a = &pair[0];
            let b = &pair[1];
            a.est_empty() || b.est_empty() || a.bound_sup() < b.bound_inf()
        }),
        "compiler_union requires disjoint intervals (precondition)"
    );
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let arcs: BTreeMap<Interval, NodeId> =
        intervals.iter().map(|interval| (*interval, vrai)).collect();
    pool.intern(
        anchor,
        Form::Node {
            level: lisage_algebra::interface::Level(0),
            arcs,
        },
    )
}
