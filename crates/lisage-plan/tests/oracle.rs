//! M3-3 tests (claim C4): analytical profitability oracle — predictive
//! model (domain size, density, encoding cost, downstream queries)
//! and differential guardrail (conforming in [predicted/2, 2 × predicted]).

use lisage_plan::oracle::{
    BORNE_INFERIEURE, BORNE_SUPERIEURE, ErreurOracle, Oracle, ParametresOracle, Rationnel,
    VerdictGardeFou,
};

/// M3-3: the rational is normalized (gcd) and arithmetic is exact.
#[test]
fn test_rationnel_normalise_et_exact() {
    assert_eq!(Rationnel::nouveau(2, 4), Rationnel::nouveau(1, 2));
    assert_eq!(Rationnel::nouveau(6, 3), Rationnel::entier(2));
    assert_eq!(Rationnel::nouveau(5, 0), Rationnel::nouveau(0, 1));
    let demi = Rationnel::nouveau(1, 2);
    let quart = Rationnel::nouveau(1, 4);
    assert_eq!(demi.multiplier(&quart), Rationnel::nouveau(1, 8));
    assert!(Rationnel::nouveau(1, 3).inferieur_ou_egal(&Rationnel::nouveau(1, 2)));
    assert!(!Rationnel::nouveau(2, 3).inferieur_ou_egal(&Rationnel::nouveau(1, 2)));
}

/// M3-3: the expected density is validated in [0, 1] at construction.
#[test]
fn test_parametres_densite_validee() {
    assert!(ParametresOracle::nouveau(10, Rationnel::nouveau(1, 2), 3, 4).is_ok());
    assert!(ParametresOracle::nouveau(10, Rationnel::nouveau(0, 1), 3, 4).is_ok());
    assert!(ParametresOracle::nouveau(10, Rationnel::entier(1), 3, 4).is_ok());
    assert_eq!(
        ParametresOracle::nouveau(10, Rationnel::nouveau(2, 1), 3, 4).err(),
        Some(ErreurOracle::DensiteHorsBornes)
    );
    assert_eq!(
        ParametresOracle::nouveau(10, Rationnel::nouveau(3, 2), 3, 4).err(),
        Some(ErreurOracle::DensiteHorsBornes)
    );
}

/// M3-3: the model predicts `1 + n·d` nodes and the corresponding total cost.
#[test]
fn test_prediction_taille_et_cout_total() {
    let params = ParametresOracle::nouveau(10, Rationnel::nouveau(1, 2), 3, 4).unwrap();
    let prediction = Oracle::predire(&params);
    assert_eq!(prediction.taille_predite, 6, "1 + 10·(1/2) nodes");
    assert_eq!(prediction.cout_total_predit, 6 * 3 * 4);

    // Null density: only the terminal. Full density: the whole chain.
    let nul = Oracle::predire(
        &ParametresOracle::nouveau(
            params.taille_domaine(),
            Rationnel::nouveau(0, 1),
            params.cout_encodage(),
            params.requetes_aval(),
        )
        .unwrap(),
    );
    assert_eq!(nul.taille_predite, 1);
    let plein = Oracle::predire(
        &ParametresOracle::nouveau(
            params.taille_domaine(),
            Rationnel::entier(1),
            params.cout_encodage(),
            params.requetes_aval(),
        )
        .unwrap(),
    );
    assert_eq!(plein.taille_predite, 11);
}

/// M3-3: the differential guardrail compares prediction and measurement
/// within a factor (C4 falsifiable out of bounds).
#[test]
fn test_garde_fou_differentiel() {
    // conforming: measure = predicted, and at bounds [predicted/2, 2 × predicted].
    assert!(Oracle::garde_fou(100, 100).est_conforme());
    assert!(Oracle::garde_fou(100, 50).est_conforme());
    assert!(Oracle::garde_fou(100, 200).est_conforme());
    // out of bounds: model falsified.
    assert!(!Oracle::garde_fou(100, 49).est_conforme());
    assert!(!Oracle::garde_fou(100, 201).est_conforme());
    // exact ratio.
    assert_eq!(
        VerdictGardeFou::nouveau(100, 150).ratio(),
        Rationnel::nouveau(3, 2)
    );
    assert!(BORNE_INFERIEURE.inferieur_ou_egal(&BORNE_SUPERIEURE));
    // predicted = 0 (null encoding cost or queries): conforming iff measure = 0.
    assert!(Oracle::garde_fou(0, 0).est_conforme());
    assert!(!Oracle::garde_fou(0, 1).est_conforme());
}

/// Property (deterministic LCG): the prediction is monotonic in density
/// and in downstream queries, and the guardrail is conforming when the
/// measurement equals the prediction.
#[test]
fn test_propriete_prediction_monotone_et_garde_fou() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _cas in 0..50 {
        let taille_domaine = 1 + suivant() % 100;
        let densite = Rationnel::nouveau(suivant() % 101, 100);
        let cout_encodage = suivant() % 10; // 0 possible: predicted can be null
        let requetes_aval = suivant() % 20;
        let params =
            ParametresOracle::nouveau(taille_domaine, densite, cout_encodage, requetes_aval)
                .unwrap();
        let prediction = Oracle::predire(&params);
        // Monotonic in density.
        let plus_dense = Oracle::predire(
            &ParametresOracle::nouveau(
                params.taille_domaine(),
                Rationnel::entier(1),
                params.cout_encodage(),
                params.requetes_aval(),
            )
            .unwrap(),
        );
        assert!(prediction.taille_predite <= plus_dense.taille_predite);
        // Monotonic in downstream queries.
        let plus_de_requetes = Oracle::predire(
            &ParametresOracle::nouveau(
                params.taille_domaine(),
                params.densite_attendue(),
                params.cout_encodage(),
                requetes_aval + 1,
            )
            .unwrap(),
        );
        assert!(prediction.cout_total_predit <= plus_de_requetes.cout_total_predit);
        // Guardrail conforming when measure = prediction.
        assert!(
            Oracle::garde_fou(prediction.cout_total_predit, prediction.cout_total_predit)
                .est_conforme()
        );
    }
}
