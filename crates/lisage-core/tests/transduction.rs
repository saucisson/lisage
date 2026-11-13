//! Transduction engine tests (M1-4, spec § 3.1, L3): the two
//! channels (inherited/synthesized), the φ_aggr aggregation and the memo (op, node, h).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiled};
use lisage_core::transduction::{ContextPhiOut, Transducer, Transduction};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Identity transduction with inherited channel = depth: H = u64.
struct IdentityDepth;

impl Transduction for IdentityDepth {
    type H = u64;
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        1001
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
    ) -> (NodeId, lisage_core::memory::EffectMemory<Self::M>) {
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
        h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), *h)
    }
}

/// Counting transduction of accepting paths (synthesized channel only).
struct CountPaths {
    calls: std::sync::Arc<AtomicU64>,
}

impl Transduction for CountPaths {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        1002
    }
    fn h0(&self) {}
    fn phi_dispatch(&self, _level: Level, h: &(), _edge: u64) {
        let _ = h;
    }
    fn phi_aggregate(
        &self,
        _level: Level,
        _h: &(),
        edges: &BTreeMap<u64, (Option<Level>, u64)>,
    ) -> u64 {
        edges.values().map(|(_, s)| *s).sum()
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<Self::M>) {
        self.calls.fetch_add(1, Ordering::SeqCst);
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
    ) -> (NodeId, u64) {
        (
            pool.intern(anchor, Form::Terminal(payload)),
            u64::from(payload),
        )
    }
}

/// Identity without inherited channel, to compare memo fingerprints.
struct Identity;

impl Transduction for Identity {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        1003
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
    ) -> (NodeId, lisage_core::memory::EffectMemory<Self::M>) {
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

/// Builds a small DAG in the input pool: b = ν(1, {0 ↦ a, 1 ↦ ⊤}),
/// a = ν(0, {0 ↦ ⊤, 1 ↦ ⊥}).
fn dag(pool: &mut PoolRef, anchor: Anchor) -> (NodeId, NodeId) {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let false_node = pool.intern(anchor, Form::Terminal(false));
    let a = pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, true_node), (1, false_node)]),
        },
    );
    let b = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            edges: BTreeMap::from([(0, a), (1, true_node)]),
        },
    );
    (a, b)
}

/// Inherited channel: the identity transduction rebuilds the same forms;
/// the bottom-up depth equals the height of the DAG.
#[test]
fn test_channel_inherited_identity_and_depth() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let (_, b) = dag(&mut input, anchor);
    let mut runner = Transducer::new(IdentityDepth);
    let (result, depth) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(
        output.resolve(result).unwrap(),
        input.resolve(b).unwrap(),
        "identity ⇒ same forms"
    );
    assert_eq!(depth, 2, "max depth reached = height of the DAG");
}

/// Synthesized channel: φ_aggr sums the weights of the children; the count is the
/// number of accepting paths (2 in the DAG: 0·0 and 1).
#[test]
fn test_channel_synthetise_counting() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let (_, b) = dag(&mut input, anchor);
    let calls = std::sync::Arc::new(AtomicU64::new(0));
    let count = CountPaths {
        calls: calls.clone(),
    };
    let mut runner = Transducer::new(count);
    let (result, count_paths) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(output.resolve(result).unwrap(), input.resolve(b).unwrap());
    assert_eq!(count_paths, 2, "two paths lead to ⊤");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "φ_out called once per not-terminal node"
    );
}

/// Memo (op, node, h): a second run recomputes nothing (memo hit),
/// and two operations with distinct fingerprints have separate entries.
#[test]
fn test_memo_fingerprint_op_node() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let (_, b) = dag(&mut input, anchor);
    let mut runner = Transducer::new(Identity);
    let (r1, _) = runner.run(&input, &mut output, anchor, b).unwrap();
    let size_after_1 = runner.size_memo();
    let (r2, _) = runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(r1, r2);
    assert_eq!(
        runner.size_memo(),
        size_after_1,
        "second run: only memo hits"
    );
    // 4 distinct (node, h) pairs: b, a, ⊤, ⊥.
    assert_eq!(size_after_1, 4);

    // Another operation (different fingerprint) on the same node: separate
    // memo. With H = depth, ⊤ is visitd at two distinct h (1 via
    // edge 1 of b, 2 via edge 0 of a): 5 entries (b, a, ⊤×2, ⊥).
    let mut other = Transducer::new(IdentityDepth);
    let (r3, _) = other.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(output.resolve(r3).unwrap(), input.resolve(b).unwrap());
    assert_eq!(
        other.size_memo(),
        5,
        "memo specific to the operation and to h: key (op, node, h)"
    );
}

/// The transduction also applies to a diagram compiled from a value
/// (schema M1-3): identity on a compiled set.
#[test]
fn test_transduction_on_diagram_compiled() {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let anchor = Anchor(0);
    let set = Value::Set(std::collections::BTreeSet::from([
        Value::Atom(Atom(0)),
        Value::Atom(Atom(1)),
    ]));
    let root = compiled(&mut input, anchor, &schema, &set).unwrap();
    let mut runner = Transducer::new(Identity);
    let (result, _) = runner.run(&input, &mut output, anchor, root).unwrap();
    assert_eq!(
        output.resolve(result).unwrap(),
        input.resolve(root).unwrap()
    );
}
