//! Tests of `saturate` (M2-4, spec § 7, T2/T3): increasing fixpoint,
//! termination, restriction by levels of the Static policy, removal
//! of the levels stabilized by the AutoDetectee policy, and increasing
//! multi-iteration chain via `saturer`.
//!
//! The diagrams denote families of sets (ZDD style): the fixpoint of
//! `X ⊕ τ(X)` with a step emitting the singleton `{a}` is
//! the family `{X, {a}}` — so we compare nodes (and cardinalities
//! via `count`), not the decompiled values.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::saturate::{BOUND_ITERATIONS, ParametersSaturation, PolicySaturation, saturer};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_core::transduction::{ContextPhiOut, Transduction};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the test identity transduction.
const FINGERPRINT_IDENTITE: u64 = 4001;
/// Fingerprint of the `EmitsSingleton` step transduction.
const FINGERPRINT_EMET_SINGLETON: u64 = 4002;

type PoolRef = Arena<ReferenceController>;

/// Identity transduction: copies the diagram (union with itself).
#[derive(Clone)]
struct Identity;

impl Transduction for Identity {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_IDENTITE
    }
    fn h0(&self) {}
    fn phi_dispatch(&self, _level: Level, h: &(), _edge: u64) {
        let _ = h;
    }
    fn phi_aggregate(&self, _level: Level, _h: &(), _edges: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    edges: ctx.edges.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memory::EffectMemory::new(),
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

/// Step transduction that always emits the given singleton, whatever the
/// input node: `τ(X) = {{ajout}}` for any not-terminal `X`. The union
/// `X ⊕ {{ajout}}` grows the family `X` until it contains `{ajout}`.
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
        FINGERPRINT_EMET_SINGLETON
    }
    fn h0(&self) {}
    fn phi_dispatch(&self, _level: Level, h: &(), _edge: u64) {
        let _ = h;
    }
    fn phi_aggregate(&self, _level: Level, _h: &(), _edges: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        _pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
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

/// Builds a new pool for the given schema.
fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms into the pool.
fn compile_set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> NodeId {
    compiled(
        pool,
        anchor,
        schema,
        &Value::Set(atoms.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// Cardinality of a diagram (number of members of the denoted family).
fn cardinal(pool: &PoolRef, schema: &Schema, node: NodeId) -> u64 {
    let mut output = new_pool(schema);
    count(pool, &mut output, Anchor(0), schema, node).unwrap()
}

/// M2-4, T2: `saturer` terminates and reaches a fixpoint — with an
/// identity step, the fixpoint is immediate (1 iteration) and `X ⊕ id(X) = X`.
#[test]
fn test_saturate_done_and_point_fixe() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);
    let initial = compile_set(&mut pool, anchor, &schema, &[0]);

    let policy = PolicySaturation::AutoDetectee;
    let mut params = ParametersSaturation {
        transduction: &Identity,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturer(&mut params).unwrap();
    assert_eq!(result.iterations, 1, "identity: immediate fixpoint (T2)");
    assert_eq!(result.node, initial, "X ⊕ id(X) = X");
}

/// M2-4: the two policies reach the same fixpoint when
/// `Static` declares all the levels of the schema.
#[test]
fn test_saturate_policies_equivalent() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy_static = PolicySaturation::Static(levels);
    let policy_auto = PolicySaturation::AutoDetectee;

    let mut pool_s = new_pool(&schema);
    let initial_s = compile_set(&mut pool_s, anchor, &schema, &[0]);
    let mut params = ParametersSaturation {
        transduction: &Identity,
        policy: &policy_static,
        pool: &mut pool_s,
        keys: &keys,
        anchor,
        initial: initial_s,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_static = saturer(&mut params).unwrap();

    let mut pool_a = new_pool(&schema);
    let initial_a = compile_set(&mut pool_a, anchor, &schema, &[0]);
    let mut params = ParametersSaturation {
        transduction: &Identity,
        policy: &policy_auto,
        pool: &mut pool_a,
        keys: &keys,
        anchor,
        initial: initial_a,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_auto = saturer(&mut params).unwrap();

    assert_eq!(r_static.node, initial_s, "static : point fixe = initial");
    assert_eq!(r_auto.node, initial_a, "auto-detected: fixpoint = initial");
    assert_eq!(r_static.iterations, r_auto.iterations);
}

/// M2-4: the `Static` policy really restricts the active levels —
/// with a step that emits a singleton at the root (level 0), excluding
/// level 0 from the static list leaves the fixpoint unchanged, while
/// `AutoDetectee` (all levels active) reaches it with the singleton added.
/// This test fails if the `policy` parameter is ignored.
#[test]
fn test_saturate_static_restricted_the_levels() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    // Static policy excluding the root level 0.
    let levels_without_root: Vec<Level> = (1..schema.number_levels()).map(Level).collect();
    let policy_static = PolicySaturation::Static(levels_without_root);
    let mut pool_s = new_pool(&schema);
    let initial_s = compile_set(&mut pool_s, anchor, &schema, &[0]);
    let singleton_s = compile_set(&mut pool_s, anchor, &schema, &[2]);
    let step_s = EmitsSingleton {
        singleton: singleton_s,
    };
    let mut params = ParametersSaturation {
        transduction: &step_s,
        policy: &policy_static,
        pool: &mut pool_s,
        keys: &keys,
        anchor,
        initial: initial_s,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_static = saturer(&mut params).unwrap();
    assert_eq!(
        r_static.node, initial_s,
        "root level excluded: the step is not applied there, fixpoint = initial"
    );
    assert_eq!(r_static.iterations, 1);

    // AutoDetected policy: all levels active, the singleton is
    // reached.
    let policy_auto = PolicySaturation::AutoDetectee;
    let mut pool_a = new_pool(&schema);
    let initial_a = compile_set(&mut pool_a, anchor, &schema, &[0]);
    let singleton_a = compile_set(&mut pool_a, anchor, &schema, &[2]);
    let step_a = EmitsSingleton {
        singleton: singleton_a,
    };
    let expected_a = union(&mut pool_a, &keys, anchor, initial_a, singleton_a).unwrap();
    let mut params = ParametersSaturation {
        transduction: &step_a,
        policy: &policy_auto,
        pool: &mut pool_a,
        keys: &keys,
        anchor,
        initial: initial_a,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_auto = saturer(&mut params).unwrap();
    assert_eq!(
        r_auto.node, expected_a,
        "all levels active: the fixpoint contains the emitted singleton"
    );
    assert!(
        r_auto.iterations >= 2,
        "increasing chain: several iterations, got {}",
        r_auto.iterations
    );
}

/// M2-4, T2: `saturer` with a not-trivial step — increasing chain
/// `{{0}} → {{0},{2}}` by calling `saturer` (several iterations of the
/// fixpoint loop, not a hand-rewritten loop).
#[test]
fn test_saturate_string_increasing_multi_iterations() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);

    let initial = compile_set(&mut pool, anchor, &schema, &[0]);
    let singleton_2 = compile_set(&mut pool, anchor, &schema, &[2]);
    let expected = union(&mut pool, &keys, anchor, initial, singleton_2).unwrap();
    let step = EmitsSingleton {
        singleton: singleton_2,
    };
    let policy = PolicySaturation::AutoDetectee;
    let mut params = ParametersSaturation {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturer(&mut params).unwrap();
    assert!(
        result.iterations >= 2,
        "increasing chain: several iterations, got {}",
        result.iterations
    );
    assert!(result.iterations <= BOUND_ITERATIONS, "terminaison (T2)");
    assert_eq!(
        result.node, expected,
        "fixpoint = union of the initial and the singleton"
    );
    assert_eq!(cardinal(&pool, &schema, result.node), 2);
}
