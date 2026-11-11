//! Tests de M3-3 (énoncé C4) : oracle de rentabilité analytique — modèle
//! prédictif (taille du domaine, densité, coût d'encodage, requêtes aval)
//! et garde-fou par différentiel (conforme dans [prédit/2, 2 × prédit]).

use lisage_plan::oracle::{
    BORNE_INFERIEURE, BORNE_SUPERIEURE, Oracle, ParametresOracle, Rationnel, VerdictGardeFou,
};

/// M3-3 : le rationnel est normalisé (pgcd) et l'arithmétique exacte.
#[test]
fn test_rationnel_normalise_et_exact() {
    assert_eq!(Rationnel::nouveau(2, 4), Rationnel { num: 1, den: 2 });
    assert_eq!(Rationnel::nouveau(6, 3), Rationnel::entier(2));
    assert_eq!(Rationnel::nouveau(5, 0), Rationnel { num: 0, den: 1 });
    let demi = Rationnel::nouveau(1, 2);
    let quart = Rationnel::nouveau(1, 4);
    assert_eq!(demi.multiplier(&quart), Rationnel::nouveau(1, 8));
    assert!(Rationnel::nouveau(1, 3).inferieur_ou_egal(&Rationnel::nouveau(1, 2)));
    assert!(!Rationnel::nouveau(2, 3).inferieur_ou_egal(&Rationnel::nouveau(1, 2)));
}

/// M3-3 : le modèle prédit `1 + n·d` nœuds et le coût total correspondant.
#[test]
fn test_prediction_taille_et_cout_total() {
    let params = ParametresOracle {
        taille_domaine: 10,
        densite_attendue: Rationnel::nouveau(1, 2),
        cout_encodage: 3,
        requetes_aval: 4,
    };
    let prediction = Oracle::predire(&params);
    assert_eq!(prediction.taille_predite, 6, "1 + 10·(1/2) nœuds");
    assert_eq!(prediction.cout_total_predit, 6 * 3 * 4);

    // Densité nulle : seul le terminal. Densité pleine : toute la chaîne.
    let nul = Oracle::predire(&ParametresOracle {
        densite_attendue: Rationnel::nouveau(0, 1),
        ..params
    });
    assert_eq!(nul.taille_predite, 1);
    let plein = Oracle::predire(&ParametresOracle {
        densite_attendue: Rationnel::entier(1),
        ..params
    });
    assert_eq!(plein.taille_predite, 11);
}

/// M3-3 : le garde-fou différentiel compare prédiction et mesure à un
/// facteur près (C4 falsifiable hors bornes).
#[test]
fn test_garde_fou_differentiel() {
    // conforme : mesure = prédit, et aux bornes [prédit/2, 2 × prédit].
    assert!(Oracle::garde_fou(100, 100).est_conforme());
    assert!(Oracle::garde_fou(100, 50).est_conforme());
    assert!(Oracle::garde_fou(100, 200).est_conforme());
    // hors bornes : modèle falsifié.
    assert!(!Oracle::garde_fou(100, 49).est_conforme());
    assert!(!Oracle::garde_fou(100, 201).est_conforme());
    // ratio exact.
    assert_eq!(
        VerdictGardeFou::nouveau(100, 150).ratio(),
        Rationnel::nouveau(3, 2)
    );
    assert!(BORNE_INFERIEURE.inferieur_ou_egal(&BORNE_SUPERIEURE));
}

/// Propriété (LCG déterministe) : la prédiction est croissante en densité
/// et en requêtes aval, et le garde-fou est conforme quand la mesure égale
/// la prédiction.
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
        let cout_encodage = 1 + suivant() % 10;
        let requetes_aval = 1 + suivant() % 20;
        let params = ParametresOracle {
            taille_domaine,
            densite_attendue: densite,
            cout_encodage,
            requetes_aval,
        };
        let prediction = Oracle::predire(&params);
        // Croissance en densité.
        let plus_dense = Oracle::predire(&ParametresOracle {
            densite_attendue: Rationnel::entier(1),
            ..params
        });
        assert!(prediction.taille_predite <= plus_dense.taille_predite);
        // Croissance en requêtes aval.
        let plus_de_requetes = Oracle::predire(&ParametresOracle {
            requetes_aval: requetes_aval + 1,
            ..params
        });
        assert!(prediction.cout_total_predit <= plus_de_requetes.cout_total_predit);
        // Garde-fou conforme quand mesure = prédiction.
        assert!(
            Oracle::garde_fou(prediction.cout_total_predit, prediction.cout_total_predit)
                .est_conforme()
        );
    }
}
