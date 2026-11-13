//! Tests of the sentential SDD installation without ad hoc extension
//! (M5-3, C6): vtree = structured order, controller tower = local anchor
//! per node, C1 canonicity by handle, global cardinality = product of the
//! local cardinalities when the function is decomposable along the vtree.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Pool};
use lisage_algebra::vtree::{ErrorVtree, Vtree};
use lisage_bench::sdd::{ErrorSdd, install_sdd, pool_for_sdd, projeter};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_lang::value::{Atom, Value};

/// Test vtree: (x0, x1) on the left, (x2, x3) on the right.
fn vtree_test() -> Vtree {
    Vtree::node(
        Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
    )
}

/// Builds a `Value::Map` from (key, value) pairs.
fn map_of(pairs: &[(u64, u64)]) -> Value {
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
            set.insert(map_of(&[(0, a), (1, a), (2, b), (3, b)]));
        }
    }
    set
}

/// Non-decomposable set: {x0 = x2} — 8 maps over 4 keys.
fn set_not_decomposable() -> BTreeSet<Value> {
    let mut set = BTreeSet::new();
    for c in 0..=1u64 {
        for x1 in 0..=1u64 {
            for x3 in 0..=1u64 {
                set.insert(map_of(&[(0, c), (1, x1), (2, c), (3, x3)]));
            }
        }
    }
    set
}

/// Pool for a test: root schema of the test vtree over {0,1}.
fn pool_nine() -> Arena<ReferenceController> {
    pool_for_sdd(&vtree_test(), &set_decomposable())
}

/// Cardinality of the flat DD of the set over the 4 keys (lane B reference).
fn cardinal_flat(pool: &mut Arena<ReferenceController>, set: &BTreeSet<Value>) -> u64 {
    let keys: Vec<Atom> = (0..4).map(Atom).collect();
    let schema = Schema::Map(keys, Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut root = pool.intern(anchor, Form::Terminal(false));
    for value in set {
        let membre = compiled(pool, anchor, &schema, value).unwrap();
        root = union(pool, &keys, anchor, root, membre).unwrap();
    }
    let mut output = pool_for_sdd(&vtree_test(), set);
    count(pool, &mut output, anchor, &schema, root).unwrap()
}

/// C6 falsification: global cardinality of the flat DD == cardinality of
/// the hierarchical installation, and for a set decomposable along the
/// vtree, root cardinality == product of the cardinalities of the two
/// subtrees.
#[test]
fn test_cardinal_global_equal_product_of_local_si_decomposable() {
    let set = set_decomposable();
    let mut pool = pool_nine();
    let level = install_sdd(&mut pool, &vtree_test(), &set).unwrap();
    assert_eq!(level.cardinal, 4);
    let left = install_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        &set,
    )
    .unwrap();
    let right = install_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
        &set,
    )
    .unwrap();
    assert_eq!(left.cardinal, 2);
    assert_eq!(right.cardinal, 2);
    assert_eq!(level.cardinal, left.cardinal * right.cardinal);
    assert_eq!(cardinal_flat(&mut pool, &set), 4);
}

/// Non-decomposable set: the product of the local cardinalities upper-bounds
/// the global cardinality (over-approximation of the decomposition), the
/// global cardinality remaining exact and equal to the flat DD.
#[test]
fn test_product_of_local_majore_the_global_si_not_decomposable() {
    let set = set_not_decomposable();
    let mut pool = pool_nine();
    let level = install_sdd(&mut pool, &vtree_test(), &set).unwrap();
    assert_eq!(level.cardinal, 8);
    let left = install_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        &set,
    )
    .unwrap();
    let right = install_sdd(
        &mut pool,
        &Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
        &set,
    )
    .unwrap();
    assert_eq!(left.cardinal, 4);
    assert_eq!(right.cardinal, 4);
    assert!(left.cardinal * right.cardinal >= level.cardinal);
    assert_eq!(cardinal_flat(&mut pool, &set), 8);
}

/// C1 canonicity by handle: reinstalling the same vtree on the same set in
/// the same pool gives the same handles (hash-consing).
#[test]
fn test_canonicity_by_handle_reinstall_identical() {
    let set = set_decomposable();
    let mut pool = pool_nine();
    let first = install_sdd(&mut pool, &vtree_test(), &set).unwrap();
    let second = install_sdd(&mut pool, &vtree_test(), &set).unwrap();
    assert_eq!(first, second);
}

/// C1 canonicity: same form under two distinct anchors = two distinct
/// handles (quotient classes), same form under the same anchor = same
/// handle.
#[test]
fn test_canonicity_two_anchors_distinct() {
    let mut pool = pool_nine();
    let form = Form::Terminal(true);
    let same_anchor_a = pool.intern(Anchor(0), form.clone());
    let same_anchor_b = pool.intern(Anchor(0), form.clone());
    let other_anchor = pool.intern(Anchor(1), form);
    assert_eq!(same_anchor_a, same_anchor_b);
    assert_ne!(same_anchor_a, other_anchor);
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

/// Invalid vtree (duplicate variable): typed error at installation.
#[test]
fn test_vtree_invalid_rejected() {
    let vtree = Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(0)));
    let mut pool = pool_nine();
    assert_eq!(
        install_sdd(&mut pool, &vtree, &BTreeSet::new()).unwrap_err(),
        ErrorSdd::Vtree(ErrorVtree::VariableInDouble(Atom(0)))
    );
}
