//! Tests du vtree comme ordre de première classe (M5-3, C6) : ordre
//! canonique = parcours plat, contraintes gauche-avant-droite vérifiées
//! à la construction de `Order`, π valide adopté, π violant rejeté.

use lisage_algebra::vtree::{ErreurVtree, Vtree};
use lisage_lang::Atom;
use lisage_plan::ordre::{ErreurOrdre, Order};

/// Vtree de test : (x0, x1) à gauche, (x2, x3) à droite.
fn vtree_test() -> Vtree {
    Vtree::noeud(
        Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
        Vtree::noeud(Vtree::feuille(Atom(2)), Vtree::feuille(Atom(3))),
    )
}

/// Ordre canonique du vtree = parcours plat gauche-avant-droite.
#[test]
fn test_ordre_canonique_vtree() {
    let ordre = Order::canonique(&vtree_test());
    assert_eq!(ordre.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// Un vtree induit un ordre total (contrainte gauche-avant-droite à chaque
/// nœud interne) : l'ordre plat est la seule permutation valide, adoptée.
#[test]
fn test_pi_valide_adopte() {
    let vtree = vtree_test();
    let ordre = Order::nouveau(&vtree, vtree.ordre_plat()).unwrap();
    assert_eq!(ordre.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// Une autre forme de vtree induit un autre ordre canonique : (x2, (x0, x1))
/// donne l'ordre plat [2, 0, 1], et π = [0, 1, 2] viole la contrainte 2 < 0.
#[test]
fn test_autre_vtree_autre_ordre() {
    let vtree = Vtree::noeud(
        Vtree::feuille(Atom(2)),
        Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
    );
    let ordre = Order::canonique(&vtree);
    assert_eq!(ordre.sequence(), &[Atom(2), Atom(0), Atom(1)]);
    let resultat = Order::nouveau(&vtree, vec![Atom(0), Atom(1), Atom(2)]);
    assert_eq!(
        resultat.unwrap_err(),
        ErreurOrdre::ContrainteViolee {
            avant: Atom(2),
            apres: Atom(0),
        }
    );
}

/// π plaçant une variable de droite avant une variable de gauche :
/// contrainte du vtree violée, rejeté avec la paire fautive.
#[test]
fn test_pi_violant_vtree_rejete() {
    let resultat = Order::nouveau(&vtree_test(), vec![Atom(2), Atom(0), Atom(1), Atom(3)]);
    assert_eq!(
        resultat.unwrap_err(),
        ErreurOrdre::ContrainteViolee {
            avant: Atom(0),
            apres: Atom(2),
        }
    );
}

/// π qui n'est pas une permutation des variables du vtree : rejeté.
#[test]
fn test_pi_pas_une_permutation_rejete() {
    let resultat = Order::nouveau(&vtree_test(), vec![Atom(0), Atom(1), Atom(2), Atom(9)]);
    assert_eq!(resultat.unwrap_err(), ErreurOrdre::PasUnePermutation);
}

/// Vtree invalide : variable en double, erreur typée.
#[test]
fn test_vtree_variable_en_double_rejete() {
    let vtree = Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(0)));
    assert_eq!(
        vtree.valider().unwrap_err(),
        ErreurVtree::VariableEnDouble(Atom(0))
    );
}

/// Génère un vtree aléatoire sur `nombre_variables` variables, par LCG
/// déterministe (mêmes constantes que les autres tests de propriété).
fn vtree_aleatoire(graine: &mut u64, nombre_variables: u64) -> Vtree {
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    fn construire(graine: &mut u64, variables: &[Atom]) -> Vtree {
        if variables.len() == 1 {
            return Vtree::feuille(variables[0]);
        }
        let point_coupe = (suivant(graine) % (variables.len() as u64 - 1)) as usize + 1;
        Vtree::noeud(
            construire(graine, &variables[..point_coupe]),
            construire(graine, &variables[point_coupe..]),
        )
    }
    fn suivant(graine: &mut u64) -> u64 {
        *graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        *graine
    }
    let variables: Vec<Atom> = (0..nombre_variables).map(Atom).collect();
    construire(graine, &variables)
}

/// Vérifie structurellement (par parcours direct de l'arbre, indépendant de
/// `contraintes_ordre`) que `sequence` place toute variable de gauche avant
/// toute variable de droite à chaque nœud interne.
fn verifier_gauche_avant_droite(vtree: &Vtree, sequence: &[Atom]) {
    if let Vtree::Noeud { gauche, droite } = vtree {
        for variable_gauche in gauche.ordre_plat() {
            for variable_droite in droite.ordre_plat() {
                let position_gauche = sequence.iter().position(|a| *a == variable_gauche).unwrap();
                let position_droite = sequence.iter().position(|a| *a == variable_droite).unwrap();
                assert!(position_gauche < position_droite);
            }
        }
        verifier_gauche_avant_droite(gauche, sequence);
        verifier_gauche_avant_droite(droite, sequence);
    }
}

/// Propriété (cas générés par LCG) : pour tout vtree, l'ordre canonique est
/// une permutation des variables et satisfait gauche-avant-droite à chaque
/// nœud, vérifié par parcours direct de l'arbre.
#[test]
fn test_propriete_ordre_canonique_satisfait_contraintes() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const NOMBRE_CAS: u64 = 50;
    const BORNE_VARIABLES: u64 = 8;
    let mut graine = GRAINE_LCG;
    for _ in 0..NOMBRE_CAS {
        graine = graine
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let nombre_variables = 2 + graine % (BORNE_VARIABLES - 1);
        let vtree = vtree_aleatoire(&mut graine, nombre_variables);
        assert!(vtree.valider().is_ok());
        let ordre = Order::canonique(&vtree);
        let sequence = ordre.sequence();
        let mut triee = sequence.to_vec();
        triee.sort();
        let mut attendues: Vec<Atom> = (0..nombre_variables).map(Atom).collect();
        attendues.sort();
        assert_eq!(triee, attendues);
        verifier_gauche_avant_droite(&vtree, sequence);
    }
}
