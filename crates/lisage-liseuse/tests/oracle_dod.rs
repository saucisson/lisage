//! D5 DoD integration tests:
//! - D5-1: a `Filter` with a faulty predicate is caught at the offending
//!   operation of a chain, not at the end. The oracle evaluates the REAL
//!   denotation of the output node (via `bdd::evaluate` on the real pool)
//!   and compares it against an independent model of τ.
//! - D5-2: an injected not-monotonic τ is caught at the faulty round,
//!   not at the end.

use std::collections::BTreeSet;

use lisage_core::arena::Arena;
use lisage_core::observer::TraceEvent;
use lisage_core::schema::{ABSENT, PRESENT, ReferenceController};
use lisage_core::{Form, NodeId, Pool};
use lisage_liseuse::debugger::standard_scenario_with_pool;
use lisage_liseuse::oracle::{FixpointOracle, SemanticOracle};

/// Domain size of the standard scenario used below.
const DOMAIN_SIZE: u64 = 4;

/// Real denotation: walk the real pool forms under the witness bitmask
/// (bit ℓ = atom ℓ). A missing arc denotes the false terminal (BDD
/// reduction), so the walk returns `false` instead of failing.
fn real_denotation(pool: &Arena<ReferenceController>) -> impl FnMut(NodeId, u64) -> bool + '_ {
    move |root, mask| {
        let mut node = root;
        loop {
            match pool.resolve(node).expect("node resolvable in the pool") {
                Form::Terminal(payload) => return payload,
                Form::Node { level, arcs } => {
                    let bit = (mask >> level.0) & 1;
                    match arcs.get(&bit) {
                        Some(target) => node = *target,
                        None => return false,
                    }
                }
            }
        }
    }
}

/// Naive aggregation model: mirrors the engine's `phi_out` aggregation —
/// the maximum of `level + 1` over every PRESENT arc, recursively — by a
/// plain form walk independent of the engine's transduction. Returns
/// `None` for the empty family.
fn naive_union_max(pool: &Arena<ReferenceController>, root: NodeId) -> Option<u64> {
    fn walk(pool: &Arena<ReferenceController>, node: NodeId) -> Option<u64> {
        match pool.resolve(node).expect("node resolvable in the pool") {
            Form::Terminal(true) => Some(0),
            Form::Terminal(false) => None,
            Form::Node { level, arcs } => {
                let present = arcs
                    .get(&PRESENT)
                    .and_then(|target| walk(pool, *target))
                    .map(|max| max.max(level.0 as u64 + 1));
                let absent = arcs.get(&ABSENT).and_then(|target| walk(pool, *target));
                match (present, absent) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (Some(a), None) => Some(a),
                    (None, b) => b,
                }
            }
        }
    }
    walk(pool, root)
}

/// Independent model of the standard chain τ: the output family is the set
/// of valuations containing atom `m`, where `m` is the aggregation
/// `max(level + 1)` over the PRESENT arcs of the input form (the same rule
/// as the engine's `phi_out`, recomputed here by a naive form walk). The
/// expected output denotation under `witness` is therefore
/// `witness.contains(&m)`, false when `m` leaves the domain. Terminal
/// nodes are fixed points of the visit, so the expected value is the
/// input value.
fn chain_semantics(
    pool: &Arena<ReferenceController>,
) -> impl FnMut(u64, NodeId, bool, &BTreeSet<u64>) -> bool + '_ {
    move |_fingerprint, input_node, input_value, witness| match pool
        .resolve(input_node)
        .expect("node resolvable in the pool")
    {
        Form::Terminal(_) => input_value,
        Form::Node { .. } => match naive_union_max(pool, input_node) {
            Some(max) => max < DOMAIN_SIZE && witness.contains(&max),
            None => false,
        },
    }
}

/// Synthetic family model for the D5-2 traces below: each `NodeId` maps to
/// a bitmask of witness valuations (bit ℓ = valuation {ℓ}). Inclusion is
/// bitmask inclusion.
fn family_of(node: NodeId) -> u8 {
    match node {
        NodeId(0) => 0b0000,
        NodeId(1) => 0b0001,
        NodeId(2) => 0b0011,
        NodeId(3) => 0b0001,
        _ => 0,
    }
}

fn family_included(before: NodeId, after: NodeId) -> bool {
    family_of(before) & !family_of(after) == 0
}

/// D5-1 DoD: the semantic oracle over a real standard scenario trace
/// (saturate chain τ(X) = {{max(X)+1}}) reports no divergence when the
/// semantics model is the correct τ — the actual output denotations come
/// from `bdd::evaluate` on the real pool.
#[test]
fn test_d5_1_real_scenario_clean() {
    let (events, pool) = standard_scenario_with_pool(DOMAIN_SIZE);
    let oracle = SemanticOracle::run(
        &events,
        DOMAIN_SIZE,
        real_denotation(&pool),
        chain_semantics(&pool),
    );
    assert!(
        oracle.is_clean(),
        "standard scenario must be semantically clean, got {:?}",
        oracle.divergences()
    );
}

/// D5-1 DoD: a faulty predicate (off-by-one: τ'(X) = {{max(X)}} instead of
/// {{max(X)+1}}) is caught at the offending operation — every divergence is
/// reported at an `OpEnd` position carrying the operation fingerprint.
#[test]
fn test_d5_1_faulty_predicate_caught() {
    let (events, pool) = standard_scenario_with_pool(DOMAIN_SIZE);
    let faulty_semantics = |_fingerprint: u64,
                            input_node: NodeId,
                            input_value: bool,
                            witness: &BTreeSet<u64>|
     -> bool {
        match pool
            .resolve(input_node)
            .expect("node resolvable in the pool")
        {
            Form::Terminal(_) => input_value,
            Form::Node { .. } => match naive_union_max(&pool, input_node) {
                Some(max) => max >= 1 && witness.contains(&(max - 1)),
                None => false,
            },
        }
    };
    let oracle = SemanticOracle::run(
        &events,
        DOMAIN_SIZE,
        real_denotation(&pool),
        faulty_semantics,
    );
    assert!(!oracle.is_clean(), "faulty predicate must be caught");
    let fingerprints: BTreeSet<u64> = oracle.divergences().iter().map(|d| d.fingerprint).collect();
    assert_eq!(
        fingerprints.len(),
        1,
        "all divergences belong to the single chain operation"
    );
    for divergence in oracle.divergences() {
        assert!(divergence.position < events.len());
        assert!(matches!(
            events[divergence.position],
            TraceEvent::OpEnd { .. }
        ));
    }
}

/// D5-2 DoD: a not-monotonic τ injected at round 3 is caught at round 3,
/// not at the end. We build a synthetic trace where round 3 shrinks.
#[test]
fn test_d5_2_not_monotonic_tau_caught_at_faulty_round() {
    // Rounds 1-2 grow normally, round 3 shrinks (bug), round 4 would grow again.
    let events = vec![
        TraceEvent::Round {
            number: 1,
            active_levels: vec![0, 1],
            modified_levels: vec![0],
            before: NodeId(0),
            image: NodeId(1),
            after: NodeId(1),
            growth: 3,
            span: None,
        },
        TraceEvent::Round {
            number: 2,
            active_levels: vec![0, 1],
            modified_levels: vec![1],
            before: NodeId(1),
            image: NodeId(2),
            after: NodeId(2),
            growth: 2,
            span: None,
        },
        // Faulty round: the family shrinks from {{0},{1}} back to {{0}}.
        TraceEvent::Round {
            number: 3,
            active_levels: vec![0, 1],
            modified_levels: vec![0, 1],
            before: NodeId(2),
            image: NodeId(3),
            after: NodeId(3),
            growth: -3,
            span: None,
        },
        TraceEvent::Round {
            number: 4,
            active_levels: vec![0],
            modified_levels: vec![],
            before: NodeId(3),
            image: NodeId(3),
            after: NodeId(3),
            growth: 0,
            span: None,
        },
    ];
    // Rounds 1-2 grow by inclusion (∅ ⊆ {{0}} ⊆ {{0},{1}}), round 3
    // shrinks back to {{0}} — caught at round 3.
    let oracle = FixpointOracle::run(&events, family_included);
    assert_eq!(oracle.violations().len(), 1, "exactly one violation");
    assert_eq!(
        oracle.violations()[0].round,
        3,
        "caught at the faulty round, not at the end"
    );
    assert_eq!(oracle.dashboard().len(), 4);
    assert!(oracle.dashboard()[3].converged, "round 4 is the fixpoint");
    let lines = oracle.render_dashboard();
    assert_eq!(lines.len(), 4);
    assert!(lines[3].contains("FIXPOINT"));
}

/// D5-2: a clean monotonic trace has no violations and the dashboard
/// marks the converged round.
#[test]
fn test_d5_2_clean_monotonic() {
    let events = vec![
        TraceEvent::Round {
            number: 1,
            active_levels: vec![0],
            modified_levels: vec![0],
            before: NodeId(0),
            image: NodeId(1),
            after: NodeId(1),
            growth: 2,
            span: None,
        },
        TraceEvent::Round {
            number: 2,
            active_levels: vec![0],
            modified_levels: vec![],
            before: NodeId(1),
            image: NodeId(1),
            after: NodeId(1),
            growth: 0,
            span: None,
        },
    ];
    let oracle = FixpointOracle::run(&events, family_included);
    assert!(oracle.is_clean());
    assert!(oracle.dashboard()[1].converged);
}

/// D5-2: dashboard renders active and modified levels.
#[test]
fn test_d5_2_dashboard_format() {
    let events = vec![TraceEvent::Round {
        number: 7,
        active_levels: vec![0, 2, 5],
        modified_levels: vec![2],
        before: NodeId(0),
        image: NodeId(0),
        after: NodeId(0),
        growth: 0,
        span: None,
    }];
    let oracle = FixpointOracle::run(&events, family_included);
    let lines = oracle.render_dashboard();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("round   7"));
    assert!(lines[0].contains("[0, 2, 5]"));
    assert!(lines[0].contains("FIXPOINT"));
}
