//! M3-3 tests (claim C4): analytical profitability oracle — predictive
//! model (domain size, density, encoding cost, downstream queries)
//! and differential guardrail (conforming in [predicted/2, 2 × predicted]).

use lisage_plan::oracle::{
    BOUND_HIGHER, BOUND_LOWER, ErrorOracle, Oracle, OracleParameters, Rational, VerdictGuardrail,
};

/// M3-3: the rational is normalized (gcd) and arithmetic is exact.
#[test]
fn test_rational_normalise_and_exact() {
    assert_eq!(Rational::new(2, 4), Rational::new(1, 2));
    assert_eq!(Rational::new(6, 3), Rational::integer(2));
    assert_eq!(Rational::new(5, 0), Rational::new(0, 1));
    let half = Rational::new(1, 2);
    let quart = Rational::new(1, 4);
    assert_eq!(half.multiplier(&quart), Rational::new(1, 8));
    assert!(Rational::new(1, 3).inferieur_or_equals(&Rational::new(1, 2)));
    assert!(!Rational::new(2, 3).inferieur_or_equals(&Rational::new(1, 2)));
}

/// M3-3: the expected density is validated in [0, 1] at construction.
#[test]
fn test_parameters_density_validated() {
    assert!(OracleParameters::new(10, Rational::new(1, 2), 3, 4).is_ok());
    assert!(OracleParameters::new(10, Rational::new(0, 1), 3, 4).is_ok());
    assert!(OracleParameters::new(10, Rational::integer(1), 3, 4).is_ok());
    assert_eq!(
        OracleParameters::new(10, Rational::new(2, 1), 3, 4).err(),
        Some(ErrorOracle::DensityOutsideBounds)
    );
    assert_eq!(
        OracleParameters::new(10, Rational::new(3, 2), 3, 4).err(),
        Some(ErrorOracle::DensityOutsideBounds)
    );
}

/// M3-3: the model predicts `1 + n·d` nodes and the corresponding total cost.
#[test]
fn test_prediction_size_and_cost_total() {
    let params = OracleParameters::new(10, Rational::new(1, 2), 3, 4).unwrap();
    let prediction = Oracle::predict(&params);
    assert_eq!(prediction.size_predicted, 6, "1 + 10·(1/2) nodes");
    assert_eq!(prediction.cost_total_predicted, 6 * 3 * 4);

    // Null density: only the terminal. Full density: the whole chain.
    let zero = Oracle::predict(
        &OracleParameters::new(
            params.size_domain(),
            Rational::new(0, 1),
            params.cost_encoding(),
            params.requests_downstream(),
        )
        .unwrap(),
    );
    assert_eq!(zero.size_predicted, 1);
    let plein = Oracle::predict(
        &OracleParameters::new(
            params.size_domain(),
            Rational::integer(1),
            params.cost_encoding(),
            params.requests_downstream(),
        )
        .unwrap(),
    );
    assert_eq!(plein.size_predicted, 11);
}

/// M3-3: the differential guardrail compares prediction and measurement
/// within a factor (C4 falsifiable out of bounds).
#[test]
fn test_guardrail_differential() {
    // conforming: measure = predicted, and at bounds [predicted/2, 2 × predicted].
    assert!(Oracle::guardrail(100, 100).is_conforming());
    assert!(Oracle::guardrail(100, 50).is_conforming());
    assert!(Oracle::guardrail(100, 200).is_conforming());
    // out of bounds: model falsified.
    assert!(!Oracle::guardrail(100, 49).is_conforming());
    assert!(!Oracle::guardrail(100, 201).is_conforming());
    // exact ratio.
    assert_eq!(VerdictGuardrail::new(100, 150).ratio(), Rational::new(3, 2));
    assert!(BOUND_LOWER.inferieur_or_equals(&BOUND_HIGHER));
    // predicted = 0 (null encoding cost or queries): conforming iff measure = 0.
    assert!(Oracle::guardrail(0, 0).is_conforming());
    assert!(!Oracle::guardrail(0, 1).is_conforming());
}

/// Property (deterministic LCG): the prediction is monotonic in density
/// and in downstream queries, and the guardrail is conforming when the
/// measurement equals the prediction.
#[test]
fn test_property_prediction_monotone_and_guardrail() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const LCG_MULTIPLIER: u64 = 6364136223846793005;
    const LCG_INCREMENT: u64 = 1442695040888963407;

    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        seed >> 33
    };

    for _cas in 0..50 {
        let size_domain = 1 + next() % 100;
        let density = Rational::new(next() % 101, 100);
        let cost_encoding = next() % 10; // 0 possible: predicted can be null
        let requests_downstream = next() % 20;
        let params =
            OracleParameters::new(size_domain, density, cost_encoding, requests_downstream)
                .unwrap();
        let prediction = Oracle::predict(&params);
        // Monotonic in density.
        let plus_dense = Oracle::predict(
            &OracleParameters::new(
                params.size_domain(),
                Rational::integer(1),
                params.cost_encoding(),
                params.requests_downstream(),
            )
            .unwrap(),
        );
        assert!(prediction.size_predicted <= plus_dense.size_predicted);
        // Monotonic in downstream queries.
        let more_queries = Oracle::predict(
            &OracleParameters::new(
                params.size_domain(),
                params.density_expected(),
                params.cost_encoding(),
                requests_downstream + 1,
            )
            .unwrap(),
        );
        assert!(prediction.cost_total_predicted <= more_queries.cost_total_predicted);
        // Guardrail conforming when measure = prediction.
        assert!(
            Oracle::guardrail(
                prediction.cost_total_predicted,
                prediction.cost_total_predicted
            )
            .is_conforming()
        );
    }
}
