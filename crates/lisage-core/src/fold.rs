//! Fold and semirings (M1-6, spec § 5, statements A4/A5/C7).
//!
//! **Two declared algebras (A5)**: the **construction** semiring
//! (the merge of writes) and the **query** semiring (parameter of the
//! transduction) are two distinct declarations. The construction is
//! idempotent `Bool` — **hash-consing is this choice**: [`union`] is
//! the idempotent union, it destroys multiplicity at write time. The
//! query semiring is the parameter `S` of [`Fold`]: a fold is
//! exact iff its reading factorizes through what the construction preserved;
//! on a Bool construction, only the cardinality is exact (spec § 5.1).
//!
//! **Fold** [`Fold<S>`]: `H` trivial, the terminal produces the base
//! (`weight(⊤) = 1`, `weight(⊥) = 0`), `φ_aggr` the structure of the semiring.
//! The counting expansion **(A4)**: between a node of level `ℓ` and a child
//! of level `ℓ'`, the levels `ℓ+1..ℓ'` have been collapsed (all their
//! values lead to the same continuation) ⇒ the weight of the edge is
//! multiplied by `∏ |Val(n)|`. The implicit default edge (R-default)
//! contributes the weight of `D_δ` — on the reference instance `D_δ =
//! terminal(false)`, of zero weight (A6: a lifted construction would pay
//! its weight here).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::schema::{Schema, SchemaKeys};
use crate::transduction::{ContextPhiOut, Transducer, Transduction};

/// Query semiring of a fold (spec § 5): `(S, ⊕, ×)` with zeros and
/// units. `since_cardinality` translates `|Val(ℓ)|` into `S` (A4).
pub trait SemiRing: Clone + PartialEq + Eq + Hash + Debug + Send + Sync {
    /// Fingerprint of the semiring: enters the memo fingerprint.
    const FINGERPRINT: u64;

    /// Neutral element of ⊕ (zero).
    fn zero() -> Self;
    /// Neutral element of × (one).
    fn one() -> Self;
    /// Cardinality of a level, translated into the semiring (A4).
    fn since_cardinality(cardinality: u64) -> Self;
    /// ⊕ (aggregation of the children).
    fn add(&self, other: &Self) -> Self;
    /// × (weight of the edges by the subtotals).
    fn times(&self, other: &Self) -> Self;
}

/// `(ℕ, +, ×)` : the semiring of cardinal counting (spec § 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Naturel(pub u64);

impl SemiRing for Naturel {
    const FINGERPRINT: u64 = 4001;

    fn zero() -> Self {
        Naturel(0)
    }
    fn one() -> Self {
        Naturel(1)
    }
    fn since_cardinality(cardinality: u64) -> Self {
        Naturel(cardinality)
    }
    fn add(&self, other: &Self) -> Self {
        Naturel(self.0 + other.0)
    }
    fn times(&self, other: &Self) -> Self {
        Naturel(self.0 * other.0)
    }
}

/// `(Bool, ∨, ∧)` : the semiring of ∃-summaries (`exists`, spec § 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BoolOr(pub bool);

impl SemiRing for BoolOr {
    const FINGERPRINT: u64 = 4002;

    fn zero() -> Self {
        BoolOr(false)
    }
    fn one() -> Self {
        BoolOr(true)
    }
    fn since_cardinality(cardinality: u64) -> Self {
        BoolOr(cardinality > 0)
    }
    fn add(&self, other: &Self) -> Self {
        BoolOr(self.0 || other.0)
    }
    fn times(&self, other: &Self) -> Self {
        BoolOr(self.0 && other.0)
    }
}

/// Fold over a query semiring `S` (spec § 5): `H` trivial,
/// the output diagram is the identity copy of the input, and the
/// synthesized attribute is the value of the fold. `Fold<Naturel>` = `count`.
pub struct Fold<S: SemiRing> {
    keys: SchemaKeys,
    number_levels: u32,
    _semi_anneau: PhantomData<S>,
}

impl<S: SemiRing> Fold<S> {
    /// Fold over the given schema (the level cardinalities come from
    /// the key algebra of the schema, A4).
    pub fn new(schema: Schema) -> Self {
        let number_levels = schema.number_levels();
        Fold {
            keys: SchemaKeys::new(schema),
            number_levels,
            _semi_anneau: PhantomData,
        }
    }

    /// Weight of a terminal: `weight(⊤) = 1`, `weight(⊥) = 0` (spec § 5).
    fn weight(&self, payload: bool) -> S {
        if payload { S::one() } else { S::zero() }
    }

    /// (A4) multiplier of the edge `v` of level `ℓ` to a child of
    /// level `level_target` (`None` = terminal): product of the cardinalities
    /// of the levels skipped **on the path** of the edge. A skipped value
    /// level was collapsed with all its explicit values ⇒ factor
    /// `|Val(n)|`; a skipped presence level only collapses through its
    /// default edge (factor 1) and its value level `n+1` is off the path.
    fn multiplicateur(&self, level: Level, edge: u64, level_target: Option<Level>) -> S {
        let bound = match level_target {
            Some(level) => level.0,
            None => self.number_levels,
        };
        // Next level on the edge's path: a value level is
        // always followed; a presence followed by the default edge skips
        // the value level.
        let mut n = match self.keys.default(level) {
            Some(default) if edge == default => level.0 + 2,
            _ => level.0 + 1,
        };
        let mut product = S::one();
        while n < bound {
            if self.keys.default(Level(n)).is_some() {
                // Skipped presence level: default collapse only.
                n += 2;
            } else {
                product = product.times(&S::since_cardinality(self.keys.cardinality(Level(n))));
                n += 1;
            }
        }
        product
    }
}

impl<S: SemiRing> Transduction for Fold<S> {
    type H = ();
    type S = S;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        S::FINGERPRINT
    }

    fn h0(&self) {}

    fn phi_dispatch(&self, _level: Level, h: &(), _edge: u64) {
        let _ = h;
    }

    fn phi_aggregate(&self, level: Level, _h: &(), edges: &BTreeMap<u64, (Option<Level>, S)>) -> S {
        let mut total = S::zero();
        for (edge, (level_target, s)) in edges {
            let multiplicateur = self.multiplicateur(level, *edge, *level_target);
            total = total.add(&multiplicateur.times(s));
        }
        // Implicit default edge (R-default): its continuation is D_δ,
        // of weight `weight(false)` on the reference instance (A6).
        if let Some(default) = self.keys.default(level)
            && !edges.contains_key(&default)
        {
            total = total.add(&self.weight(false));
        }
        total
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
        // Identity copy: the fold does not transform the diagram.
        let edges = ctx.edges.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    edges,
                },
            ),
            crate::memory::EffectMemory::new(),
        )
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, S) {
        let terminal = pool.intern(anchor, Form::Terminal(payload));
        let weight = self.weight(payload);
        (terminal, weight)
    }
}

/// Counting cardinality of a diagram (denoted set): fold over `(ℕ, +, ×)`,
/// exact on Bool construction (A5). Raises the result of the synthesized channel.
pub fn count<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    node: NodeId,
) -> Result<u64, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducer::new(Fold::<Naturel>::new(schema.clone()));
    let (_, total) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(total.0)
}

/// Union of two diagrams with `bool` payload (construction merge, A5):
/// the idempotent union — **hash-consing is this Bool choice**: the
/// multiplicity is destroyed at write time, only the cardinality is exact.
/// ZDD style: the terminal `⊤` is the member "all levels by
/// default" (carried by the edge `δ(ℓ)`), and a node of level greater than
/// `ℓ` is the continuation of the default edge of level `ℓ`.
pub fn union<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    a: NodeId,
    b: NodeId,
) -> Result<NodeId, CoreError> {
    if a == b {
        return Ok(a);
    }
    let (form_a, form_b) = (pool.resolve(a)?.clone(), pool.resolve(b)?.clone());
    match (form_a, form_b) {
        (Form::Terminal(pa), Form::Terminal(pb)) => {
            Ok(pool.intern(anchor, Form::Terminal(pa || pb)))
        }
        (Form::Node { level, edges }, Form::Terminal(pb)) => {
            if pb {
                let true_node = pool.intern(anchor, Form::Terminal(true));
                union_with_other(pool, keys, anchor, level, edges, true_node)
            } else {
                Ok(a)
            }
        }
        (Form::Terminal(pa), Form::Node { level, edges }) => {
            if pa {
                let true_node = pool.intern(anchor, Form::Terminal(true));
                union_with_other(pool, keys, anchor, level, edges, true_node)
            } else {
                Ok(b)
            }
        }
        (
            Form::Node {
                level: level_a,
                edges: edges_a,
            },
            Form::Node {
                level: level_b,
                edges: edges_b,
            },
        ) => {
            if level_a == level_b {
                let mut edges = edges_a;
                for (edge, target_b) in edges_b {
                    match edges.remove(&edge) {
                        Some(target_a) => {
                            let uni = union(pool, keys, anchor, target_a, target_b)?;
                            edges.insert(edge, uni);
                        }
                        None => {
                            edges.insert(edge, target_b);
                        }
                    }
                }
                Ok(pool.intern(
                    anchor,
                    Form::Node {
                        level: level_a,
                        edges,
                    },
                ))
            } else if level_a < level_b {
                // Node b is the continuation of the default edge of a.
                union_with_other(pool, keys, anchor, level_a, edges_a, b)
            } else {
                union_with_other(pool, keys, anchor, level_b, edges_b, a)
            }
        }
    }
}

/// Union of a node of level `level` with another diagram `other`
/// (terminal `⊤`, or deeper node): the path of `other` goes through
/// the default edge `δ(level)` if the level has one (the absent edges
/// are δ); otherwise (value level) it goes through **all** the values
/// of the level, explicit or not.
fn union_with_other<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    level: Level,
    mut edges: BTreeMap<u64, NodeId>,
    other: NodeId,
) -> Result<NodeId, CoreError> {
    match keys.default(level) {
        Some(default) => {
            let target = match edges.remove(&default) {
                Some(target) => union(pool, keys, anchor, target, other)?,
                None => other,
            };
            edges.insert(default, target);
        }
        None => {
            let cardinality = keys.cardinality(level);
            for v in 0..cardinality {
                let target = match edges.remove(&v) {
                    Some(target) => union(pool, keys, anchor, target, other)?,
                    None => other,
                };
                edges.insert(v, target);
            }
        }
    }
    Ok(pool.intern(anchor, Form::Node { level, edges }))
}
