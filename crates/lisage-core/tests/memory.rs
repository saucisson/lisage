//! Level-memory tests (M2-3, spec § 3.1, statement T3): storage between two
//! visits of the same level, versioning (L1), memory effect produced by
//! φ_out, and policy declared by the controller.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, Form, Level, MemoryPolicy, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::memory::{EffectMemory, MemoryLevel};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{PhiOutContext, Transducer, Transduction};
use lisage_lang::value::Atom;

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// M2-3: the level memory stores and versions (L1).
#[test]
fn test_memory_level_storage_and_version() {
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    assert_eq!(memory.version(), 0);
    assert_eq!(memory.read(NodeId(0), Level(0)), None);

    let old = memory.write(NodeId(0), Level(0), 42);
    assert_eq!(old, None);
    assert_eq!(memory.version(), 1, "each write increments the version");
    assert_eq!(memory.read(NodeId(0), Level(0)), Some(&42));

    let old = memory.write(NodeId(0), Level(0), 43);
    assert_eq!(old, Some(42), "the write returns the old value");
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

/// Transduction that reads then writes in the level memory at each node
/// visit: `M[id(n), ℓ] ← h` (the inherited attribute = depth). The read
/// value (if present) is accumulated in a shared counter to prove that φ_out
/// does observe the state stored by a previous visit.
struct WritesMemory {
    lus: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Transduction for WritesMemory {
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
    fn phi_disp(&self, _level: Level, h: &u64, _arc: u64) -> u64 {
        h + 1
    }
    fn phi_aggr(&self, _level: Level, _h: &u64, arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).max().unwrap_or(0)
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<u64>) {
        // Read the memory: if a value was already stored at this (node, level),
        // it is visible here and counted (test: φ_out observes the state stored
        // by a previous visit of the same node).
        if let Some(v) = ctx.memory.read(ctx.node_input, ctx.level) {
            self.lus.fetch_add(*v, std::sync::atomic::Ordering::SeqCst);
        }
        // Write h into the memory at (input_node, level).
        let effect = EffectMemory::one(ctx.node_input, ctx.level, *ctx.h);
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
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

/// M2-3: a memory transduction really writes into the level memory via the
/// effect produced by φ_out, and the version is bumped (L1).
#[test]
fn test_transduction_with_memory_writes() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let true_node = input.intern(anchor, Form::Terminal(true));
    let false_node = input.intern(anchor, Form::Terminal(false));
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: ArcRun::from_pairs([(0, true_node), (1, false_node)]),
        },
    );
    let b = input.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: ArcRun::from_pairs([(0, a), (1, true_node)]),
        },
    );

    let lus = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut runner =
        Transducer::with_policy(WritesMemory { lus: lus.clone() }, MemoryPolicy::PerLevel);
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
    // BLOCKING review: the memory read must be observed. Run a second time:
    // each node re-reads the value stored on the first pass.
    let (r2, _) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(
        r2, result,
        "determinism: same result (the bumped version invalidates the memo, the node is recomputed)"
    );
    let total_lus = lus.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        total_lus > 0,
        "φ_out observed the state stored by a previous visit (T3), total read = {total_lus}"
    );
}

/// M2-3: the `NoMemory` policy rejects memory effects (TO-FIX review).
#[test]
fn test_policy_none_rejects_effect_memory() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let true_node = input.intern(anchor, Form::Terminal(true));
    let false_node = input.intern(anchor, Form::Terminal(false));
    // Two arcs to distinct targets: no R-collapse.
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: ArcRun::from_pairs([(0, true_node), (1, false_node)]),
        },
    );
    let lus = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    // NoMemory policy (default): the memory effect must be rejected.
    let mut runner = Transducer::new(WritesMemory { lus: lus.clone() });
    let result = runner.run(&input, &mut output, anchor, a);
    assert!(
        matches!(result, Err(lisage_core::error::CoreError::MemoryDisallowed)),
        "NoMemory policy ⇒ memory effect rejected, got {result:?}"
    );
}

/// M2-3: the storage policy is declared by the controller (`NoMemory` by
/// default in M1, `PerLevel` to allow the memory).
#[test]
fn test_controller_memory_policy() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let controller = ReferenceController::new(schema);
    assert_eq!(
        controller.memory_policy(),
        MemoryPolicy::NoMemory,
        "default policy (M1): no level memory"
    );
}

/// MEM-15: dense parallel arrays per class — the footprint per entry is
/// divided by at least 2 against the old `HashMap<(NodeId, Level), V>`
/// (~48 B per entry).
#[test]
fn test_fingerprint_memo_dense_divisee() {
    const BASELINE_HASHMAP_BYTES_PER_ENTRY: usize = 48;
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    for node in 0..500u32 {
        memory.write(NodeId(node), Level(node % 7), node as u64 * 3);
    }
    assert_eq!(memory.size(), 500);
    let per_entry = memory.bytes() as f64 / memory.size() as f64;
    assert!(
        per_entry <= (BASELINE_HASHMAP_BYTES_PER_ENTRY / 2) as f64,
        "dense arrays: {per_entry:.1} B/entry, must be ≤ {} B/entry (half of the ~48 B HashMap)",
        BASELINE_HASHMAP_BYTES_PER_ENTRY / 2
    );
    // Read-after-write still exact.
    for node in 0..500u32 {
        assert_eq!(
            memory.read(NodeId(node), Level(node % 7)),
            Some(&(node as u64 * 3))
        );
    }
}

/// MEM-15: several levels for the same node keep the exact `(node, level)`
/// semantics (dense slot + overflow list).
#[test]
fn test_several_levels_per_node() {
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    assert_eq!(memory.write(NodeId(5), Level(0), 10), None);
    assert_eq!(
        memory.write(NodeId(5), Level(1), 11),
        None,
        "second level: new key"
    );
    assert_eq!(
        memory.write(NodeId(5), Level(0), 12),
        Some(10),
        "overwrite same key"
    );
    assert_eq!(memory.size(), 2);
    assert_eq!(memory.read(NodeId(5), Level(0)), Some(&12));
    assert_eq!(memory.read(NodeId(5), Level(1)), Some(&11));
    assert_eq!(memory.read(NodeId(5), Level(2)), None);
    assert_eq!(memory.version(), 3);
}

/// MEM-15: the memory holders apply the compaction remap — stored node ids
/// are translated, dead nodes drop their entry, the version bumps (L1).
#[test]
fn test_remap_applies_per_the_carriers() {
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    memory.write(NodeId(0), Level(0), 100);
    memory.write(NodeId(1), Level(0), 200);
    memory.write(NodeId(2), Level(1), 300);
    let version_before = memory.version();
    // Compaction remap: 0 → 10, 1 → dead, 2 → 20.
    let remap: BTreeMap<u32, Option<NodeId>> =
        BTreeMap::from([(0, Some(NodeId(10))), (1, None), (2, Some(NodeId(20)))]);
    memory.remap(&remap);
    assert_eq!(memory.size(), 2, "the dead entry is dropped");
    assert_eq!(memory.read(NodeId(10), Level(0)), Some(&100));
    assert_eq!(memory.read(NodeId(20), Level(1)), Some(&300));
    assert_eq!(memory.read(NodeId(0), Level(0)), None);
    assert_eq!(memory.read(NodeId(1), Level(0)), None);
    assert!(memory.version() > version_before, "L1: the memory changed");
}

/// MEM-15: with a pool class width `b`, the memory decodes `(class, index)`
/// — entries land in the dense array of their class.
#[test]
fn test_decode_with_pool_class_bits() {
    use lisage_algebra::class_bits::TAG_BITS;
    // b = 2: class in bits [26..28), index in the low 26 bits.
    let mut memory: MemoryLevel<u64> = MemoryLevel::with_class_bits(2);
    let class_2_index_5 = NodeId::encode(2, 5, 2);
    let class_3_index_1 = NodeId::encode(3, 1, 2);
    assert_eq!(class_2_index_5.0 >> (32 - TAG_BITS - 2), 2);
    memory.write(class_2_index_5, Level(0), 42);
    memory.write(class_3_index_1, Level(1), 43);
    assert_eq!(memory.read(class_2_index_5, Level(0)), Some(&42));
    assert_eq!(memory.read(class_3_index_1, Level(1)), Some(&43));
    assert_eq!(
        memory.read(class_2_index_5, Level(1)),
        None,
        "keyed by (node, level)"
    );
    assert_eq!(memory.size(), 2);
}
