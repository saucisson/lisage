//! P0-2 (issue #137): mono vs multi equivalence harness (PAR-1..PAR-4).
//!
//! Property under test: on the same inputs, the multi-thread executor and
//! the mono-thread executor produce the same node counts and the same
//! semantics (cardinality of the fixpoint, structural form of the root).
//! The harness is written red before any multi-thread implementation: the
//! `MultiExecutor` leg is a placeholder that delegates to the mono path, so
//! the harness is green on the current mono-thread (DoD) and becomes the
//! falsifier for PAR-1..PAR-4 as soon as a real parallel executor (P1-1,
//! P2-1, P4-1) replaces the placeholder — see the ignored marker test at the
//! bottom of this file.
//!
//! Survey of existing NodeId-comparing tests (issue #137): within one pool,
//! NodeId equality is the canonicity property C1 and stays valid under
//! multi-thread (`schemas.rs`, `schema.rs`, `cdd.rs`, `ddd.rs`,
//! `liseuse_v0.rs`, `classes.rs`, `dense.rs` — same-pool comparisons, kept).
//! Across pools, NodeIds are pool-local: `transductions.rs` already compares
//! forms (`forms_egales`), and `handles.rs` compared bare NodeIds across two
//! arenas — requalified in this change as a form comparison (the handle
//! comparison next to it remains the spec-level C1 assertion).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};
use lisage_core::pli::{count, union};
use lisage_core::saturate::{ParametresSaturation, PolicySaturation, saturer};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

const ANCHOR: Anchor = Anchor(0);
/// Fingerprint of the fixed saturation step transduction.
const STEP_FINGERPRINT: u64 = 6002;

/// A workload: a bounded set schema and a list of members (subsets of the
/// domain) to compile and union into R, then saturate and count.
#[derive(Debug)]
struct Workload {
    domain_size: u64,
    members: Vec<Vec<u64>>,
}

/// Outcome of one execution of a workload. `root` is pool-local: compare
/// roots across executions with [`forms_equal`], never by NodeId.
struct Outcome {
    live: usize,
    cardinality: u64,
    iterations: u32,
    root: NodeId,
}

/// Executor of a workload. The mono executor is the current single-threaded
/// `Arena` path; the multi executor is a placeholder until a real parallel
/// implementation lands (P1-1/P2-1).
trait Executor {
    /// Whether this executor really runs multi-threaded (red marker for
    /// issue #137: false while the multi leg is the mono placeholder).
    fn is_parallel(&self) -> bool;
    fn run(&self, workload: &Workload) -> (PoolRef, Outcome);
}

/// Fixed saturation step: always emits one fixed member (terminals → ⊥),
/// like the PBT step of `saturate.rs`.
#[derive(Clone)]
struct EmitSingleton {
    singleton: NodeId,
}

impl Transduction for EmitSingleton {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        STEP_FINGERPRINT
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        _pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<()>) {
        let _ = ctx;
        (self.singleton, lisage_core::memory::EffetMemory::new())
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

/// Counting observer: rounds only (the structural counters come from the
/// pool metrics).
#[derive(Default)]
struct RoundCounter {
    rounds: Mutex<u32>,
}

impl Observer for RoundCounter {
    fn on_event(&self, event: &TraceEvent) {
        if let TraceEvent::Round { .. } = event {
            *self.rounds.lock().unwrap() += 1;
        }
    }
}

/// Runs a workload on a fresh pool: compile + union the members into R,
/// saturate with the fixed singleton step, count the fixpoint.
fn run_workload(workload: &Workload, observer: Option<Arc<dyn Observer>>) -> (PoolRef, Outcome) {
    let schema = Schema::Set(Domain::new((0..workload.domain_size).map(Atom).collect()));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );

    let mut root = pool.intern(ANCHOR, Form::Terminal(false));
    for member in &workload.members {
        let value = Value::Set(
            member
                .iter()
                .map(|&a| Value::Atom(Atom(a)))
                .collect::<BTreeSet<_>>(),
        );
        let node = compiler(&mut pool, ANCHOR, &schema, &value).unwrap();
        root = union(&mut pool, &keys, ANCHOR, root, node).unwrap();
    }

    // Saturation step: one fixed member (the full domain).
    let full = Value::Set(
        (0..workload.domain_size)
            .map(|a| Value::Atom(Atom(a)))
            .collect::<BTreeSet<_>>(),
    );
    let singleton = compiler(&mut pool, ANCHOR, &schema, &full).unwrap();
    let step = EmitSingleton { singleton };
    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy = PolicySaturation::Statique(levels);
    let mut params = ParametresSaturation {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor: ANCHOR,
        initial: root,
        number_levels: schema.number_levels(),
        observer,
    };
    let result = saturer(&mut params).expect("saturation terminates");

    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let cardinality = count(&pool, &mut output, ANCHOR, &schema, result.node).unwrap();
    let live = pool.live();
    (
        pool,
        Outcome {
            live,
            cardinality,
            iterations: result.iterations,
            root: result.node,
        },
    )
}

/// The current mono-thread executor.
struct MonoExecutor;

impl Executor for MonoExecutor {
    fn is_parallel(&self) -> bool {
        false
    }
    fn run(&self, workload: &Workload) -> (PoolRef, Outcome) {
        run_workload(workload, None)
    }
}

/// The multi-thread executor. Placeholder until P1-1/P2-1 land a real
/// parallel executor: delegates to the mono path, so the harness is green on
/// the current mono-thread (DoD of issue #137) and turns into the
/// falsifier for PAR-1..PAR-4 when replaced.
struct MultiExecutor;

impl Executor for MultiExecutor {
    fn is_parallel(&self) -> bool {
        false
    }
    fn run(&self, workload: &Workload) -> (PoolRef, Outcome) {
        run_workload(workload, None)
    }
}

/// Structural equality of two forms in two distinct pools (the NodeIds are
/// specific to each pool).
fn forms_equal(
    pool_a: &PoolRef,
    node_a: NodeId,
    pool_b: &PoolRef,
    node_b: NodeId,
    visited: &mut HashMap<(NodeId, NodeId), bool>,
) -> bool {
    if let Some(&seen) = visited.get(&(node_a, node_b)) {
        return seen;
    }
    let equal = match (
        pool_a.resolve(node_a).unwrap(),
        pool_b.resolve(node_b).unwrap(),
    ) {
        (Form::Terminal(payload_a), Form::Terminal(payload_b)) => payload_a == payload_b,
        (
            Form::Node {
                level: level_a,
                arcs: arcs_a,
            },
            Form::Node {
                level: level_b,
                arcs: arcs_b,
            },
        ) => {
            level_a == level_b
                && arcs_a.len() == arcs_b.len()
                && arcs_a.iter().zip(arcs_b.iter()).all(
                    |((value_a, child_a), (value_b, child_b))| {
                        value_a == value_b
                            && forms_equal(pool_a, *child_a, pool_b, *child_b, visited)
                    },
                )
        }
        _ => false,
    };
    visited.insert((node_a, node_b), equal);
    equal
}

fn workload_strategy() -> impl Strategy<Value = Workload> {
    (
        2u64..=4,
        prop::collection::vec(prop::collection::vec(0u64..4, 0..5), 0..6),
    )
        .prop_map(|(domain_size, members)| Workload {
            domain_size,
            members: members
                .into_iter()
                .map(|m| m.into_iter().filter(|&a| a < domain_size).collect())
                .collect(),
        })
}

proptest! {
    /// PAR-1..PAR-4 harness: on the same inputs, mono and multi agree on
    /// node counts, fixpoint iterations, semantics (cardinality) and the
    /// structural form of the root.
    #[test]
    fn test_mono_multi_equivalence(workload in workload_strategy()) {
        let (pool_mono, out_mono) = MonoExecutor.run(&workload);
        let (pool_multi, out_multi) = MultiExecutor.run(&workload);
        prop_assert_eq!(out_mono.live, out_multi.live, "same live node count");
        prop_assert_eq!(out_mono.iterations, out_multi.iterations, "same fixpoint iterations");
        prop_assert_eq!(out_mono.cardinality, out_multi.cardinality, "same semantics (cardinality)");
        prop_assert!(
            forms_equal(&pool_mono, out_mono.root, &pool_multi, out_multi.root, &mut HashMap::new()),
            "same structural form of the fixpoint root"
        );
    }

    /// The saturation leg is exercised: rounds actually happen and the
    /// fixpoint grows past R (or stays equal when the step member is
    /// already present) — guards against a vacuous harness.
    #[test]
    fn test_harness_is_not_vacuous(workload in workload_strategy()) {
        let counter = Arc::new(RoundCounter::default());
        struct Bridge(Arc<RoundCounter>);
        impl Observer for Bridge {
            fn on_event(&self, event: &TraceEvent) {
                self.0.on_event(event);
            }
        }
        let (_, outcome) = run_workload(&workload, Some(Arc::new(Bridge(counter.clone()))));
        prop_assert!(outcome.iterations >= 1, "at least one round");
        prop_assert!(*counter.rounds.lock().unwrap() >= 1, "rounds observed");
        prop_assert!(outcome.live >= 1, "the pool is not empty");
    }
}

/// Red marker (issue #137, "written red before any implementation"): fails
/// while `MultiExecutor` is still the mono placeholder; un-ignore when a
/// real multi-thread executor (P1-1/P2-1) lands.
#[test]
#[ignore = "red until a real multi-thread executor exists (issue #137)"]
fn test_multi_executor_is_parallel() {
    assert!(
        MultiExecutor.is_parallel(),
        "MultiExecutor must be a real parallel executor"
    );
}

// ---------------------------------------------------------------------------
// P1-1 (issue #139): independent operations on a read-shared input pool.
// ---------------------------------------------------------------------------

/// Counting transduction: synthesized attribute = number of accepting
/// paths (independent oracle for the semantics).
#[derive(Clone)]
struct CountPaths;

impl Transduction for CountPaths {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        7002
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}
    fn phi_aggr(&self, _level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).sum()
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memory::EffetMemory::new(),
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, u64) {
        (
            pool.intern(anchor, Form::Terminal(payload)),
            u64::from(payload),
        )
    }
}

/// Builds a random set-diagram in a fresh pool: union of random members.
/// Returns the pool, its schema, the union root and the member roots.
fn random_input_pool(workload: &Workload) -> (PoolRef, Schema, NodeId, Vec<NodeId>) {
    let schema = Schema::Set(Domain::new((0..workload.domain_size).map(Atom).collect()));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut root = pool.intern(ANCHOR, Form::Terminal(false));
    let mut members = Vec::new();
    for member in &workload.members {
        let value = Value::Set(
            member
                .iter()
                .map(|&a| Value::Atom(Atom(a)))
                .collect::<BTreeSet<_>>(),
        );
        let node = compiler(&mut pool, ANCHOR, &schema, &value).unwrap();
        root = union(&mut pool, &keys, ANCHOR, root, node).unwrap();
        members.push(node);
    }
    (pool, schema, root, members)
}

proptest! {
    /// P1-1 (issue #139, PAR-1): n independent operations on the same
    /// read-shared input pool — the sequential path and the rayon parallel
    /// path agree on every synthesized attribute and on the structural form
    /// of every output root (each op in its own output pool, INV10).
    #[test]
    fn test_p1_1_independent_ops_equivalence(
        workload in workload_strategy(),
        number_ops in 1usize..6,
    ) {
        use lisage_core::parallel_ops::{
            IndependentOp, run_independent_par, run_independent_seq,
        };
        let (input, schema, root, members) = random_input_pool(&workload);
        // Each op counts the accepting paths of a root of the shared input
        // pool (the union root, or a member root when available).
        let roots: Vec<NodeId> = (0..number_ops)
            .map(|i| {
                if members.is_empty() || i % 2 == 0 {
                    root
                } else {
                    members[i % members.len()]
                }
            })
            .collect();
        let ops: Vec<IndependentOp<CountPaths>> = roots
            .iter()
            .map(|&root| IndependentOp { transduction: CountPaths, root })
            .collect();
        let seq = run_independent_seq(&input, &schema, ANCHOR, &ops).unwrap();
        let par = run_independent_par(&input, &schema, ANCHOR, &ops).unwrap();
        prop_assert_eq!(seq.len(), par.len());
        prop_assert_eq!(seq.len(), number_ops);
        for (i, ((pool_seq, node_seq, attr_seq), (pool_par, node_par, attr_par))) in
            seq.iter().zip(par.iter()).enumerate()
        {
            prop_assert_eq!(attr_seq, attr_par, "op {}: same synthesized attribute", i);
            prop_assert!(
                forms_equal(pool_seq, *node_seq, pool_par, *node_par, &mut HashMap::new()),
                "op {}: same structural form of the output root", i
            );
        }
    }
}
