//! ADD/MTBDD tests (M5-4, C7, open question n° 8): non-idempotent merge,
//! unique table indexed by weights, weighted fold exact on the lifted
//! construction, and compatibility guard (on a Bool construction, only the
//! cardinal is exact — multiplicity is destroyed at write time).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::add::{AddController, compiler_pondere, merge_add, somme_poids};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Test anchor.
const ANCRE: Anchor = Anchor(0);

type PoolAdd = Arena<AddController>;
type PoolBool = Arena<ReferenceController>;

/// Test schema: `Map({0,1}, {0,1})`.
fn schema_test() -> Schema {
    Schema::Map(
        vec![Atom(0), Atom(1)],
        Domaine::nouveau(vec![Atom(0), Atom(1)]),
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

/// ADD pool for the test schema.
fn pool_add() -> PoolAdd {
    Arena::new(AddController::new(schema_test()), Box::new(NoOpObserver))
}

/// Bool pool (reference instance) for the test schema.
fn pool_bool() -> PoolBool {
    Arena::new(
        ReferenceController::new(schema_test()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a weighted singleton (given weight) into the ADD pool.
fn singleton_add(pool: &mut PoolAdd, map: &Value, poids: u64) -> NodeId {
    compiler_pondere(pool, ANCRE, &schema_test(), map, poids).unwrap()
}

/// Compiles a Bool singleton into the reference pool.
fn singleton_bool(pool: &mut PoolBool, map: &Value) -> NodeId {
    compiler(pool, ANCRE, &schema_test(), map).unwrap()
}

/// Builds the set of maps in the ADD pool by non-idempotent merge of the
/// weight-1 singletons.
fn construire_add(pool: &mut PoolAdd, ensemble: &BTreeSet<Value>) -> NodeId {
    let keys = SchemaKeys::new(schema_test());
    let mut racine = pool.intern(ANCRE, Form::Terminal(0));
    for map in ensemble {
        let membre = singleton_add(pool, map, 1);
        racine = merge_add(pool, &keys, ANCRE, racine, membre).unwrap();
    }
    racine
}

/// Builds the set of maps in the Bool pool by idempotent union.
fn construire_bool(pool: &mut PoolBool, ensemble: &BTreeSet<Value>) -> NodeId {
    let keys = SchemaKeys::new(schema_test());
    let mut racine = pool.intern(ANCRE, Form::Terminal(false));
    for map in ensemble {
        let membre = singleton_bool(pool, map);
        racine = union(pool, &keys, ANCRE, racine, membre).unwrap();
    }
    racine
}

/// Counts the cardinal in the Bool pool (fold `(ℕ,+,×)`).
fn cardinal_bool(pool: &PoolBool, racine: NodeId) -> u64 {
    let mut sortie = pool_bool();
    compter(pool, &mut sortie, ANCRE, &schema_test(), racine).unwrap()
}

/// The unique table is indexed by the weights: terminals of different
/// weights = distinct handles, and their merge lands on the handle of the
/// sum weight.
#[test]
fn test_table_indexee_par_les_poids() {
    let mut pool = pool_add();
    let poids_un = pool.intern(ANCRE, Form::Terminal(1));
    let poids_deux = pool.intern(ANCRE, Form::Terminal(2));
    assert_ne!(poids_un, poids_deux);
    let keys = SchemaKeys::new(schema_test());
    let fusion = merge_add(&mut pool, &keys, ANCRE, poids_un, poids_deux).unwrap();
    let poids_trois = pool.intern(ANCRE, Form::Terminal(3));
    assert_eq!(fusion, poids_trois);
}

/// Non-idempotent merge: `merge_add(a, a)` doubles the weights (unlike Bool
/// `union` where `union(a, a) = a`).
#[test]
fn test_merge_non_idempotent() {
    let map = map_de(&[(0, 1), (1, 0)]);
    let mut pool = pool_add();
    let a = singleton_add(&mut pool, &map, 1);
    let keys = SchemaKeys::new(schema_test());
    let double = merge_add(&mut pool, &keys, ANCRE, a, a).unwrap();
    assert_ne!(double, a);
    assert_eq!(somme_poids(&pool, &schema_test(), a).unwrap(), 1);
    assert_eq!(somme_poids(&pool, &schema_test(), double).unwrap(), 2);
}

/// Weighted fold exact on the lifted construction: sum of weights == set
/// cardinal, equal to the fold count on the Bool construction.
#[test]
fn test_somme_poids_egale_cardinal() {
    let ensemble: BTreeSet<Value> = [
        map_de(&[(0, 0), (1, 0)]),
        map_de(&[(0, 0), (1, 1)]),
        map_de(&[(0, 1), (1, 0)]),
    ]
    .into_iter()
    .collect();
    let mut pool_a = pool_add();
    let racine_add = construire_add(&mut pool_a, &ensemble);
    assert_eq!(somme_poids(&pool_a, &schema_test(), racine_add).unwrap(), 3);
    let mut pool_b = pool_bool();
    let racine_bool = construire_bool(&mut pool_b, &ensemble);
    assert_eq!(cardinal_bool(&pool_b, racine_bool), 3);
}

/// Compatibility guard (Q8): with a duplicate, the lifted construction
/// counts the multiplicity (weighted sum = 4) while the Bool construction
/// destroys it (cardinal = 3) — a weighted fold is exact only on the lifted
/// construction.
#[test]
fn test_garde_fou_compatibilite_doublon() {
    let map = map_de(&[(0, 1), (1, 1)]);
    let ensemble: BTreeSet<Value> = [
        map_de(&[(0, 0), (1, 0)]),
        map_de(&[(0, 0), (1, 1)]),
        map.clone(),
    ]
    .into_iter()
    .collect();
    let mut pool_a = pool_add();
    let keys = SchemaKeys::new(schema_test());
    let mut racine = pool_a.intern(ANCRE, Form::Terminal(0));
    for membre in &ensemble {
        let singleton = singleton_add(&mut pool_a, membre, 1);
        racine = merge_add(&mut pool_a, &keys, ANCRE, racine, singleton).unwrap();
    }
    // The duplicate is counted twice: 4, not 3.
    let doublon = singleton_add(&mut pool_a, &map, 1);
    racine = merge_add(&mut pool_a, &keys, ANCRE, racine, doublon).unwrap();
    assert_eq!(somme_poids(&pool_a, &schema_test(), racine).unwrap(), 4);
    let mut pool_b = pool_bool();
    let racine_bool = construire_bool(&mut pool_b, &ensemble);
    assert_eq!(cardinal_bool(&pool_b, racine_bool), 3);
}

/// Property (LCG-generated cases): for any set of maps over 2 keys with
/// domain {0,1}, weighted sum on the ADD construction == cardinal on the
/// Bool construction == set size.
#[test]
fn test_propriete_somme_poids_egale_cardinal_lcg() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    for _ in 0..NOMBRE_CAS {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        let nombre_maps = (graine % 5) as usize;
        let mut ensemble = BTreeSet::new();
        for _ in 0..nombre_maps {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            let x0 = graine % 2;
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            let x1 = graine % 2;
            ensemble.insert(map_de(&[(0, x0), (1, x1)]));
        }
        let mut pool_a = pool_add();
        let racine_add = construire_add(&mut pool_a, &ensemble);
        let mut pool_b = pool_bool();
        let racine_bool = construire_bool(&mut pool_b, &ensemble);
        assert_eq!(
            somme_poids(&pool_a, &schema_test(), racine_add).unwrap(),
            ensemble.len() as u64
        );
        assert_eq!(cardinal_bool(&pool_b, racine_bool), ensemble.len() as u64);
    }
}
