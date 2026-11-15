//! ADD/MTBDD tests (M5-4, C7, open question n° 8): non-idempotent merge,
//! unique table indexed by weights, weighted fold exact on the lifted
//! construction, and compatibility guard (on a Bool construction, only the
//! cardinality is exact — multiplicity is destroyed at write time).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::add::{AddController, compiler_pondere, merge_add, somme_poids};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{count, union};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Test anchor.
const ANCHOR: Anchor = Anchor(0);

type PoolAdd = Arena<AddController>;
type PoolBool = Arena<ReferenceController>;

/// Test schema: `Map({0,1}, {0,1})`.
fn schema_test() -> Schema {
    Schema::Map(vec![Atom(0), Atom(1)], Domain::new(vec![Atom(0), Atom(1)]))
}

/// Builds a `Value::Map` from (key, value) pairs.
fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(key, value)| (Value::Atom(Atom(*key)), Value::Atom(Atom(*value))))
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
    compiler_pondere(pool, ANCHOR, &schema_test(), map, poids).unwrap()
}

/// Compiles a Bool singleton into the reference pool.
fn singleton_bool(pool: &mut PoolBool, map: &Value) -> NodeId {
    compiler(pool, ANCHOR, &schema_test(), map).unwrap()
}

/// Builds the set of maps in the ADD pool by non-idempotent merge of the
/// weight-1 singletons.
fn build_add(pool: &mut PoolAdd, set: &BTreeSet<Value>) -> NodeId {
    let keys = SchemaKeys::new(schema_test());
    let mut root = pool.intern(ANCHOR, Form::Terminal(0));
    for map in set {
        let membre = singleton_add(pool, map, 1);
        root = merge_add(pool, &keys, ANCHOR, root, membre).unwrap();
    }
    root
}

/// Builds the set of maps in the Bool pool by idempotent union.
fn build_bool(pool: &mut PoolBool, set: &BTreeSet<Value>) -> NodeId {
    let keys = SchemaKeys::new(schema_test());
    let mut root = pool.intern(ANCHOR, Form::Terminal(false));
    for map in set {
        let membre = singleton_bool(pool, map);
        root = union(pool, &keys, ANCHOR, root, membre).unwrap();
    }
    root
}

/// Counts the cardinality in the Bool pool (fold `(ℕ,+,×)`).
fn cardinality_bool(pool: &PoolBool, root: NodeId) -> u64 {
    let mut output = pool_bool();
    count(pool, &mut output, ANCHOR, &schema_test(), root).unwrap()
}

/// The unique table is indexed by the weights: terminals of different
/// weights = distinct handles, and their merge lands on the handle of the
/// sum weight.
#[test]
fn test_table_indexee_par_les_poids() {
    let mut pool = pool_add();
    let poids_un = pool.intern(ANCHOR, Form::Terminal(1));
    let poids_deux = pool.intern(ANCHOR, Form::Terminal(2));
    assert_ne!(poids_un, poids_deux);
    let keys = SchemaKeys::new(schema_test());
    let fusion = merge_add(&mut pool, &keys, ANCHOR, poids_un, poids_deux).unwrap();
    let poids_trois = pool.intern(ANCHOR, Form::Terminal(3));
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
    let double = merge_add(&mut pool, &keys, ANCHOR, a, a).unwrap();
    assert_ne!(double, a);
    assert_eq!(somme_poids(&pool, &schema_test(), a).unwrap(), 1);
    assert_eq!(somme_poids(&pool, &schema_test(), double).unwrap(), 2);
}

/// Weighted fold exact on the lifted construction: sum of weights == set
/// cardinality, equal to the fold count on the Bool construction.
#[test]
fn test_somme_poids_egale_cardinality() {
    let set: BTreeSet<Value> = [
        map_de(&[(0, 0), (1, 0)]),
        map_de(&[(0, 0), (1, 1)]),
        map_de(&[(0, 1), (1, 0)]),
    ]
    .into_iter()
    .collect();
    let mut pool_a = pool_add();
    let root_add = build_add(&mut pool_a, &set);
    assert_eq!(somme_poids(&pool_a, &schema_test(), root_add).unwrap(), 3);
    let mut pool_b = pool_bool();
    let root_bool = build_bool(&mut pool_b, &set);
    assert_eq!(cardinality_bool(&pool_b, root_bool), 3);
}

/// Compatibility guard (Q8): with a duplicate, the lifted construction
/// counts the multiplicity (weighted sum = 4) while the Bool construction
/// destroys it (cardinality = 3) — a weighted fold is exact only on the lifted
/// construction.
#[test]
fn test_garde_fou_compatibilite_doublon() {
    let map = map_de(&[(0, 1), (1, 1)]);
    let set: BTreeSet<Value> = [
        map_de(&[(0, 0), (1, 0)]),
        map_de(&[(0, 0), (1, 1)]),
        map.clone(),
    ]
    .into_iter()
    .collect();
    let mut pool_a = pool_add();
    let keys = SchemaKeys::new(schema_test());
    let mut root = pool_a.intern(ANCHOR, Form::Terminal(0));
    for membre in &set {
        let singleton = singleton_add(&mut pool_a, membre, 1);
        root = merge_add(&mut pool_a, &keys, ANCHOR, root, singleton).unwrap();
    }
    // The duplicate is counted twice: 4, not 3.
    let doublon = singleton_add(&mut pool_a, &map, 1);
    root = merge_add(&mut pool_a, &keys, ANCHOR, root, doublon).unwrap();
    assert_eq!(somme_poids(&pool_a, &schema_test(), root).unwrap(), 4);
    let mut pool_b = pool_bool();
    let root_bool = build_bool(&mut pool_b, &set);
    assert_eq!(cardinality_bool(&pool_b, root_bool), 3);
}

/// Property (LCG-generated cases): for any set of maps over 2 keys with
/// domain {0,1}, weighted sum on the ADD construction == cardinality on the
/// Bool construction == set size.
#[test]
fn test_propriete_somme_poids_egale_cardinality_lcg() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CAS: u64 = 30;
    let mut seed = SEED_LCG;
    for _ in 0..NUMBER_CAS {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        let number_maps = (seed % 5) as usize;
        let mut set = BTreeSet::new();
        for _ in 0..number_maps {
            seed = seed
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            let x0 = seed % 2;
            seed = seed
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            let x1 = seed % 2;
            set.insert(map_de(&[(0, x0), (1, x1)]));
        }
        let mut pool_a = pool_add();
        let root_add = build_add(&mut pool_a, &set);
        let mut pool_b = pool_bool();
        let root_bool = build_bool(&mut pool_b, &set);
        assert_eq!(
            somme_poids(&pool_a, &schema_test(), root_add).unwrap(),
            set.len() as u64
        );
        assert_eq!(cardinality_bool(&pool_b, root_bool), set.len() as u64);
    }
}
