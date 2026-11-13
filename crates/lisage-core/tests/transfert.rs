//! Non-local event tests (M2-5, spec § 7, statement T3): the
//! transfer `m(a) -= 1; m(b) += 1` compiled into a transduction with level memory
//! — inherited token between not-adjacent levels, recording of the
//! decrement in `M`, idempotence, and termination under `saturate`.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, PolicyMemory};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::saturate::{ParametersSaturation, PolicySaturation, saturer};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled, decompile};
use lisage_core::transduction::Transducer;
use lisage_core::transfer::{TransferNotLocal, WIDTH_TRANSFERT};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

/// Schema `Map([a, b, c])` over the domain `{0, 1, 2, 3}`.
fn schema_map() -> Schema {
    Schema::Map(
        vec![Atom(0), Atom(1), Atom(2)],
        Domain::new(vec![Atom(0), Atom(1), Atom(2), Atom(3)]),
    )
}

fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a map `key ↦ value` (atoms) into the pool.
fn compile_map(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    inputs: &[(u64, u64)],
) -> lisage_algebra::interface::NodeId {
    compiled(
        pool,
        anchor,
        schema,
        &Value::Map(
            inputs
                .iter()
                .map(|(k, v)| (Value::Atom(Atom(*k)), Value::Atom(Atom(*v))))
                .collect(),
        ),
    )
    .unwrap()
}

/// Decompiles a node into a map `key ↦ value` (atoms).
fn decompile_map(
    pool: &PoolRef,
    schema: &Schema,
    node: lisage_algebra::interface::NodeId,
) -> BTreeMap<u64, u64> {
    match decompile(pool, schema, node).unwrap() {
        Value::Map(map) => map
            .into_iter()
            .map(|(k, v)| match (k, v) {
                (Value::Atom(Atom(k)), Value::Atom(Atom(v))) => (k, v),
                _ => panic!("expected atoms"),
            })
            .collect(),
        _ => panic!("map expected"),
    }
}

/// Cardinality of a diagram (number of members of the denoted family).
fn cardinal(pool: &PoolRef, schema: &Schema, node: lisage_algebra::interface::NodeId) -> u64 {
    let mut output = new_pool(schema);
    count(pool, &mut output, Anchor(0), schema, node).unwrap()
}

/// M2-5, T3: the not-local transfer `a -= 1; b += 1` between two not-
/// adjacent keys (a and b separated by key c) applies in one pass.
#[test]
fn test_transfer_a_toward_b_not_adjacent() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let initial = compile_map(&mut pool, anchor, &schema, &[(0, 2), (1, 0), (2, 1)]);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let (result, _) = runner.run_same_pool(&mut pool, anchor, initial).unwrap();

    let expected = BTreeMap::from([(0, 1), (1, 1), (2, 1)]);
    assert_eq!(
        decompile_map(&pool, &schema, result),
        expected,
        "m(a) -= 1 ; m(b) += 1, the intermediate key c is unchanged"
    );
}

/// M2-5: with no unit to transfer (m(a) = 0), the map is unchanged.
#[test]
fn test_transfer_without_unite() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let initial = compile_map(&mut pool, anchor, &schema, &[(0, 0), (1, 1), (2, 2)]);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let (result, _) = runner.run_same_pool(&mut pool, anchor, initial).unwrap();

    assert_eq!(result, initial, "m(a) = 0: no transfer");
}

/// M2-5, T3: the level memory records the decrement at the emitting node
/// (one entry per touched node — O(w) overhead per node), and the width of the
/// transition is bounded.
#[test]
fn test_transfer_memory_and_width() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let initial = compile_map(&mut pool, anchor, &schema, &[(0, 2), (1, 0)]);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    assert_eq!(transfer.width(), WIDTH_TRANSFERT);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let _ = runner.run_same_pool(&mut pool, anchor, initial).unwrap();

    // The decrement was produced: one memory entry, bumped version (L1).
    assert_eq!(runner.memory().size(), 1, "one entry per emitting node");
    assert!(runner.version_memory() >= 1, "each write bumps the version");
}

/// M2-5, pinned edge case: destination at the maximum of the domain — the placement
/// is dropped (edge kept) but the source decrement still
/// applies (per-edge semantics, spec § 7 `φ_disp(val(a), h, v)`).
#[test]
fn test_transfer_destination_out_domain() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    // m(a) = 2, m(b) = 3 = max of the domain {0,1,2,3}.
    let initial = compile_map(&mut pool, anchor, &schema, &[(0, 2), (1, 3)]);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let (result, _) = runner.run_same_pool(&mut pool, anchor, initial).unwrap();

    let expected = BTreeMap::from([(0, 1), (1, 3)]);
    assert_eq!(
        decompile_map(&pool, &schema, result),
        expected,
        "m(b) at max: placement dropped, m(a) decrement applied"
    );
}

/// M2-5: under `saturate`, the transfer iterates to the fixpoint
/// ({a:2,b:0} → {a:1,b:1} → {a:0,b:2}) and terminates (T2) — the family
/// grows by 1 member per iteration.
#[test]
fn test_transfer_under_saturate_done() {
    let schema = Schema::Map(
        vec![Atom(0), Atom(1)],
        Domain::new(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let initial = compile_map(&mut pool, anchor, &schema, &[(0, 2), (1, 0)]);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let levels: Vec<lisage_algebra::interface::Level> = (0..schema.number_levels())
        .map(lisage_algebra::interface::Level)
        .collect();
    let policy = PolicySaturation::Static(levels);
    let mut params = ParametersSaturation {
        transduction: &transfer,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturer(&mut params).unwrap();
    assert_eq!(result.iterations, 3, "a:2,b:0 → a:1,b:1 → a:0,b:2 → fixe");
    assert_eq!(
        cardinal(&pool, &schema, result.node),
        3,
        "family of the three reached states"
    );
}

/// M2-5, regression: key collision in the remap `v → v-1` on a `val(a)`
/// multi-edge node (family produced by union) — the two subtrees
/// must be **unioned**, not overwritten. Family {{a:0,b:0}, {a:1,b:0}}:
/// the transfer gives {{a:0,b:0}, {a:0,b:1}} (cardinality 2, not 1).
#[test]
fn test_transfer_collision_source_union() {
    let schema = Schema::Map(
        vec![Atom(0), Atom(1)],
        Domain::new(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let m1 = compile_map(&mut pool, anchor, &schema, &[(0, 0), (1, 0)]);
    let m2 = compile_map(&mut pool, anchor, &schema, &[(0, 1), (1, 0)]);
    let family = union(&mut pool, &keys, anchor, m1, m2).unwrap();
    assert_eq!(cardinal(&pool, &schema, family), 2);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let (result, _) = runner.run_same_pool(&mut pool, anchor, family).unwrap();
    assert_eq!(
        cardinal(&pool, &schema, result),
        2,
        "collision v→v-1: both members of the family are kept"
    );
}

/// M2-5, regression: collision in the remap `u → u+1` on a `val(b)` node
/// with multiple edges — edge 2 → key 3 and edge 3 (outside the domain, kept) → key 3.
/// Family {{a:1,b:2}, {a:2,b:3}} → {{a:0,b:3}, {a:1,b:3}} (cardinal 2).
#[test]
fn test_transfer_collision_destination_union() {
    let schema = schema_map();
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let m1 = compile_map(&mut pool, anchor, &schema, &[(0, 1), (1, 2)]);
    let m2 = compile_map(&mut pool, anchor, &schema, &[(0, 2), (1, 3)]);
    let family = union(&mut pool, &keys, anchor, m1, m2).unwrap();
    assert_eq!(cardinal(&pool, &schema, family), 2);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let (result, _) = runner.run_same_pool(&mut pool, anchor, family).unwrap();
    assert_eq!(
        cardinal(&pool, &schema, result),
        2,
        "collision u→u+1: both members of the family are kept"
    );
}

/// M2-5: destination **absent** in a member of a family whose presence node
/// has multiple edges — the ABSENT edge becomes PRESENT with the
/// transferred value (unioned with the existing PRESENT edge). Family
/// {{a:1,b:0,c:2}, {a:1,c:2}} → {a:0,b:1,c:2} (the two members coincide).
#[test]
fn test_transfer_destination_absent_in_family() {
    let schema = schema_map();
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let m1 = compile_map(&mut pool, anchor, &schema, &[(0, 1), (1, 0), (2, 2)]);
    let m2 = compile_map(&mut pool, anchor, &schema, &[(0, 1), (2, 2)]);
    let family = union(&mut pool, &keys, anchor, m1, m2).unwrap();
    assert_eq!(cardinal(&pool, &schema, family), 2);

    let transfer = TransferNotLocal::new(&schema, 0, 1);
    let mut runner = Transducer::with_policy(transfer, PolicyMemory::ByLevel);
    let (result, _) = runner.run_same_pool(&mut pool, anchor, family).unwrap();

    let expected = compile_map(&mut pool, anchor, &schema, &[(0, 0), (1, 1), (2, 2)]);
    assert_eq!(
        result, expected,
        "destination absent in one member: made present with the transferred unit"
    );
}
