//! PBT of the transduction engine (M1-4, L3): identity (same forms in
//! output), counting of accepting paths (transduction vs naive recursive
//! count), and memo sanity (op, node).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{ContextPhiOut, Transducer, Transduction};
use lisage_lang::value::Atom;
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn new_pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

struct Identity;

impl Transduction for Identity {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        2001
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

struct CountPaths;

impl Transduction for CountPaths {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        2002
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

/// Structural equality of two forms in two distinct pools (NodeIds are
/// specific to each pool).
fn forms_equal(
    pool_a: &PoolRef,
    node_a: NodeId,
    pool_b: &PoolRef,
    node_b: NodeId,
    visits: &mut std::collections::HashMap<(NodeId, NodeId), bool>,
) -> bool {
    if let Some(&seen) = visits.get(&(node_a, node_b)) {
        return seen;
    }
    let equal = match (
        pool_a.resolve(node_a).unwrap(),
        pool_b.resolve(node_b).unwrap(),
    ) {
        (Form::Terminal(pa), Form::Terminal(pb)) => pa == pb,
        (
            Form::Node {
                level: la,
                edges: aa,
            },
            Form::Node {
                level: lb,
                edges: ab,
            },
        ) => {
            la == lb
                && aa.len() == ab.len()
                && aa.iter().zip(ab.iter()).all(|((va, na), (vb, count))| {
                    va == vb && forms_equal(pool_a, *na, pool_b, *count, visits)
                })
        }
        _ => false,
    };
    visits.insert((node_a, node_b), equal);
    equal
}

/// Naive recursive counting of accepting paths (independent oracle).
fn count_naive(pool: &PoolRef, node: NodeId) -> u64 {
    match pool.resolve(node).unwrap() {
        Form::Terminal(payload) => u64::from(*payload),
        Form::Node { edges, .. } => edges.values().map(|&c| count_naive(pool, c)).sum(),
    }
}

/// Builds a random DAG in the input pool: terminals then nodes of levels
/// 0..3, edges toward nodes of strictly lower index.
fn dag_random(pool: &mut PoolRef, anchor: Anchor, pairs: Vec<(u8, u32)>) -> NodeId {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let false_node = pool.intern(anchor, Form::Terminal(false));
    let mut nodes = vec![true_node, false_node];
    for (level, target) in pairs {
        let target = nodes[usize::try_from(target).unwrap() % nodes.len()];
        let node = pool.intern(
            anchor,
            Form::Node {
                level: Level(u32::from(level % 4)),
                edges: BTreeMap::from([(u64::from(level % 2), target)]),
            },
        );
        nodes.push(node);
    }
    *nodes.last().unwrap()
}

proptest! {
    /// Identity: the transduction rebuilds the same forms in output.
    #[test]
    fn test_identity_pbt(pairs in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut input = new_pool();
        let mut output = new_pool();
        let anchor = Anchor(0);
        let root = dag_random(&mut input, anchor, pairs);
        let mut runner = Transducer::new(Identity);
        let (result, _) = runner.run(&input, &mut output, anchor, root).unwrap();
        prop_assert!(forms_equal(
            &input,
            root,
            &output,
            result,
            &mut std::collections::HashMap::new()
        ));
    }

    /// Counting: the transduction (φ_aggr = sum) equals the naive count.
    #[test]
    fn test_counting_pbt(pairs in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut input = new_pool();
        let mut output = new_pool();
        let anchor = Anchor(0);
        let root = dag_random(&mut input, anchor, pairs);
        let expected = count_naive(&input, root);
        let mut runner = Transducer::new(CountPaths);
        let (result, count) = runner.run(&input, &mut output, anchor, root).unwrap();
        prop_assert!(forms_equal(
            &input,
            root,
            &output,
            result,
            &mut std::collections::HashMap::new()
        ));
        prop_assert_eq!(count, expected);
        // Healthy memo: a second run changes nothing.
        let size = runner.size_memo();
        let (r2, c2) = runner.run(&input, &mut output, anchor, root).unwrap();
        prop_assert_eq!(r2, result);
        prop_assert_eq!(c2, count);
        prop_assert_eq!(runner.size_memo(), size);
        prop_assert!(size <= input.node_count());
    }
}
