//! Tests for `saturate` (M2-4, spec § 7, T2/T3): increasing fixpoint,
//! termination, level restriction of the Static policy, removal of
//! stabilized levels by the AutoDetected policy, and multi-iteration
//! increasing chain via `saturate`.
//!
//! The diagrams denote families of sets (ZDD style): the fixpoint of
//! `X ⊕ τ(X)` with a step emitting the singleton `{a}` is the family
//! `{X, {a}}` — so we compare nodes (and cardinalities via `count`),
//! not decompiled values.

use std::collections::BTreeMap;
use std::sync::Arc;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::saturate::{BOUND_ITERATIONS, PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{PhiOutContext, Transduction};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the identity test transduction.
const FINGERPRINT_IDENTITY: u64 = 4001;
/// Fingerprint of the `EmitsSingleton` step transduction.
const FINGERPRINT_EMITS_SINGLETON: u64 = 4002;

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

/// Step transduction that always emits the given singleton, whatever the
/// input node: `τ(X) = {{added}}` for any not-terminal `X`. The union
/// `X ⊕ {{added}}` grows the family `X` until it contains `{added}`.
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

/// M2-4, T2: `saturate` terminates and reaches a fixpoint — with an identity
/// step, the fixpoint is immediate (1 iteration) and `X ⊕ id(X) = X`.
#[test]
fn test_saturate_terminates_and_point_fixed() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);
    let initial = compiler_set(&mut pool, anchor, &schema, &[0]);

    let policy = PolicySaturation::AutoDetected;
    let mut params = SaturationParameters {
        transduction: &Identity,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturate(&mut params).unwrap();
    assert_eq!(result.iterations, 1, "identity: immediate fixpoint (T2)");
    assert_eq!(result.node, initial, "X ⊕ id(X) = X");
}

/// M2-4: both policies reach the same fixpoint when `Static` declares
/// all levels of the schema.
#[test]
fn test_saturate_policies_equivalentes() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy_static = PolicySaturation::Static(levels);
    let policy_auto = PolicySaturation::AutoDetected;

    let mut pool_s = new_pool(&schema);
    let initial_s = compiler_set(&mut pool_s, anchor, &schema, &[0]);
    let mut params = SaturationParameters {
        transduction: &Identity,
        policy: &policy_static,
        pool: &mut pool_s,
        keys: &keys,
        anchor,
        initial: initial_s,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_static = saturate(&mut params).unwrap();

    let mut pool_a = new_pool(&schema);
    let initial_a = compiler_set(&mut pool_a, anchor, &schema, &[0]);
    let mut params = SaturationParameters {
        transduction: &Identity,
        policy: &policy_auto,
        pool: &mut pool_a,
        keys: &keys,
        anchor,
        initial: initial_a,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_auto = saturate(&mut params).unwrap();

    assert_eq!(r_static.node, initial_s, "static: fixpoint = initial");
    assert_eq!(r_auto.node, initial_a, "auto-detected: fixpoint = initial");
    assert_eq!(r_static.iterations, r_auto.iterations);
}

/// M2-4: the `Static` policy really restricts the active levels — with a
/// step that emits a singleton at the root (level 0), excluding level 0
/// from the static list leaves the fixpoint unchanged, while `AutoDetected`
/// (all levels active) reaches it with the singleton added. This test fails
/// if the `policy` parameter is ignored.
#[test]
fn test_saturate_static_restricted_the_levels() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    // Static policy excluding the root level 0.
    let levels_without_root: Vec<Level> = (1..schema.number_levels()).map(Level).collect();
    let policy_static = PolicySaturation::Static(levels_without_root);
    let mut pool_s = new_pool(&schema);
    let initial_s = compiler_set(&mut pool_s, anchor, &schema, &[0]);
    let singleton_s = compiler_set(&mut pool_s, anchor, &schema, &[2]);
    let step_s = EmitsSingleton {
        singleton: singleton_s,
    };
    let mut params = SaturationParameters {
        transduction: &step_s,
        policy: &policy_static,
        pool: &mut pool_s,
        keys: &keys,
        anchor,
        initial: initial_s,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_static = saturate(&mut params).unwrap();
    assert_eq!(
        r_static.node, initial_s,
        "root level excluded: the step is not applied there, fixpoint = initial"
    );
    assert_eq!(r_static.iterations, 1);

    // AutoDetected policy: all levels active, the singleton is reached.
    let policy_auto = PolicySaturation::AutoDetected;
    let mut pool_a = new_pool(&schema);
    let initial_a = compiler_set(&mut pool_a, anchor, &schema, &[0]);
    let singleton_a = compiler_set(&mut pool_a, anchor, &schema, &[2]);
    let step_a = EmitsSingleton {
        singleton: singleton_a,
    };
    let expected_a = union(&mut pool_a, &keys, anchor, initial_a, singleton_a).unwrap();
    let mut params = SaturationParameters {
        transduction: &step_a,
        policy: &policy_auto,
        pool: &mut pool_a,
        keys: &keys,
        anchor,
        initial: initial_a,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let r_auto = saturate(&mut params).unwrap();
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

/// M2-4, T2: `saturate` with a not-trivial step — increasing chain
/// `{{0}} → {{0},{2}}` by calling `saturate` (several iterations of the
/// fixpoint loop, not a hand-rewritten loop).
#[test]
fn test_saturate_string_growing_multi_iterations() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = new_pool(&schema);
    let anchor = Anchor(0);

    let initial = compiler_set(&mut pool, anchor, &schema, &[0]);
    let singleton_2 = compiler_set(&mut pool, anchor, &schema, &[2]);
    let expected = union(&mut pool, &keys, anchor, initial, singleton_2).unwrap();
    let step = EmitsSingleton {
        singleton: singleton_2,
    };
    let policy = PolicySaturation::AutoDetected;
    let mut params = SaturationParameters {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let result = saturate(&mut params).unwrap();
    assert!(
        result.iterations >= 2,
        "increasing chain: several iterations, got {}",
        result.iterations
    );
    assert!(result.iterations <= BOUND_ITERATIONS, "termination (T2)");
    assert_eq!(
        result.node, expected,
        "fixpoint = union of the initial and the singleton"
    );
    assert_eq!(cardinality(&pool, &schema, result.node), 2);
}

/// Observer of the cross-round memo reuse (M2-5, the #388 core
/// remainder): records the span ids of the Round spans and the span
/// ids of the memo hits — the span ids are globally monotone, so a hit
/// whose span id passes the span of the second round answered from an
/// entry built in a PREVIOUS round (the MemoHit carries the
/// innermost Visit span, opened inside the round).
#[derive(Clone, Default)]
struct RoundsAndHits {
    round_spans: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
    hits: std::sync::Arc<std::sync::Mutex<Vec<Option<lisage_core::observer::SpanId>>>>,
}

impl lisage_core::observer::Observer for RoundsAndHits {
    fn on_event(&self, event: &lisage_core::observer::TraceEvent) {
        use lisage_core::observer::TraceEvent;
        match event {
            TraceEvent::SpanOpen { span, kind, .. }
                if *kind == lisage_core::observer::SpanKind::Round =>
            {
                self.round_spans
                    .lock()
                    .expect("collector lock")
                    .push(span.0);
            }
            TraceEvent::MemoHit { span, .. } => {
                self.hits.lock().expect("collector lock").push(*span);
            }
            _ => {}
        }
    }
}

/// M2-5, the #388 saturation remainder: ONE runner for the whole
/// fixpoint — the computed table of one round answers in the NEXT
/// ones (the per-fragment versions of `MemoryLevel` carry the validity
/// of the entries, the folded wrapper fingerprint the active set).
/// Measured: the increasing chain saturates to the same fixpoint in
/// the same number of iterations whether observed or not, and at
/// least one memo hit of the run answers with a span PAST the span of
/// the second round — an entry built in the first round surviving into
/// the confirming one.
#[test]
fn test_saturate_reuses_memo_across_rounds() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    // Unobserved baseline: the fixpoint and its iterations.
    let mut pool = new_pool(&schema);
    let initial = compiler_set(&mut pool, anchor, &schema, &[0]);
    let singleton = compiler_set(&mut pool, anchor, &schema, &[2]);
    let step_plain = EmitsSingleton { singleton };
    let policy = PolicySaturation::AutoDetectee;
    let mut params = SaturationParameters {
        transduction: &step_plain,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: None,
    };
    let plain = saturate(&mut params).unwrap();
    assert!(
        plain.iterations >= 2,
        "the increasing chain takes several rounds: {}",
        plain.iterations
    );

    // Observed run, same pool, same initial: hash-consing reproduces
    // the identical fixpoint in the identical number of rounds.
    let collector = RoundsAndHits::default();
    let observer: Arc<dyn lisage_core::observer::Observer> = Arc::new(collector.clone());
    let step = EmitsSingleton { singleton };
    let mut params = SaturationParameters {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(observer),
    };
    let observed = saturate(&mut params).unwrap();
    assert_eq!(observed.node, plain.node, "same fixpoint, same pool");
    assert_eq!(
        observed.iterations, plain.iterations,
        "same schedule with the persistent runner"
    );

    // The cross-round reuse: at least one memo hit answers with a
    // span id past the span of the second round — a round-two visit
    // served by a round-one entry.
    let round_spans = collector.round_spans.lock().expect("collector lock");
    let hits = collector.hits.lock().expect("collector lock");
    assert!(
        round_spans.len() >= 2,
        "one span per round: {:?}",
        round_spans
    );
    let late: Vec<_> = hits
        .iter()
        .filter(|span| span.is_some_and(|s| s.0 > round_spans[1]))
        .collect();
    assert!(
        !late.is_empty(),
        "the memo answers across the rounds: {} hits, none past the second round span",
        hits.len()
    );
}
