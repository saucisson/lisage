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

/// Propriété : l'ordre canonique d'un vtree contient chaque variable
/// exactement une fois, et toute contrainte gauche-droite y est satisfaite.
#[test]
fn test_propriete_ordre_canonique_satisfait_contraintes() {
    let vtree = Vtree::noeud(
        Vtree::noeud(Vtree::feuille(Atom(3)), Vtree::feuille(Atom(1))),
        Vtree::feuille(Atom(2)),
    );
    let ordre = Order::canonique(&vtree);
    let sequence = ordre.sequence();
    let mut triee = sequence.to_vec();
    triee.sort();
    assert_eq!(triee, vec![Atom(1), Atom(2), Atom(3)]);
    for (avant, apres) in vtree.contraintes_ordre() {
        let position_avant = ordre.position(&avant).unwrap();
        let position_apres = ordre.position(&apres).unwrap();
        assert!(position_avant < position_apres);
    }
}
