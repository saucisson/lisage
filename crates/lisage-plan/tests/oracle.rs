//! M3-3 tests (claim C4): analytical profitability oracle — predictive
//! model (domain size, density, encoding cost, downstream queries)
//! and differential guardrail (conforming in [predicted/2, 2 × predicted]).

use lisage_plan::oracle::{
    BOUND_INFERIEURE, BOUND_SUPERIEURE, ErrorOracle, Oracle, ParametresOracle, Rationnel,
    VerdictGardeFou,
};

/// M3-3: the rational is normalized (gcd) and arithmetic is exact.
#[test]
fn test_rationnel_normalise_et_exact() {
    assert_eq!(Rationnel::new(2, 4), Rationnel::new(1, 2));
    assert_eq!(Rationnel::new(6, 3), Rationnel::entier(2));
    assert_eq!(Rationnel::new(5, 0), Rationnel::new(0, 1));
    let demi = Rationnel::new(1, 2);
    let quart = Rationnel::new(1, 4);
    assert_eq!(demi.multiplier(&quart), Rationnel::new(1, 8));
    assert!(Rationnel::new(1, 3).inferieur_ou_egal(&Rationnel::new(1, 2)));
    assert!(!Rationnel::new(2, 3).inferieur_ou_egal(&Rationnel::new(1, 2)));
}

/// M3-3: the expected density is validated in [0, 1] at construction.
#[test]
fn test_parametres_densite_validee() {
    assert!(ParametresOracle::new(10, Rationnel::new(1, 2), 3, 4).is_ok());
    assert!(ParametresOracle::new(10, Rationnel::new(0, 1), 3, 4).is_ok());
    assert!(ParametresOracle::new(10, Rationnel::entier(1), 3, 4).is_ok());
    assert_eq!(
        ParametresOracle::new(10, Rationnel::new(2, 1), 3, 4).err(),
        Some(ErrorOracle::DensiteHorsBounds)
    );
    assert_eq!(
        ParametresOracle::new(10, Rationnel::new(3, 2), 3, 4).err(),
        Some(ErrorOracle::DensiteHorsBounds)
    );
}

/// M3-3: the model predicts `1 + n·d` nodes and the corresponding total cost.
#[test]
fn test_prediction_size_et_cout_total() {
    let params = ParametresOracle::new(10, Rationnel::new(1, 2), 3, 4).unwrap();
    let prediction = Oracle::predire(&params);
    assert_eq!(prediction.size_predite, 6, "1 + 10·(1/2) nodes");
    assert_eq!(prediction.cout_total_predit, 6 * 3 * 4);

    // Null density: only the terminal. Full density: the whole chain.
    let nul = Oracle::predire(
        &ParametresOracle::new(
            params.size_domain(),
            Rationnel::new(0, 1),
            params.cout_encodage(),
            params.requetes_aval(),
        )
        .unwrap(),
    );
    assert_eq!(nul.size_predite, 1);
    let plein = Oracle::predire(
        &ParametresOracle::new(
            params.size_domain(),
            Rationnel::entier(1),
            params.cout_encodage(),
            params.requetes_aval(),
        )
        .unwrap(),
    );
    assert_eq!(plein.size_predite, 11);
}

/// M3-3: the differential guardrail compares prediction and measurement
/// within a factor (C4 falsifiable out of bounds).
#[test]
fn test_garde_fou_differentiel() {
    // conforming: measure = predicted, and at bounds [predicted/2, 2 × predicted].
    assert!(Oracle::garde_fou(100, 100).est_conforming());
    assert!(Oracle::garde_fou(100, 50).est_conforming());
    assert!(Oracle::garde_fou(100, 200).est_conforming());
    // out of bounds: model falsified.
    assert!(!Oracle::garde_fou(100, 49).est_conforming());
    assert!(!Oracle::garde_fou(100, 201).est_conforming());
    // exact ratio.
    assert_eq!(VerdictGardeFou::new(100, 150).ratio(), Rationnel::new(3, 2));
    assert!(BOUND_INFERIEURE.inferieur_ou_egal(&BOUND_SUPERIEURE));
    // predicted = 0 (null encoding cost or queries): conforming iff measure = 0.
    assert!(Oracle::garde_fou(0, 0).est_conforming());
    assert!(!Oracle::garde_fou(0, 1).est_conforming());
}

/// Property (deterministic LCG): the prediction is monotonic in density
/// and in downstream queries, and the guardrail is conforming when the
/// measurement equals the prediction.
#[test]
fn test_propriete_prediction_monotone_et_garde_fou() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed >> 33
    };

    for _cas in 0..50 {
        let size_domain = 1 + next() % 100;
        let densite = Rationnel::new(next() % 101, 100);
        let cout_encodage = next() % 10; // 0 possible: predicted can be null
        let requetes_aval = next() % 20;
        let params =
            ParametresOracle::new(size_domain, densite, cout_encodage, requetes_aval).unwrap();
        let prediction = Oracle::predire(&params);
        // Monotonic in density.
        let plus_dense = Oracle::predire(
            &ParametresOracle::new(
                params.size_domain(),
                Rationnel::entier(1),
                params.cout_encodage(),
                params.requetes_aval(),
            )
            .unwrap(),
        );
        assert!(prediction.size_predite <= plus_dense.size_predite);
        // Monotonic in downstream queries.
        let plus_de_requetes = Oracle::predire(
            &ParametresOracle::new(
                params.size_domain(),
                params.densite_expectede(),
                params.cout_encodage(),
                requetes_aval + 1,
            )
            .unwrap(),
        );
        assert!(prediction.cout_total_predit <= plus_de_requetes.cout_total_predit);
        // Guardrail conforming when measure = prediction.
        assert!(
            Oracle::garde_fou(prediction.cout_total_predit, prediction.cout_total_predit)
                .est_conforming()
        );
    }
}
