//! Tests for `inplace` (`saturate_in_place`, the M2-4 in-place
//! kernel): same least fixpoint as the round kernel `saturate` (T2),
//! termination on the identity step, and equivalence of the two
//! kernels on an emitting step: the fixpoint is canonical, so both
//! kernels must reach the same node id in the same pool.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::count;
use lisage_core::inplace::saturate_in_place;
use lisage_core::observer::{NoOpObserver, Observer, SpanId, SpanKind, TraceEvent};
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{PhiOutContext, Transduction};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the identity test transduction.
const FINGERPRINT_IDENTITY: u64 = 5001;
/// Fingerprint of the `EmitsSingleton` step transduction.
const FINGERPRINT_EMITS_SINGLETON: u64 = 5002;

type PoolRef = Arena<ReferenceController>;

/// Trace collector for the firing-order tests (ADR-0109): records every
/// event in emission order.
#[derive(Clone, Default)]
struct RecordingObserver {
    /// Events collected so far, in emission order.
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Observer for RecordingObserver {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

impl RecordingObserver {
    /// Snapshot of the collected events.
    fn collected(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("mutex poisoned").clone()
    }
}

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

/// Runs the in-place kernel on the `EmitsSingleton` step with the
/// recording observer, and returns (result, events).
fn run_traced(
    pool: &mut PoolRef,
    keys: &SchemaKeys,
    schema: &Schema,
    initial: NodeId,
    singleton: NodeId,
) -> (lisage_core::saturate::ResultSaturation, Vec<TraceEvent>) {
    let recorder = Arc::new(RecordingObserver::default());
    let policy = PolicySaturation::AutoDetected;
    let transduction = EmitsSingleton { singleton };
    let mut params = SaturationParameters {
        transduction: &transduction,
        policy: &policy,
        pool,
        keys,
        anchor: Anchor(0),
        initial,
        number_levels: schema.number_levels(),
        observer: Some(recorder.clone()),
    };
    let result = saturate_in_place(&mut params).unwrap();
    let events = recorder.collected();
    (result, events)
}

/// ADR-0109 (trace of the firing order): the kernel emits exactly one
/// `LocalFire` event per local fire, numbered from 1 in firing order,
/// each inside its own `Fire` span; the spans of the whole trace are
/// properly nested (every `SpanClose` pops the innermost open span,
/// nothing stays open at the end).
#[test]
fn test_in_place_trace_of_the_firing_order() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);
    let initial = compile_set(&mut pool, anchor, &schema, &[0]);
    let singleton = compile_set(&mut pool, anchor, &schema, &[1]);

    let (result, events) = run_traced(&mut pool, &keys, &schema, initial, singleton);

    let fires: Vec<&TraceEvent> = events
        .iter()
        .filter(|event| matches!(event, TraceEvent::LocalFire { .. }))
        .collect();
    assert_eq!(
        fires.len() as u32,
        result.iterations,
        "one LocalFire event per local fire"
    );
    for (position, event) in fires.iter().enumerate() {
        let TraceEvent::LocalFire { number, level, .. } = event else {
            unreachable!("filtered to LocalFire above");
        };
        assert_eq!(
            *number,
            position as u32 + 1,
            "fires are numbered from 1, in firing order"
        );
        assert!(
            *level < schema.number_levels(),
            "the fire happens at a level of the schema"
        );
    }
    let fire_spans = events
        .iter()
        .filter(|event| matches!(event, TraceEvent::SpanOpen { kind: SpanKind::Fire, .. }))
        .count();
    assert_eq!(fire_spans, fires.len(), "one Fire span per local fire");
    // D1-1 over the whole emission: proper nesting, innermost close.
    let mut open: Vec<SpanId> = Vec::new();
    for event in &events {
        match event {
            TraceEvent::SpanOpen { span, .. } => open.push(*span),
            TraceEvent::SpanClose { span } => {
                assert_eq!(
                    open.pop(),
                    Some(*span),
                    "SpanClose pops the innermost open span"
                );
            }
            _ => {}
        }
    }
    assert!(open.is_empty(), "no span stays open at the end");
    // Every fire event carries the `Fire` span in which it was emitted.
    for event in &events {
        let TraceEvent::LocalFire { span, .. } = event else {
            continue;
        };
        let span = span.expect("the LocalFire event carries its Fire span");
        let opened = events.iter().any(|candidate| {
            matches!(
                candidate,
                TraceEvent::SpanOpen { span: candidate_span, kind: SpanKind::Fire, .. }
                    if *candidate_span == span
            )
        });
        assert!(opened, "the fire's span is a Fire span opened in the trace");
    }
}

/// ADR-0109: the last fire of the kernel is the sterile fire that
/// detects the root's local fixpoint (`merged == before`), and its
/// number is the final `iterations` count.
#[test]
fn test_in_place_last_fire_is_sterile() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);
    let initial = compile_set(&mut pool, anchor, &schema, &[0]);
    let singleton = compile_set(&mut pool, anchor, &schema, &[1]);

    let (result, events) = run_traced(&mut pool, &keys, &schema, initial, singleton);

    let last = events
        .iter()
        .rev()
        .find(|event| matches!(event, TraceEvent::LocalFire { .. }))
        .expect("at least one local fire");
    let TraceEvent::LocalFire {
        number,
        before,
        merged,
        ..
    } = last
    else {
        unreachable!("filtered to LocalFire above");
    };
    assert_eq!(
        before, merged,
        "the last fire changes nothing (the root reached its local fixpoint)"
    );
    assert_eq!(*number, result.iterations, "the last fire carries the count");
}
