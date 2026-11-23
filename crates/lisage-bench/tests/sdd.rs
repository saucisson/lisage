//! Tests of the sentential SDD installation without ad hoc extension
//! (M5-3, C6): vtree = structured order, controller tower = local anchor
//! per node, C1 canonicity by handle, global cardinality = product of the
//! local cardinalities when the function is decomposable along the vtree.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, Pool};
use lisage_algebra::vtree::{ErrorVtree, Vtree};
use lisage_bench::sdd::{ErrorSdd, installer_sdd, pool_for_sdd, projeter};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Test vtree: (x0, x1) on the left, (x2, x3) on the right.
fn vtree_test() -> Vtree {
    Vtree::node(
        Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
    )
}

/// Builds a `Value::Map` from (key, value) pairs.
fn map_from(pairs: &[(u64, u64)]) -> Value {
    Value::Map(
        pairs
            .iter()
            .map(|(key, value)| (Value::Atom(Atom(*key)), Value::Atom(Atom(*value))))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Set decomposable along the vtree: S = A × B with A = {x0 = x1}
/// (2 maps over {0,1}) and B = {x2 = x3} (2 maps over {2,3}) — 4 maps over 4 keys.
fn set_decomposable() -> BTreeSet<Value> {
    let mut set = BTreeSet::new();
    for a in 0..=1u64 {
        for b in 0..=1u64 {
            set.insert(map_from(&[(0, a), (1, a), (2, b), (3, b)]));
        }
    }
    set
}

/// Not-decomposable set: {x0 = x2} — 8 maps over 4 keys.
fn set_not_decomposable() -> BTreeSet<Value> {
    let mut set = BTreeSet::new();
    for c in 0..=1u64 {
        for x1 in 0..=1u64 {
            for x3 in 0..=1u64 {
                set.insert(map_from(&[(0, c), (1, x1), (2, c), (3, x3)]));
            }
        }
    }
    set
}

/// Pool for a test: root schema of the test vtree over {0,1}.
fn fresh_pool() -> Arena<ReferenceController> {
    pool_for_sdd(&vtree_test(), &set_decomposable())
}

/// Cardinality of the flat DD of the set over the 4 keys (path B reference).
fn cardinality_plat(pool: &mut Arena<ReferenceController>, set: &BTreeSet<Value>) -> u64 {
    let keys: Vec<Atom> = (0..4).map(Atom).collect();
    let schema = Schema::Map(keys, Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut root = pool.intern(anchor, Form::Terminal(false));
    for value in set {
        let membre = compiler(pool, anchor, &schema, value).unwrap();
        root = union(pool, &keys, anchor, root, membre).unwrap();
    }
    let mut output = pool_for_sdd(&vtree_test(), set);
    count(pool, &mut output, anchor, &schema, root).unwrap()
}

/// C6 falsification: global cardinality of the flat DD == cardinality of the
/// hierarchical installation, and for a set decomposable along the
/// vtree, root cardinality == product of the cardinalities of the two subtrees.
#[test]
fn test_cardinality_global_equals_product_of_locals_if_decomposable() {
    let set = set_decomposable();
    let mut pool = fresh_pool();
    let level = installer_sdd(&mut pool, &vtree_test(), &set).unwrap();
    assert_eq!(level.cardinality, 4);
    let left = installer_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        &set,
    )
    .unwrap();
    let right = installer_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
        &set,
    )
    .unwrap();
    assert_eq!(left.cardinality, 2);
    assert_eq!(right.cardinality, 2);
    assert_eq!(level.cardinality, left.cardinality * right.cardinality);
    assert_eq!(cardinality_plat(&mut pool, &set), 4);
}

/// Not-decomposable set: the product of the local cardinalities upper-bounds the
/// global cardinality (over-approximation of the decomposition), the global
/// cardinality remaining exact and equal to the flat DD.
#[test]
fn test_product_of_locals_bounds_the_global_if_not_decomposable() {
    let set = set_not_decomposable();
    let mut pool = fresh_pool();
    let level = installer_sdd(&mut pool, &vtree_test(), &set).unwrap();
    assert_eq!(level.cardinality, 8);
    let left = installer_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        &set,
    )
    .unwrap();
    let right = installer_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
        &set,
    )
    .unwrap();
    assert_eq!(left.cardinality, 4);
    assert_eq!(right.cardinality, 4);
    assert!(left.cardinality * right.cardinality >= level.cardinality);
    assert_eq!(cardinality_plat(&mut pool, &set), 8);
}

/// C1 canonicity by handle: reinstalling the same vtree on the same
/// set in the same pool gives the same handles (hash-consing).
#[test]
fn test_canonicity_per_handle_reinstallation_identical() {
    let set = set_decomposable();
    let mut pool = fresh_pool();
    let first = installer_sdd(&mut pool, &vtree_test(), &set).unwrap();
    let second = installer_sdd(&mut pool, &vtree_test(), &set).unwrap();
    assert_eq!(first, second);
}

/// C1 canonicity: same internal form under two distinct anchors = two
/// distinct handles (quotient classes), same form under the same anchor = same
/// handle. Terminals are the exception (MEM-11): pool-wide singletons keyed
/// by payload only, shared across anchors.
#[test]
fn test_canonicity_two_anchors_distinct() {
    let mut pool = fresh_pool();
    let t_true = pool.intern(Anchor(0), Form::Terminal(true));
    let t_false = pool.intern(Anchor(0), Form::Terminal(false));
    let form = Form::Node {
        level: Level(0),
        arcs: ArcRun::from_pairs([(0, t_true), (1, t_false)]),
    };
    let same_anchor_a = pool.intern(Anchor(0), form.clone());
    let same_anchor_b = pool.intern(Anchor(0), form.clone());
    let other_anchor = pool.intern(Anchor(1), form);
    assert_eq!(same_anchor_a, same_anchor_b);
    assert_ne!(same_anchor_a, other_anchor);
    // MEM-11: a terminal interned under another anchor is the same singleton.
    let t_true_other_ancre = pool.intern(Anchor(1), Form::Terminal(true));
    assert_eq!(t_true, t_true_other_ancre);
}

/// Projection: keeps only the requested keys.
#[test]
fn test_projection() {
    let set = set_decomposable();
    let projection = projeter(&set, &[Atom(0), Atom(1)]);
    assert_eq!(projection.len(), 2);
    for value in &projection {
        let Value::Map(map) = value else {
            panic!("the projection is not a map");
        };
        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&Value::Atom(Atom(0))));
        assert!(map.contains_key(&Value::Atom(Atom(1))));
    }
}

/// Invalid vtree (duplicated variable): typed error at installation.
#[test]
fn test_vtree_invalid_rejected() {
    let vtree = Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(0)));
    let mut pool = fresh_pool();
    assert_eq!(
        installer_sdd(&mut pool, &vtree, &BTreeSet::new()).unwrap_err(),
        ErrorSdd::Vtree(ErrorVtree::DuplicateVariable(Atom(0)))
    );
}
