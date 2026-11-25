//! PBT of `saturate` (M2-4, T2): fixpoint invariant — `saturate`
//! applied to its own result terminates in **one** iteration with the
//! same node (idempotence), and the cardinality of the fixpoint is greater or
//! equal to that of the initial (increasing chain for the inclusion of the
//! denoted families).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::count;
use lisage_core::observer::NoOpObserver;
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{PhiOutContext, Transduction};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

/// Size of the atom domain of the random schemas.
const SIZE_DOMAIN: u64 = 3;
/// Fingerprint of the PBT step transduction.
const FINGERPRINT_STEP: u64 = 4003;

type PoolRef = Arena<ReferenceController>;

/// Step transduction: always emits the singleton `{ajout}` whatever the
/// input node (terminals → ⊥).
#[derive(Clone)]
struct EmitsSingleton {
    singleton: NodeId,
}

impl Transduction for EmitsSingleton {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_STEP
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        _pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<()>) {
        let _ = ctx;
        (self.singleton, lisage_core::memory::EffectMemory::new())
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        _payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(false)), ())
    }
}

/// `Set` schema over the domain `{0, 1, 2}`.
fn schema_test() -> Schema {
    Schema::Set(Domain::new((0..SIZE_DOMAIN).map(Atom).collect()))
}

/// Builds a fresh pool for the given schema.
fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms into the pool.
fn compiler_set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atoms.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// Cardinality of a diagram (number of members of the denoted family).
fn cardinality(pool: &PoolRef, schema: &Schema, node: NodeId) -> u64 {
    let mut output = new_pool(schema);
    count(pool, &mut output, Anchor(0), schema, node).unwrap()
}

proptest! {
    /// Fixpoint invariant (T2): let `R = saturate(τ, D)`; then
    /// `saturate(τ, R)` terminates in 1 iteration with the same node `R`
    /// (idempotence), and the cardinality of `R` is greater or equal to that
    /// of `D` (increasing chain).
    #[test]
    fn test_saturate_point_fixed_idempotent(
        initial in prop::collection::vec(0u64..SIZE_DOMAIN, 0..SIZE_DOMAIN as usize),
        ajout in 0u64..SIZE_DOMAIN,
    ) {
        let schema = schema_test();
        let keys = SchemaKeys::new(schema.clone());
        let anchor = Anchor(0);
        let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
        let policy = PolicySaturation::Static(levels);

        let mut pool = new_pool(&schema);
        let node_initial = compiler_set(&mut pool, anchor, &schema, &initial);
        let singleton = compiler_set(&mut pool, anchor, &schema, &[ajout]);
        let step = EmitsSingleton { singleton };

        let mut params = SaturationParameters {
            transduction: &step,
            policy: &policy,
            pool: &mut pool,
            keys: &keys,
            anchor,
            initial: node_initial,
            number_levels: schema.number_levels(),
            observer: None,
        };
        let first = saturate(&mut params).unwrap();

        // Increasing chain: the fixpoint contains the initial family.
        let cardinality_initial = cardinality(&pool, &schema, node_initial);
        let cardinality_fixed = cardinality(&pool, &schema, first.node);
        prop_assert!(
            cardinality_fixed >= cardinality_initial,
            "fixpoint ⊇ initial: {cardinality_fixed} >= {cardinality_initial}"
        );

        // Idempotence: saturate applied to the result terminates in 1 iteration
        // with the same node.
        let mut params = SaturationParameters {
            transduction: &step,
            policy: &policy,
            pool: &mut pool,
            keys: &keys,
            anchor,
            initial: first.node,
            number_levels: schema.number_levels(),
            observer: None,
        };
        let second = saturate(&mut params).unwrap();
        prop_assert_eq!(second.iterations, 1, "fixpoint: 1 iteration");
        prop_assert_eq!(second.node, first.node, "fixpoint idempotence");
    }
}
