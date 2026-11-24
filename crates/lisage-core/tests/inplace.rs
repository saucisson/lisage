//! Tests for `inplace` (`saturate_in_place`, the M2-4 in-place
//! kernel): same least fixpoint as the round kernel `saturate` (T2),
//! termination on the identity step, and equivalence of the two
//! kernels on an emitting step: the fixpoint is canonical, so both
//! kernels must reach the same node id in the same pool.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::count;
use lisage_core::inplace::saturate_in_place;
use lisage_core::observer::NoOpObserver;
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{PhiOutContext, Transduction};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the identity test transduction.
const FINGERPRINT_IDENTITY: u64 = 5001;
/// Fingerprint of the `EmitsSingleton` step transduction.
const FINGERPRINT_EMITS_SINGLETON: u64 = 5002;

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
        FINGERPRINT_IDENTITY
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
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

/// Step transduction emitting the given singleton from any not-terminal
/// input node: `τ(X) = {{emits}}`. The union `X ⊕ τ(X)` grows the
/// family until it contains the singleton.
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
        FINGERPRINT_EMITS_SINGLETON
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

/// Builds a fresh pool for the given schema.
fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms into the pool.
fn compile_set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> NodeId {
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

/// M2-4, T2: the in-place kernel terminates and, on the identity step,
/// the fixpoint is `X ⊕ id(X) = X` — every local fire is a no-op.
#[test]
fn test_in_place_identity_fixpoint() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);
    let initial = compile_set(&mut pool, anchor, &schema, &[0]);

    let policy = PolicySaturation::AutoDetected;
    let transduction = Identity;
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturate_in_place(&mut params).unwrap();
    assert_eq!(result.node, initial, "X plus id(X) equals X (T2)");
    assert!(result.iterations >= 1, "at least one local fire counted");
}

/// M2-4: both kernels reach the **same** fixpoint on an emitting step —
/// canonical form, same node id in the same pool. This is the
/// in-place counterpart of the policy-equivalence test of `saturate`.
#[test]
fn test_in_place_matches_round_kernel() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let initial = compile_set(&mut pool, anchor, &schema, &[0]);
    let singleton = compile_set(&mut pool, anchor, &schema, &[1]);

    // Round kernel (all levels active).
    let policy = PolicySaturation::Static(vec![Level(0), Level(1)]);
    let transduction = EmitsSingleton { singleton };
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let round_result = saturate(&mut params).unwrap();

    // In-place kernel, same pool, same initial node.
    let policy = PolicySaturation::AutoDetected;
    let transduction = EmitsSingleton { singleton };
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let in_place_result = saturate_in_place(&mut params).unwrap();

    assert_eq!(
        round_result.node, in_place_result.node,
        "same canonical fixpoint"
    );
    // The family grew from {{0}} to {{0}, {1}}.
    assert_eq!(cardinality(&pool, &schema, in_place_result.node), 2);
}

/// M2-4 "automatic saturation": the in-place kernel ignores the
/// declared policy (the recursion order replaces it) — an empty
/// `Static` active set still reaches the same fixpoint as the round
/// kernel.
#[test]
fn test_in_place_ignores_policy() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = new_pool(&schema);
    let initial = compile_set(&mut pool, anchor, &schema, &[0]);
    let singleton = compile_set(&mut pool, anchor, &schema, &[1]);

    // Round kernel for reference.
    let policy = PolicySaturation::Static(vec![Level(0), Level(1)]);
    let transduction = EmitsSingleton { singleton };
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let round_result = saturate(&mut params).unwrap();

    // In-place kernel with a policy that would block everything.
    let policy = PolicySaturation::Static(vec![]);
    let transduction = EmitsSingleton { singleton };
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let in_place_result = saturate_in_place(&mut params).unwrap();

    assert_eq!(
        round_result.node, in_place_result.node,
        "the policy is ignored"
    );
    assert_eq!(cardinality(&pool, &schema, in_place_result.node), 2);
}

/// Terminals have no level to fire at: the in-place kernel returns them
/// as-is (degenerate case: the policy restricts every active level).
#[test]
fn test_in_place_terminal_initial() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);
    let initial = pool.intern(anchor, Form::Terminal(false));

    let policy = PolicySaturation::AutoDetected;
    let transduction = EmitsSingleton {
        singleton: pool.intern(anchor, Form::Terminal(true)),
    };
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturate_in_place(&mut params).unwrap();
    assert_eq!(result.node, initial, "no fire from a terminal");
    assert_eq!(result.iterations, 0);
}
