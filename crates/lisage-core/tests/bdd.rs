//! Tests de la configuration BDD classique (M1-8, C6) : compilation par
//! table de vérité, comptage de nœuds, évaluation.

use lisage_algebra::interface::Anchor;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, compter_noeuds, evaluer};
use lisage_core::observer::NoOpObserver;

type PoolBdd = Arena<BddController>;

fn pool(nombre_variables: u32) -> PoolBdd {
    Arena::new(BddController::new(nombre_variables), Box::new(NoOpObserver))
}

/// Table de vérité de la constante ⊥.
fn table_constante(nombre_variables: u32, valeur: bool) -> Vec<bool> {
    vec![valeur; 1usize << nombre_variables]
}

/// Table de vérité du XOR de deux variables.
fn table_xor2() -> Vec<bool> {
    (0..4u64).map(|a| (a & 1) != ((a >> 1) & 1)).collect()
}

#[test]
fn test_bdd_constantes_un_seul_terminal() {
    let mut pool = pool(3);
    let anchor = Anchor(0);
    let keys = BddKeys::new(3);
    let racine_faux = compiler_bdd(&mut pool, anchor, &keys, &table_constante(3, false));
    let racine_vrai = compiler_bdd(&mut pool, anchor, &keys, &table_constante(3, true));
    // R-collapse : aucune variable, un seul terminal visité.
    assert_eq!(compter_noeuds(&pool, racine_faux), 1);
    assert_eq!(compter_noeuds(&pool, racine_vrai), 1);
    assert!(!evaluer(&pool, racine_faux, 0b101).unwrap());
    assert!(evaluer(&pool, racine_vrai, 0b101).unwrap());
}

#[test]
fn test_bdd_xor2_compte_et_evalue() {
    let mut pool = pool(2);
    let anchor = Anchor(0);
    let keys = BddKeys::new(2);
    let table = table_xor2();
    let racine = compiler_bdd(&mut pool, anchor, &keys, &table);
    // XOR(x0, x1) : racine x0 + deux nœuds x1 distincts + ⊥/⊤ = 5 nœuds.
    assert_eq!(compter_noeuds(&pool, racine), 5);
    for a in 0..4u64 {
        assert_eq!(
            evaluer(&pool, racine, a).unwrap(),
            table[a as usize],
            "affectation {a}"
        );
    }
}

#[test]
fn test_bdd_partage_hash_consing() {
    let mut pool = pool(2);
    let anchor = Anchor(0);
    let keys = BddKeys::new(2);
    // x0 (projection) : le nœud x1 est partagé entre les deux branches de x0.
    let table = (0..4u64).map(|a| a & 1 == 1).collect::<Vec<bool>>();
    let racine = compiler_bdd(&mut pool, anchor, &keys, &table);
    // Projection x0 : racine x0 → (⊥, ⊤), soit 3 nœuds (R-collapse des
    // branches constantes).
    assert_eq!(compter_noeuds(&pool, racine), 3);
}
