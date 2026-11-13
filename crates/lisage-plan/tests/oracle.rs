//! Tests for M3-3 (statement C4): analytic profitability oracle — predictive
//! model (domain size, density, encoding cost, downstream requests) and
//! differential guardrail (compliant within [predicted/2, 2 × predicted]).

use lisage_plan::oracle::{
    BOUND_INFERIEURE, BOUND_SUPERIEURE, ErrorOracle, Oracle, ParametersOracle, Rational,
    VerdictKeepsWild,
};

/// M3-3: the rational is normalized (gcd) and the arithmetic exact.
#[test]
fn test_rational_normalize_and_exact() {
    assert_eq!(Rational::new(2, 4), Rational::new(1, 2));
    assert_eq!(Rational::new(6, 3), Rational::entier(2));
    assert_eq!(Rational::new(5, 0), Rational::new(0, 1));
    let demi = Rational::new(1, 2);
    let quart = Rational::new(1, 4);
    assert_eq!(demi.multiplier(&quart), Rational::new(1, 8));
    assert!(Rational::new(1, 3).lower_or_equal(&Rational::new(1, 2)));
    assert!(!Rational::new(2, 3).lower_or_equal(&Rational::new(1, 2)));
}

/// M3-3: the expected density is validated in [0, 1] at construction.
#[test]
fn test_parameters_density_validated() {
    assert!(ParametersOracle::new(10, Rational::new(1, 2), 3, 4).is_ok());
    assert!(ParametersOracle::new(10, Rational::new(0, 1), 3, 4).is_ok());
    assert!(ParametersOracle::new(10, Rational::entier(1), 3, 4).is_ok());
    assert_eq!(
        ParametersOracle::new(10, Rational::new(2, 1), 3, 4).err(),
        Some(ErrorOracle::DensityOutBounds)
    );
    assert_eq!(
        ParametersOracle::new(10, Rational::new(3, 2), 3, 4).err(),
        Some(ErrorOracle::DensityOutBounds)
    );
}

/// M3-3: the model predicts `1 + n·d` nodes and the corresponding total cost.
#[test]
fn test_prediction_size_and_cost_total() {
    let params = ParametersOracle::new(10, Rational::new(1, 2), 3, 4).unwrap();
    let prediction = Oracle::predire(&params);
    assert_eq!(prediction.size_predicted, 6, "1 + 10·(1/2) nodes");
    assert_eq!(prediction.cost_total_predicted, 6 * 3 * 4);

    // Null density: terminal only. Full density: the whole chain.
    let null_node = Oracle::predire(
        &ParametersOracle::new(
            params.size_domain(),
            Rational::new(0, 1),
            params.cost_encoding(),
            params.requests_downstream(),
        )
        .unwrap(),
    );
    assert_eq!(null_node.size_predicted, 1);
    let full = Oracle::predire(
        &ParametersOracle::new(
            params.size_domain(),
            Rational::entier(1),
            params.cost_encoding(),
            params.requests_downstream(),
        )
        .unwrap(),
    );
    assert_eq!(full.size_predicted, 11);
}

/// M3-3: the differential guardrail compares prediction and measurement up to
/// a factor (C4 falsifiable out of bounds).
#[test]
fn test_keeps_wild_differential() {
    // Compliant: measurement = predicted, and at the bounds [predicted/2, 2 × predicted].
    assert!(Oracle::keeps_wild(100, 100).is_compliant());
    assert!(Oracle::keeps_wild(100, 50).is_compliant());
    assert!(Oracle::keeps_wild(100, 200).is_compliant());
    // Out of bounds: model falsified.
    assert!(!Oracle::keeps_wild(100, 49).is_compliant());
    assert!(!Oracle::keeps_wild(100, 201).is_compliant());
    // Exact ratio.
    assert_eq!(VerdictKeepsWild::new(100, 150).ratio(), Rational::new(3, 2));
    assert!(BOUND_INFERIEURE.lower_or_equal(&BOUND_SUPERIEURE));
    // predicted = 0 (encoding cost or requests zero): compliant iff measure = 0.
    assert!(Oracle::keeps_wild(0, 0).is_compliant());
    assert!(!Oracle::keeps_wild(0, 1).is_compliant());
}

/// Property (deterministic LCG): the prediction is increasing in density and
/// in downstream requests, and the guardrail is compliant when the
/// measurement equals the prediction.
#[test]
fn test_property_prediction_monotone_and_keeps_wild() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut graine = GRAINE_LCG;
    let mut next = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _cas in 0..50 {
        let size_domain = 1 + next() % 100;
        let density = Rational::new(next() % 101, 100);
        let cost_encoding = next() % 10; // 0 possible: predicted can be zero
        let requests_downstream = next() % 20;
        let params =
            ParametersOracle::new(size_domain, density, cost_encoding, requests_downstream)
                .unwrap();
        let prediction = Oracle::predire(&params);
        // Growth in density.
        let more_dense = Oracle::predire(
            &ParametersOracle::new(
                params.size_domain(),
                Rational::entier(1),
                params.cost_encoding(),
                params.requests_downstream(),
            )
            .unwrap(),
        );
        assert!(prediction.size_predicted <= more_dense.size_predicted);
        // Growth in downstream requests.
        let more_of_requests = Oracle::predire(
            &ParametersOracle::new(
                params.size_domain(),
                params.density_expected(),
                params.cost_encoding(),
                requests_downstream + 1,
            )
            .unwrap(),
        );
        assert!(prediction.cost_total_predicted <= more_of_requests.cost_total_predicted);
        // Guardrail compliant when measure = predicted.
        assert!(
            Oracle::keeps_wild(
                prediction.cost_total_predicted,
                prediction.cost_total_predicted
            )
            .is_compliant()
        );
    }
}
