//! D5 DoD integration tests:
//! - D5-1: a `Filtre` with a faulty predicate is caught at the offending
//!   operation of a chain, not at the end.
//! - D5-2: an injected non-monotonic τ is caught at the faulty round,
//!   not at the end.

use lisage_core::observer::TraceEvent;
use lisage_liseuse::debugger::standard_scenario;
use lisage_liseuse::oracle::{FixpointOracle, SemanticOracle};

/// D5-1 DoD: the semantic oracle over a real standard scenario trace
/// (saturer chain τ(X) = {{max(X)+1}}) reports no divergence when the
/// evaluator is the identity (the chain is correct).
#[test]
fn test_d5_1_real_scenario_clean() {
    let events = standard_scenario(4);
    // The standard scenario is a correct chain; an identity evaluator
    // (expected output == input denotation) must find no divergence.
    let oracle = SemanticOracle::run(&events, |_fp, _witness, input| input);
    assert!(
        oracle.is_clean(),
        "standard scenario must be semantically clean, got {:?}",
        oracle.divergences()
    );
}

/// D5-1 DoD: a faulty predicate (flips output) on the standard scenario
/// is caught at the offending operation.
#[test]
fn test_d5_1_faulty_predicate_caught() {
    let events = standard_scenario(4);
    // Faulty evaluator: expected = NOT input → every op diverges.
    let oracle = SemanticOracle::run(&events, |_fp, _witness, input| !input);
    assert!(!oracle.is_clean(), "faulty predicate must be caught");
    // Every divergence is at an OpEnd position within the trace.
    for divergence in oracle.divergences() {
        assert!(divergence.position < events.len());
        assert!(matches!(
            events[divergence.position],
            TraceEvent::OpEnd { .. }
        ));
    }
}

/// D5-2 DoD: a non-monotonic τ injected at round 3 is caught at round 3,
/// not at the end. We build a synthetic trace where round 3 shrinks.
#[test]
fn test_d5_2_non_monotonic_tau_caught_at_faulty_round() {
    use lisage_core::NodeId;
    // Rounds 1-2 grow normally, round 3 shrinks (bug), round 4 would grow again.
    let events = vec![
        TraceEvent::Round {
            numero: 1,
            niveaux_actifs: vec![0, 1],
            niveaux_modifies: vec![0],
            avant: NodeId(0),
            image: NodeId(1),
            apres: NodeId(1),
            croissance: 3,
            span: None,
        },
        TraceEvent::Round {
            numero: 2,
            niveaux_actifs: vec![0, 1],
            niveaux_modifies: vec![1],
            avant: NodeId(1),
            image: NodeId(2),
            apres: NodeId(2),
            croissance: 2,
            span: None,
        },
        // Faulty round: size shrinks from 5 to 2.
        TraceEvent::Round {
            numero: 3,
            niveaux_actifs: vec![0, 1],
            niveaux_modifies: vec![0, 1],
            avant: NodeId(2),
            image: NodeId(3),
            apres: NodeId(3),
            croissance: -3,
            span: None,
        },
        TraceEvent::Round {
            numero: 4,
            niveaux_actifs: vec![0],
            niveaux_modifies: vec![],
            avant: NodeId(3),
            image: NodeId(3),
            apres: NodeId(3),
            croissance: 0,
            span: None,
        },
    ];
    // node_size: NodeId(0)=1, NodeId(1)=4, NodeId(2)=5, NodeId(3)=2
    let oracle = FixpointOracle::run(&events, |node| match node {
        NodeId(0) => 1,
        NodeId(1) => 4,
        NodeId(2) => 5,
        NodeId(3) => 2,
        _ => 0,
    });
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
    use lisage_core::NodeId;
    let events = vec![
        TraceEvent::Round {
            numero: 1,
            niveaux_actifs: vec![0],
            niveaux_modifies: vec![0],
            avant: NodeId(0),
            image: NodeId(1),
            apres: NodeId(1),
            croissance: 2,
            span: None,
        },
        TraceEvent::Round {
            numero: 2,
            niveaux_actifs: vec![0],
            niveaux_modifies: vec![],
            avant: NodeId(1),
            image: NodeId(1),
            apres: NodeId(1),
            croissance: 0,
            span: None,
        },
    ];
    let oracle = FixpointOracle::run(&events, |node| match node {
        NodeId(0) => 1,
        NodeId(1) => 3,
        _ => 0,
    });
    assert!(oracle.is_clean());
    assert!(oracle.dashboard()[1].converged);
}

/// D5-1: witness sampling covers empty, singletons and full set.
#[test]
fn test_d5_1_witness_sampling() {
    let events = vec![
        TraceEvent::OpStart {
            fingerprint: 99,
            span: None,
        },
        TraceEvent::OpEnd { span: None },
    ];
    // Evaluator that only diverges on the full set {0,1,2,3}.
    let oracle = SemanticOracle::run(
        &events,
        |_fp, witness, input| {
            if witness.len() == 4 { !input } else { input }
        },
    );
    assert_eq!(oracle.divergences().len(), 1);
    assert_eq!(oracle.divergences()[0].witness.len(), 4);
}

/// D5-2: dashboard renders active and modified levels.
#[test]
fn test_d5_2_dashboard_format() {
    use lisage_core::NodeId;
    let events = vec![TraceEvent::Round {
        numero: 7,
        niveaux_actifs: vec![0, 2, 5],
        niveaux_modifies: vec![2],
        avant: NodeId(0),
        image: NodeId(0),
        apres: NodeId(0),
        croissance: 0,
        span: None,
    }];
    let oracle = FixpointOracle::run(&events, |_| 1);
    let lines = oracle.render_dashboard();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("round   7"));
    assert!(lines[0].contains("[0, 2, 5]"));
    assert!(lines[0].contains("FIXPOINT"));
}
