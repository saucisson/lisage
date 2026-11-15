//! Level-memory tests (M2-3, spec § 3.1, statement T3): storage between two
//! visits of the same level, versioning (L1), memory effect produced by
//! φ_out, and policy declared by the controller.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Controller, Form, Level, MemoryPolicy, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::memory::{EffetMemory, MemoryLevel};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{ContextePhiOut, Transducteur, Transduction};
use lisage_lang::value::Atom;

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// M2-3: the level memory stores and versions (L1).
#[test]
fn test_memory_level_stockage_et_version() {
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
fn test_effet_memory_appliquer() {
    let mut memory: MemoryLevel<u64> = MemoryLevel::new();
    let effet = EffetMemory::une(NodeId(3), Level(1), 7);
    effet.apply(&mut memory);
    assert_eq!(memory.version(), 1);
    assert_eq!(memory.read(NodeId(3), Level(1)), Some(&7));

    let effet_empty: EffetMemory<u64> = EffetMemory::new();
    effet_empty.apply(&mut memory);
    assert_eq!(memory.version(), 1, "empty effect: no bump");
}

/// Transduction that reads then writes in the level memory at each node
/// visit: `M[id(n), ℓ] ← h` (the inherited attribute = depth). The read
/// value (if present) is accumulated in a shared counter to prove that φ_out
/// does observe the state stored by a previous visit.
struct EcritMemory {
    lus: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Transduction for EcritMemory {
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
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffetMemory<u64>) {
        // Read the memory: if a value was already stored at this (node, level),
        // it is visible here and counted (test: φ_out observes the state stored
        // by a previous visit of the same node).
        if let Some(v) = ctx.memory.read(ctx.node_input, ctx.level) {
            self.lus.fetch_add(*v, std::sync::atomic::Ordering::SeqCst);
        }
        // Write h into the memory at (input_node, level).
        let effet = EffetMemory::une(ctx.node_input, ctx.level, *ctx.h);
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            effet,
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
fn test_transduction_with_memory_ecrit() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let vrai = input.intern(anchor, Form::Terminal(true));
    let faux = input.intern(anchor, Form::Terminal(false));
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let b = input.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, a), (1, vrai)]),
        },
    );

    let lus = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut runner =
        Transducteur::with_policy(EcritMemory { lus: lus.clone() }, MemoryPolicy::PerLevel);
    assert_eq!(runner.version_memory(), 0, "empty memory at start");
    let (result, _) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(
        output.resolve(result).unwrap(),
        input.resolve(b).unwrap(),
        "identity copy"
    );
    // Two non-terminal nodes (a and b) each wrote into the memory.
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
fn test_policy_aucune_rejette_effet_memory() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let vrai = input.intern(anchor, Form::Terminal(true));
    let faux = input.intern(anchor, Form::Terminal(false));
    // Two arcs to distinct targets: no R-collapse.
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let lus = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    // NoMemory policy (default): the memory effect must be rejected.
    let mut runner = Transducteur::new(EcritMemory { lus: lus.clone() });
    let result = runner.run(&input, &mut output, anchor, a);
    assert!(
        matches!(
            result,
            Err(lisage_core::error::CoreError::MemoryNonAutorisee)
        ),
        "NoMemory policy ⇒ memory effect rejected, got {result:?}"
    );
}

/// M2-3: the storage policy is declared by the controller (`NoMemory` by
/// default in M1, `PerLevel` to allow the memory).
#[test]
fn test_policy_memory_du_controleur() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let controleur = ReferenceController::new(schema);
    assert_eq!(
        controleur.memory_policy(),
        MemoryPolicy::NoMemory,
        "default policy (M1): no level memory"
    );
}
