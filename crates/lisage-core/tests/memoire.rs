//! Level memory tests (M2-3, spec § 3.1, statement T3): storage
//! between two visits of the same level, versioning (L1), memory effect
//! produced by φ_out, and policy declared by the controller.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Controller, Form, Level, NodeId, PolicyMemory, Pool};
use lisage_core::arena::Arena;
use lisage_core::memory::{EffectMemory, MemoryLevel};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{ContextPhiOut, Transducer, Transduction};
use lisage_lang::value::Atom;

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// M2-3: the level memory stores and versions (L1).
#[test]
fn test_memory_level_stockage_and_version() {
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    assert_eq!(memory.version(), 0);
    assert_eq!(memory.read(NodeId(0), Level(0)), None);

    let previous = memory.write(NodeId(0), Level(0), 42);
    assert_eq!(previous, None);
    assert_eq!(memory.version(), 1, "each write increments the version");
    assert_eq!(memory.read(NodeId(0), Level(0)), Some(&42));

    let previous = memory.write(NodeId(0), Level(0), 43);
    assert_eq!(previous, Some(42), "the write returns the old value");
    assert_eq!(memory.version(), 2);
    assert_eq!(memory.read(NodeId(0), Level(0)), Some(&43));
    assert_eq!(
        memory.read(NodeId(1), Level(0)),
        None,
        "keyed by (node, level)"
    );
    assert_eq!(memory.size(), 1);
}

/// M2-3: the memory effect applies and bumps the version.
#[test]
fn test_effect_memory_apply() {
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    let effect = EffectMemory::one(NodeId(3), Level(1), 7);
    effect.apply(&mut memory);
    assert_eq!(memory.version(), 1);
    assert_eq!(memory.read(NodeId(3), Level(1)), Some(&7));

    let effect_empty: EffectMemory<u64> = EffectMemory::new();
    effect_empty.apply(&mut memory);
    assert_eq!(memory.version(), 1, "empty effect: no bump");
}

/// Transduction that reads then writes in the level memory at each visit
/// of a node: `M[id(n), ℓ] ← h` (the inherited attribute = depth). The value
/// read (if present) is accumulated in a shared counter to prove that
/// φ_out does observe the state stored by a previous visit.
struct WrittenMemory {
    read: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Transduction for WrittenMemory {
    type H = u64;
    type S = u64;
    type Out = bool;
    type M = u64;

    fn fingerprint(&self) -> u64 {
        3001
    }
    fn h0(&self) -> u64 {
        0
    }
    fn phi_dispatch(&self, _level: Level, h: &u64, _edge: u64) -> u64 {
        h + 1
    }
    fn phi_aggregate(
        &self,
        _level: Level,
        _h: &u64,
        edges: &BTreeMap<u64, (Option<Level>, u64)>,
    ) -> u64 {
        edges.values().map(|(_, s)| *s).max().unwrap_or(0)
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<u64>) {
        // Reads the memory: if a value was already stored at this (node, level),
        // it is visible here and counted (test: φ_out observes the state
        // stored by a previous visit of the same node).
        if let Some(v) = ctx.memory.read(ctx.node_input, ctx.level) {
            self.read.fetch_add(*v, std::sync::atomic::Ordering::SeqCst);
        }
        // Writes h into the memory at (node_input, level).
        let effect = EffectMemory::one(ctx.node_input, ctx.level, *ctx.h);
        let edges = ctx.edges.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    edges,
                },
            ),
            effect,
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), 0)
    }
}

/// M2-3: a memory transduction does write into the level memory
/// via the effect produced by φ_out, and the version is bumped (L1).
#[test]
fn test_transduction_with_memory_written() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let true_node = input.intern(anchor, Form::Terminal(true));
    let false_node = input.intern(anchor, Form::Terminal(false));
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    let b = input.intern(
        anchor,
        Form::Node {
            level: Level(1),
            edges: BTreeMap::from([(0, a), (1, true_node)]),
        },
    );

    let read = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut runner =
        Transducer::with_policy(WrittenMemory { read: read.clone() }, PolicyMemory::ByLevel);
    assert_eq!(runner.version_memory(), 0, "empty memory at start");
    let (result, _) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(
        output.resolve(result).unwrap(),
        input.resolve(b).unwrap(),
        "identity copy"
    );
    // Two not-terminal nodes (a and b) each wrote into the memory.
    assert!(
        runner.version_memory() >= 2,
        "version bumped by the memory effects (L1), version = {}",
        runner.version_memory()
    );
    // BLOCKING review: the memory read must be observed. We run a
    // second time: each node re-reads the value stored on the first pass.
    let (r2, _) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(
        r2, result,
        "determinism: same result (the bumped version invalidates the memo, the node is recomputed)"
    );
    let total_read = read.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        total_read > 0,
        "φ_out observed the state stored by a previous visit (T3), total read = {total_read}"
    );
}

/// M2-3: the `No` policy rejects memory effects (TO-FIX review).
#[test]
fn test_policy_no_rejected_effect_memory() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let true_node = input.intern(anchor, Form::Terminal(true));
    let false_node = input.intern(anchor, Form::Terminal(false));
    // Two edges to distinct targets: no R-collapse.
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    let read = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    // Policy No (default): the memory effect must be rejected.
    let mut runner = Transducer::new(WrittenMemory { read: read.clone() });
    let result = runner.run(&input, &mut output, anchor, a);
    assert!(
        matches!(
            result,
            Err(lisage_core::error::CoreError::MemoryNotAutorisee)
        ),
        "policy No ⇒ memory effect rejected, got {result:?}"
    );
}

/// M2-3: the storage policy is declared by the controller
/// (`No` by default in M1, `ByLevel` to allow the memory).
#[test]
fn test_policy_memory_of_controller() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let controller = ReferenceController::new(schema);
    assert_eq!(
        controller.policy_memory(),
        PolicyMemory::No,
        "default policy (M1): no level memory"
    );
}
