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
//! of the intervals leading to `⊤` — the differential against the cardinal
//! fold (sum of sizes) shows the count → measure passage.
//!
//! **DBM** variant (spec Q9 (ii), "polyhedral regions"): on DBM
//! region-keys with 2 variables, the Lebesgue measure is the **area** of
//! the zone ∩ box (`DbmKeys::mesure_zone`, returned doubled in `i128`), and
//! the integral of constant density 1 over the diagram is the area of the
//! denoted set — `integral(node, region) = Σ_{arcs Z → child}
//! integral(child, region ∩ Z)`.
//!
//! Narration (ADR-0100 decision 6): each recursive call emits an `Attr`
//! event (channel `Synthétise`) to the optional observer, with the computed
//! measure; the test pins these events.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{Form, NodeId, Pool, PoolError};

use crate::dbm::{Dbm, DbmKeys};
use crate::observer::{CanalAttr, Observer, TraceEvent};
use crate::region::Intervalle;

/// Lebesgue measure of an interval in the continuous interpretation:
/// `length([a, b]) = b − a`; the empty interval and the isolated point
/// have measure zero.
pub fn mesure_intervalle(intervalle: &Intervalle) -> u64 {
    if intervalle.est_vide() {
        0
    } else {
        intervalle
            .borne_sup()
            .saturating_sub(intervalle.borne_inf())
    }
}

/// Intersection of two intervals (set-theoretic meet): the empty interval
/// if disjoint.
pub fn intersection(a: &Intervalle, b: &Intervalle) -> Intervalle {
    if a.est_vide() || b.est_vide() {
        return crate::region::VIDE;
    }
    Intervalle::nouveau(
        a.borne_inf().max(b.borne_inf()),
        a.borne_sup().min(b.borne_sup()),
    )
}

/// Integral of the constant density 1 over the set denoted by the
/// 1D region-key diagram, restricted to the current `region`
/// (continuous interpretation: Lebesgue measure, saturated `u64`).
///
/// `observateur`: narration channel (ADR-0100) — each visited node emits
/// an `Attr`/`Synthétise` event with the computed measure.
pub fn integrale_constante<P: Pool<Key = Intervalle, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    region: &Intervalle,
    observateur: Option<&dyn Observer>,
) -> Result<u64, PoolError> {
    let mut memo: HashMap<(NodeId, Intervalle), u64> = HashMap::new();
    integrale(pool, racine, region, observateur, &mut memo)
}

fn integrale<P: Pool<Key = Intervalle, Payload = bool>>(
    pool: &P,
    noeud: NodeId,
    region: &Intervalle,
    observateur: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Intervalle), u64>,
) -> Result<u64, PoolError> {
    if let Some(total) = memo.get(&(noeud, *region)) {
        if let Some(obs) = observateur {
            obs.on_event(&TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                noeud,
                valeur: format!("mesure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(noeud)?.clone() {
        Form::Terminal(true) => mesure_intervalle(region),
        Form::Terminal(false) => 0,
        Form::Node { arcs, .. } => {
            let mut somme = 0u64;
            for (intervalle, cible) in &arcs {
                let sous_region = intersection(region, intervalle);
                if sous_region.est_vide() {
                    continue;
                }
                let contribution = integrale(pool, *cible, &sous_region, observateur, memo)?;
                somme = somme.saturating_add(contribution);
            }
            somme
        }
    };
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud,
            valeur: format!("mesure={total}"),
        });
    }
    memo.insert((noeud, *region), total);
    Ok(total)
}

/// Integral of the constant density 1 over the set denoted by the DBM
/// region-key diagram (2 variables), restricted to the current `region`,
/// intersected with the bounded box of `keys`. The measure is the
/// **doubled** area (`i128`, exact) — see [`DbmKeys::mesure_zone`].
///
/// `observateur`: narration channel (ADR-0100) — each visited node emits
/// an `Attr`/`Synthétise` event with the computed measure.
pub fn integrale_dbm_constante<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observateur: Option<&dyn Observer>,
) -> Result<i128, PoolError> {
    let mut memo: HashMap<(NodeId, Dbm), i128> = HashMap::new();
    integrale_dbm(pool, racine, region, keys, observateur, &mut memo)
}

fn integrale_dbm<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    noeud: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observateur: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Dbm), i128>,
) -> Result<i128, PoolError> {
    if let Some(total) = memo.get(&(noeud, region.clone())) {
        if let Some(obs) = observateur {
            obs.on_event(&TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                noeud,
                valeur: format!("mesure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(noeud)?.clone() {
        Form::Terminal(true) => keys.mesure_zone(region),
        Form::Terminal(false) => 0,
        Form::Node { arcs, .. } => {
            let mut somme = 0i128;
            for (zone, cible) in &arcs {
                let sous_region = region.intersection(zone);
                if sous_region.est_vide() {
                    continue;
                }
                let contribution =
                    integrale_dbm(pool, *cible, &sous_region, keys, observateur, memo)?;
                somme += contribution;
            }
            somme
        }
    };
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud,
            valeur: format!("mesure={total}"),
        });
    }
    memo.insert((noeud, region.clone()), total);
    Ok(total)
}

/// Builds a single-level diagram denoting the union of the given
/// `intervalles` (arcs to `⊤`), the complement going to `⊥` — integral
/// instance-test: the measure of the union is the sum of the lengths.
/// **Precondition**: the intervals are pairwise disjoint (debug assertion)
/// — otherwise the integral would sum the individual lengths and
/// over-count the measure of the union.
pub fn compiler_union<P: Pool<Key = Intervalle, Payload = bool>>(
    pool: &mut P,
    anchor: lisage_algebra::interface::Anchor,
    intervalles: &[Intervalle],
) -> NodeId {
    debug_assert!(
        intervalles.windows(2).all(|pair| {
            let a = &pair[0];
            let b = &pair[1];
            a.est_vide() || b.est_vide() || a.borne_sup() < b.borne_inf()
        }),
        "compiler_union requires disjoint intervals (precondition)"
    );
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let arcs: BTreeMap<Intervalle, NodeId> = intervalles
        .iter()
        .map(|intervalle| (*intervalle, vrai))
        .collect();
    pool.intern(
        anchor,
        Form::Node {
            level: lisage_algebra::interface::Level(0),
            arcs,
        },
    )
}
