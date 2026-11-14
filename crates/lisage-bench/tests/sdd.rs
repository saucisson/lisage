//! Tests of the sentential SDD installation without ad hoc extension
//! (M5-3, C6): vtree = structured order, controller tower = local anchor
//! per node, C1 canonicity by handle, global cardinal = product of the
//! local cardinals when the function is decomposable along the vtree.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Pool};
use lisage_algebra::vtree::{ErreurVtree, Vtree};
use lisage_bench::sdd::{ErreurSdd, installer_sdd, pool_pour_sdd, projeter};
use lisage_core::arena::Arena;
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Test vtree: (x0, x1) on the left, (x2, x3) on the right.
fn vtree_test() -> Vtree {
    Vtree::noeud(
        Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
        Vtree::noeud(Vtree::feuille(Atom(2)), Vtree::feuille(Atom(3))),
    )
}

/// Builds a `Value::Map` from (key, value) pairs.
fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(cle, valeur)| (Value::Atom(Atom(*cle)), Value::Atom(Atom(*valeur))))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Set decomposable along the vtree: S = A × B with A = {x0 = x1}
/// (2 maps over {0,1}) and B = {x2 = x3} (2 maps over {2,3}) — 4 maps over 4 keys.
fn ensemble_decomposable() -> BTreeSet<Value> {
    let mut ensemble = BTreeSet::new();
    for a in 0..=1u64 {
        for b in 0..=1u64 {
            ensemble.insert(map_de(&[(0, a), (1, a), (2, b), (3, b)]));
        }
    }
    ensemble
}

/// Non-decomposable set: {x0 = x2} — 8 maps over 4 keys.
fn ensemble_non_decomposable() -> BTreeSet<Value> {
    let mut ensemble = BTreeSet::new();
    for c in 0..=1u64 {
        for x1 in 0..=1u64 {
            for x3 in 0..=1u64 {
                ensemble.insert(map_de(&[(0, c), (1, x1), (2, c), (3, x3)]));
            }
        }
    }
    ensemble
}

/// Pool for a test: root schema of the test vtree over {0,1}.
fn pool_neuf() -> Arena<ReferenceController> {
    pool_pour_sdd(&vtree_test(), &ensemble_decomposable())
}

/// Cardinal of the flat DD of the set over the 4 keys (path B reference).
fn cardinal_plat(pool: &mut Arena<ReferenceController>, ensemble: &BTreeSet<Value>) -> u64 {
    let cles: Vec<Atom> = (0..4).map(Atom).collect();
    let schema = Schema::Map(cles, Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let ancre = Anchor(0);
    let mut racine = pool.intern(ancre, Form::Terminal(false));
    for valeur in ensemble {
        let membre = compiler(pool, ancre, &schema, valeur).unwrap();
        racine = union(pool, &keys, ancre, racine, membre).unwrap();
    }
    let mut sortie = pool_pour_sdd(&vtree_test(), ensemble);
    compter(pool, &mut sortie, ancre, &schema, racine).unwrap()
}

/// C6 falsification: global cardinal of the flat DD == cardinal of the
/// hierarchical installation, and for a set decomposable along the
/// vtree, root cardinal == product of the cardinals of the two subtrees.
#[test]
fn test_cardinal_global_egal_produit_des_locaux_si_decomposable() {
    let ensemble = ensemble_decomposable();
    let mut pool = pool_neuf();
    let niveau = installer_sdd(&mut pool, &vtree_test(), &ensemble).unwrap();
    assert_eq!(niveau.cardinal, 4);
    let gauche = installer_sdd(
        &mut pool,
        &Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
        &ensemble,
    )
    .unwrap();
    let droite = installer_sdd(
        &mut pool,
        &Vtree::noeud(Vtree::feuille(Atom(2)), Vtree::feuille(Atom(3))),
        &ensemble,
    )
    .unwrap();
    assert_eq!(gauche.cardinal, 2);
    assert_eq!(droite.cardinal, 2);
    assert_eq!(niveau.cardinal, gauche.cardinal * droite.cardinal);
    assert_eq!(cardinal_plat(&mut pool, &ensemble), 4);
}

/// Non-decomposable set: the product of the local cardinals upper-bounds the
/// global cardinal (over-approximation of the decomposition), the global
/// cardinal remaining exact and equal to the flat DD.
#[test]
fn test_produit_des_locaux_majore_le_global_si_non_decomposable() {
    let ensemble = ensemble_non_decomposable();
    let mut pool = pool_neuf();
    let niveau = installer_sdd(&mut pool, &vtree_test(), &ensemble).unwrap();
    assert_eq!(niveau.cardinal, 8);
    let gauche = installer_sdd(
        &mut pool,
        &Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
        &ensemble,
    )
    .unwrap();
    let droite = installer_sdd(
        &mut pool,
        &Vtree::noeud(Vtree::feuille(Atom(2)), Vtree::feuille(Atom(3))),
        &ensemble,
    )
    .unwrap();
    assert_eq!(gauche.cardinal, 4);
    assert_eq!(droite.cardinal, 4);
    assert!(gauche.cardinal * droite.cardinal >= niveau.cardinal);
    assert_eq!(cardinal_plat(&mut pool, &ensemble), 8);
}

/// C1 canonicity by handle: reinstalling the same vtree on the same
/// set in the same pool gives the same handles (hash-consing).
#[test]
fn test_canonicite_par_handle_reinstallation_identique() {
    let ensemble = ensemble_decomposable();
    let mut pool = pool_neuf();
    let premier = installer_sdd(&mut pool, &vtree_test(), &ensemble).unwrap();
    let second = installer_sdd(&mut pool, &vtree_test(), &ensemble).unwrap();
    assert_eq!(premier, second);
}

/// C1 canonicity: same form under two distinct anchors = two
/// distinct handles (quotient classes), same form under the same anchor = same
/// handle.
#[test]
fn test_canonicite_deux_ancres_distinctes() {
    let mut pool = pool_neuf();
    let forme = Form::Terminal(true);
    let meme_ancre_a = pool.intern(Anchor(0), forme.clone());
    let meme_ancre_b = pool.intern(Anchor(0), forme.clone());
    let autre_ancre = pool.intern(Anchor(1), forme);
    assert_eq!(meme_ancre_a, meme_ancre_b);
    assert_ne!(meme_ancre_a, autre_ancre);
}

/// Projection: keeps only the requested keys.
#[test]
fn test_projection() {
    let ensemble = ensemble_decomposable();
    let projection = projeter(&ensemble, &[Atom(0), Atom(1)]);
    assert_eq!(projection.len(), 2);
    for valeur in &projection {
        let Value::Map(map) = valeur else {
            panic!("the projection is not a map");
        };
        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&Value::Atom(Atom(0))));
        assert!(map.contains_key(&Value::Atom(Atom(1))));
    }
}

/// Invalid vtree (duplicated variable): typed error at installation.
#[test]
fn test_vtree_invalide_rejete() {
    let vtree = Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(0)));
    let mut pool = pool_neuf();
    assert_eq!(
        installer_sdd(&mut pool, &vtree, &BTreeSet::new()).unwrap_err(),
        ErreurSdd::Vtree(ErreurVtree::VariableEnDouble(Atom(0)))
    );
}
