//! D5 — Semantic oracle over a trace: sampled differential checks at
//! operation and round boundaries.
//!
//! - [`SemanticOracle`] (D5-1): at each `OpEnd`, sample witness valuations
//!   and check that the real output denotation of each visited node matches
//!   the independent expected denotation; report any divergence with the
//!   operation fingerprint.
//! - [`FixpointOracle`] (D5-2): at each `Round`, verify the monotonicity
//!   X_k ⊆ X_{k+1} (any decrease = bug in τ or ⊕); render a saturation
//!   dashboard with active/modified levels, growth and convergence.

use std::collections::BTreeSet;

use lisage_core::NodeId;
use lisage_core::observer::TraceEvent;

/// D5-1: a denotation divergence detected at an operation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenotationDivergence {
    /// Position in the trace of the offending `OpEnd`.
    pub position: usize,
    /// Fingerprint of the operation (from the matching `OpStart`).
    pub fingerprint: u64,
    /// Witness valuation (set of atoms) that triggered the divergence.
    pub witness: BTreeSet<u64>,
    /// Input denotation value.
    pub input_value: bool,
    /// Output denotation value.
    pub output_value: bool,
}

/// D5-1: sampled semantic oracle. Walks the trace and, at each `OpEnd`,
/// evaluates a set of witness valuations against every `(input, output)`
/// node pair recorded by `VisiteSortie` inside the operation. The actual
/// output is the real denotation of the output node; the expected value
/// comes from an independent semantics closure. A `Filtre` with a faulty
/// predicate produces a wrong output node and is caught at the offending
/// operation, not at the end of the chain.
#[derive(Default)]
pub struct SemanticOracle {
    divergences: Vec<DenotationDivergence>,
}

impl SemanticOracle {
    /// Run the oracle over a trace. `domain_size` is the atom-domain size
    /// used for witness sampling. `denotation(node, mask)` evaluates the
    /// real denotation of a node under a witness bitmask (bit ℓ = atom ℓ).
    /// `semantics(fingerprint, input_node, input_value, witness)` returns
    /// the independent expected output denotation.
    pub fn run<D, S>(
        events: &[TraceEvent],
        domain_size: u64,
        mut denotation: D,
        mut semantics: S,
    ) -> Self
    where
        D: FnMut(NodeId, u64) -> bool,
        S: FnMut(u64, NodeId, bool, &BTreeSet<u64>) -> bool,
    {
        let mut oracle = SemanticOracle::default();
        let mut open_ops: Vec<(u64, Vec<(NodeId, NodeId)>)> = Vec::new();
        for (position, event) in events.iter().enumerate() {
            match event {
                TraceEvent::OpStart { fingerprint, .. } => {
                    open_ops.push((*fingerprint, Vec::new()));
                }
                TraceEvent::VisiteSortie { node, resultat, .. } => {
                    if let Some((_, pairs)) = open_ops.last_mut() {
                        pairs.push((*node, *resultat));
                    }
                }
                TraceEvent::OpEnd { .. } => {
                    if let Some((fingerprint, pairs)) = open_ops.pop() {
                        for (input_node, output_node) in pairs {
                            for witness in Self::sample_witnesses(domain_size) {
                                let mask = Self::witness_mask(&witness);
                                let input_value = denotation(input_node, mask);
                                let actual = denotation(output_node, mask);
                                let expected =
                                    semantics(fingerprint, input_node, input_value, &witness);
                                if actual != expected {
                                    oracle.divergences.push(DenotationDivergence {
                                        position,
                                        fingerprint,
                                        witness: witness.clone(),
                                        input_value,
                                        output_value: actual,
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        oracle
    }

    /// Bitmask of a witness valuation: bit ℓ set iff atom ℓ is in the set.
    fn witness_mask(witness: &BTreeSet<u64>) -> u64 {
        witness
            .iter()
            .fold(0u64, |mask, atom| mask | (1u64 << atom))
    }

    /// Generate witness valuations for a domain of the given size: the
    /// empty set, every singleton, and the full set.
    fn sample_witnesses(domain_size: u64) -> Vec<BTreeSet<u64>> {
        let mut witnesses = vec![BTreeSet::new()];
        for atom in 0..domain_size {
            let mut singleton = BTreeSet::new();
            singleton.insert(atom);
            witnesses.push(singleton);
        }
        let full: BTreeSet<u64> = (0..domain_size).collect();
        witnesses.push(full);
        witnesses
    }

    /// All divergences detected.
    pub fn divergences(&self) -> &[DenotationDivergence] {
        &self.divergences
    }

    /// True when no divergence was detected.
    pub fn is_clean(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// D5-2: a fixpoint monotonicity violation at a round boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonotonicityViolation {
    /// Round number (1-based) where the decrease was detected.
    pub round: u32,
    /// Position in the trace of the offending `Round` event.
    pub position: usize,
    /// Size before the round (node count of `avant`).
    pub size_before: u64,
    /// Size after the round (node count of `apres`).
    pub size_after: u64,
}

/// D5-2: one round's dashboard entry.
#[derive(Debug, Clone)]
pub struct RoundDashboard {
    /// Round number (1-based).
    pub round: u32,
    /// Active levels during the round.
    pub active_levels: Vec<u32>,
    /// Levels modified by the round.
    pub modified_levels: Vec<u32>,
    /// Pool growth in nodes during the round.
    pub growth: i64,
    /// True when the round reached a fixpoint (after == before).
    pub converged: bool,
}

/// D5-2: fixpoint oracle. Walks the trace and, at each `Round`, checks
/// that the fixpoint grows monotonically (X_k ⊆ X_{k+1}); any decrease
/// is a bug in τ or ⊕. Also renders the saturation dashboard.
#[derive(Default)]
pub struct FixpointOracle {
    violations: Vec<MonotonicityViolation>,
    dashboard: Vec<RoundDashboard>,
}

impl FixpointOracle {
    /// Run the oracle over a trace. `node_size` maps a `NodeId` to its
    /// denotation size (number of atoms in the represented set).
    pub fn run<F>(events: &[TraceEvent], mut node_size: F) -> Self
    where
        F: FnMut(NodeId) -> u64,
    {
        let mut oracle = FixpointOracle::default();
        for (position, event) in events.iter().enumerate() {
            if let TraceEvent::Round {
                numero,
                niveaux_actifs,
                niveaux_modifies,
                avant,
                apres,
                croissance,
                ..
            } = event
            {
                let size_before = node_size(*avant);
                let size_after = node_size(*apres);
                let converged = avant == apres;
                if !converged && size_after < size_before {
                    oracle.violations.push(MonotonicityViolation {
                        round: *numero,
                        position,
                        size_before,
                        size_after,
                    });
                }
                oracle.dashboard.push(RoundDashboard {
                    round: *numero,
                    active_levels: niveaux_actifs.clone(),
                    modified_levels: niveaux_modifies.clone(),
                    growth: *croissance,
                    converged,
                });
            }
        }
        oracle
    }

    /// All monotonicity violations detected.
    pub fn violations(&self) -> &[MonotonicityViolation] {
        &self.violations
    }

    /// The saturation dashboard, one entry per round.
    pub fn dashboard(&self) -> &[RoundDashboard] {
        &self.dashboard
    }

    /// True when no violation was detected.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    /// Render the dashboard as text lines.
    pub fn render_dashboard(&self) -> Vec<String> {
        self.dashboard
            .iter()
            .map(|entry| {
                let marker = if entry.converged { "FIXPOINT" } else { "" };
                format!(
                    "round {:>3} | active: {:?} | modified: {:?} | growth: {:+} | {}",
                    entry.round, entry.active_levels, entry.modified_levels, entry.growth, marker
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Domain size for the synthetic traces below.
    const DOMAIN_SIZE: u64 = 4;

    fn op_start(fingerprint: u64) -> TraceEvent {
        TraceEvent::OpStart {
            fingerprint,
            span: None,
        }
    }

    fn op_end() -> TraceEvent {
        TraceEvent::OpEnd { span: None }
    }

    fn visite_sortie(node: NodeId, resultat: NodeId) -> TraceEvent {
        TraceEvent::VisiteSortie {
            node,
            resultat,
            span: None,
        }
    }

    /// Denotation stub: `NodeId(0)` is true, every other node is false.
    fn stub_denotation(node: NodeId, _mask: u64) -> bool {
        node == NodeId(0)
    }

    #[test]
    fn test_semantic_oracle_clean() {
        // Identity operation: output node == input node, identity semantics.
        let events = vec![op_start(42), visite_sortie(NodeId(0), NodeId(0)), op_end()];
        let oracle = SemanticOracle::run(
            &events,
            DOMAIN_SIZE,
            stub_denotation,
            |_fingerprint, _input_node, input_value, _witness| input_value,
        );
        assert!(oracle.is_clean());
    }

    #[test]
    fn test_semantic_oracle_catches_faulty_filter() {
        // The engine produced the wrong output node (NodeId(1), denotation
        // false) where the semantics expects the input denotation (true):
        // a faulty predicate is caught at the offending operation.
        let events = vec![op_start(42), visite_sortie(NodeId(0), NodeId(1)), op_end()];
        let oracle = SemanticOracle::run(
            &events,
            DOMAIN_SIZE,
            stub_denotation,
            |_fingerprint, _input_node, input_value, _witness| input_value,
        );
        assert!(!oracle.is_clean());
        assert_eq!(oracle.divergences().len(), 6); // 6 witnesses, all diverge
        assert_eq!(oracle.divergences()[0].fingerprint, 42);
        assert!(!oracle.divergences()[0].output_value);
    }

    #[test]
    fn test_semantic_oracle_witness_sampling() {
        // Divergence only on the full-set witness: the actual output is
        // false, the expected value is true exactly for the full set.
        let events = vec![op_start(99), visite_sortie(NodeId(0), NodeId(1)), op_end()];
        let oracle = SemanticOracle::run(
            &events,
            DOMAIN_SIZE,
            stub_denotation,
            |_fingerprint, _input_node, _input_value, witness| witness.len() == 4,
        );
        assert_eq!(oracle.divergences().len(), 1);
        assert_eq!(oracle.divergences()[0].witness.len(), 4);
    }

    #[test]
    fn test_fixpoint_oracle_clean() {
        // Two rounds, monotonically growing.
        let events = vec![
            TraceEvent::Round {
                numero: 1,
                niveaux_actifs: vec![0],
                niveaux_modifies: vec![0],
                avant: NodeId(0),
                image: NodeId(1),
                apres: NodeId(1),
                croissance: 5,
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
            NodeId(0) => 3,
            NodeId(1) => 5,
            _ => 0,
        });
        assert!(oracle.is_clean());
        assert_eq!(oracle.dashboard().len(), 2);
        assert!(oracle.dashboard()[1].converged);
        let lines = oracle.render_dashboard();
        assert!(lines[1].contains("FIXPOINT"));
    }

    #[test]
    fn test_fixpoint_oracle_catches_decrease() {
        // Round 2 shrinks (bug in τ): size 5 → 3.
        let events = vec![TraceEvent::Round {
            numero: 2,
            niveaux_actifs: vec![0],
            niveaux_modifies: vec![0],
            avant: NodeId(1),
            image: NodeId(2),
            apres: NodeId(2),
            croissance: -2,
            span: None,
        }];
        let oracle = FixpointOracle::run(&events, |node| match node {
            NodeId(1) => 5,
            NodeId(2) => 3,
            _ => 0,
        });
        assert!(!oracle.is_clean());
        assert_eq!(oracle.violations().len(), 1);
        assert_eq!(oracle.violations()[0].round, 2);
        assert_eq!(oracle.violations()[0].size_before, 5);
        assert_eq!(oracle.violations()[0].size_after, 3);
    }
}
