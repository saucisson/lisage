//! PBT of the transduction engine (M1-4, L3): identity (same forms on
//! output), counting of accepting paths (transduction vs naive
//! recursive count), and memo sanity (op, node).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{ContextePhiOut, Transducteur, Transduction};
use lisage_lang::value::Atom;
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn new_pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

struct Identite;

impl Transduction for Identite {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        2001
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<Self::M>) {
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
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).sum()
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<Self::M>) {
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

/// Structural equality of two forms in two distinct pools (the NodeIds
/// are specific to each pool).
fn forms_egales(
    pool_a: &PoolRef,
    node_a: NodeId,
    pool_b: &PoolRef,
    node_b: NodeId,
    visites: &mut std::collections::HashMap<(NodeId, NodeId), bool>,
) -> bool {
    if let Some(&vu) = visites.get(&(node_a, node_b)) {
        return vu;
    }
    let egales = match (
        pool_a.resolve(node_a).unwrap(),
        pool_b.resolve(node_b).unwrap(),
    ) {
        (Form::Terminal(pa), Form::Terminal(pb)) => pa == pb,
        (
            Form::Node {
                level: level_a,
                arcs: aa,
            },
            Form::Node {
                level: level_b,
                arcs: ab,
            },
        ) => {
            level_a == level_b
                && aa.len() == ab.len()
                && aa.iter().zip(ab.iter()).all(|((va, na), (vb, nb))| {
                    va == vb && forms_egales(pool_a, *na, pool_b, *nb, visites)
                })
        }
        _ => false,
    };
    visites.insert((node_a, node_b), egales);
    egales
}

/// Naive recursive count of accepting paths (independent oracle).
fn count_naif(pool: &PoolRef, node: NodeId) -> u64 {
    match pool.resolve(node).unwrap() {
        Form::Terminal(payload) => u64::from(*payload),
        Form::Node { arcs, .. } => arcs.values().map(|&c| count_naif(pool, c)).sum(),
    }
}

/// Builds a random DAG in the input pool: terminals then nodes
/// of levels 0..3, arcs towards nodes of strictly lower index.
fn dag_aleatoire(pool: &mut PoolRef, anchor: Anchor, paires: Vec<(u8, u32)>) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let faux = pool.intern(anchor, Form::Terminal(false));
    let mut nodes = vec![vrai, faux];
    for (level, target) in paires {
        let target = nodes[usize::try_from(target).unwrap() % nodes.len()];
        let node = pool.intern(
            anchor,
            Form::Node {
                level: Level(u32::from(level % 4)),
                arcs: ArcRun::from_pairs([(u64::from(level % 2), target)]),
            },
        );
        nodes.push(node);
    }
    *nodes.last().unwrap()
}

proptest! {
    /// Identity: the transduction rebuilds the same forms on output.
    #[test]
    fn test_identite_pbt(paires in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut input = new_pool();
        let mut output = new_pool();
        let anchor = Anchor(0);
        let root = dag_aleatoire(&mut input, anchor, paires);
        let mut runner = Transducteur::new(Identite);
        let (result, _) = runner.run(&input, &mut output, anchor, root).unwrap();
        prop_assert!(forms_egales(
            &input,
            root,
            &output,
            result,
            &mut std::collections::HashMap::new()
        ));
    }

    /// Counting: the transduction (φ_aggr = sum) equals the naive count.
    #[test]
    fn test_comptage_pbt(paires in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut input = new_pool();
        let mut output = new_pool();
        let anchor = Anchor(0);
        let root = dag_aleatoire(&mut input, anchor, paires);
        let expected = count_naif(&input, root);
        let mut runner = Transducteur::new(CountPaths);
        let (result, count) = runner.run(&input, &mut output, anchor, root).unwrap();
        prop_assert!(forms_egales(
            &input,
            root,
            &output,
            result,
            &mut std::collections::HashMap::new()
        ));
        prop_assert_eq!(count, expected);
        // Sound memo: a second run changes nothing.
        let size = runner.size_memo();
        let (r2, c2) = runner.run(&input, &mut output, anchor, root).unwrap();
        prop_assert_eq!(r2, result);
        prop_assert_eq!(c2, count);
        prop_assert_eq!(runner.size_memo(), size);
        prop_assert!(size <= input.node_count());
    }
}
