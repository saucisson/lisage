//! Fold and semirings (M1-6, spec § 5, statements A4/A5/C7).
//!
//! **Two declared algebras (A5)**: the **construction** semiring (the merge
//! of writes) and the **query** semiring (parameter of the transduction) are
//! two distinct declarations. The construction is idempotent `Bool` —
//! **hash-consing is this choice**: [`union`] is the idempotent union, it
//! destroys multiplicity at write time. The query semiring is the parameter
//! `S` of [`Pli`]: a fold is exact iff its reading factors through what the
//! construction preserved; on a Bool construction, only the cardinal is
//! exact (spec § 5.1).
//!
//! **Fold** [`Pli<S>`]: trivial `H`, the terminal produces the base
//! (`weight(⊤) = 1`, `weight(⊥) = 0`), `φ_aggr` the structure of the
//! semiring. The **expansion-counting** (A4): between a node of level `ℓ`
//! and a child of level `ℓ'`, the levels `ℓ+1..ℓ'` have been collapsed (all
//! their values lead to the same continuation) ⇒ the weight of the arc is
//! multiplied by `∏ |Val(n)|`. The implicit default arc (R-default)
//! contributes the weight of `D_δ` — on the reference instance `D_δ =
//! terminal(false)`, of zero weight (A6: a lifted construction would pay its
//! weight here).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::schema::{Schema, SchemaKeys};
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Query semiring of a fold (spec § 5): `(S, ⊕, ×)` with zeros and units.
/// `depuis_cardinalite` translates `|Val(ℓ)|` into `S` (A4).
pub trait SemiAnneau: Clone + PartialEq + Eq + Hash + Debug + Send + Sync {
    /// Fingerprint of the semiring: enters the memo fingerprint.
    const FINGERPRINT: u64;

    /// Neutral element of ⊕ (zero).
    fn zero() -> Self;
    /// Neutral element of × (one).
    fn un() -> Self;
    /// Cardinality of a level, translated into the semiring (A4).
    fn depuis_cardinalite(cardinalite: u64) -> Self;
    /// ⊕ (aggregation of children).
    fn plus(&self, autre: &Self) -> Self;
    /// × (weights of arcs by subtotals).
    fn fois(&self, autre: &Self) -> Self;
}

/// `(ℕ, +, ×)`: the cardinal counting semiring (spec § 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Naturel(pub u64);

impl SemiAnneau for Naturel {
    const FINGERPRINT: u64 = 4001;

    fn zero() -> Self {
        Naturel(0)
    }
    fn un() -> Self {
        Naturel(1)
    }
    fn depuis_cardinalite(cardinalite: u64) -> Self {
        Naturel(cardinalite)
    }
    fn plus(&self, autre: &Self) -> Self {
        Naturel(self.0 + autre.0)
    }
    fn fois(&self, autre: &Self) -> Self {
        Naturel(self.0 * autre.0)
    }
}

/// `(Bool, ∨, ∧)`: the semiring of ∃-summaries (`exists`, spec § 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BoolOu(pub bool);

impl SemiAnneau for BoolOu {
    const FINGERPRINT: u64 = 4002;

    fn zero() -> Self {
        BoolOu(false)
    }
    fn un() -> Self {
        BoolOu(true)
    }
    fn depuis_cardinalite(cardinalite: u64) -> Self {
        BoolOu(cardinalite > 0)
    }
    fn plus(&self, autre: &Self) -> Self {
        BoolOu(self.0 || autre.0)
    }
    fn fois(&self, autre: &Self) -> Self {
        BoolOu(self.0 && autre.0)
    }
}

/// Fold over a query semiring `S` (spec § 5): trivial `H`, the output
/// diagram is the identity copy of the input, and the synthesized attribute
/// is the value of the fold. `Pli<Naturel>` = `count`.
pub struct Pli<S: SemiAnneau> {
    keys: SchemaKeys,
    nombre_niveaux: u32,
    _semi_anneau: PhantomData<S>,
}

impl<S: SemiAnneau> Pli<S> {
    /// Fold on the given schema (the level cardinalities come from the
    /// schema's key algebra, A4).
    pub fn new(schema: Schema) -> Self {
        let nombre_niveaux = schema.nombre_niveaux();
        Pli {
            keys: SchemaKeys::new(schema),
            nombre_niveaux,
            _semi_anneau: PhantomData,
        }
    }

    /// Weight of a terminal: `weight(⊤) = 1`, `weight(⊥) = 0` (spec § 5).
    fn poids(&self, payload: bool) -> S {
        if payload { S::un() } else { S::zero() }
    }

    /// (A4) multiplier of arc `v` of level `ℓ` towards a child of level
    /// `niveau_cible` (`None` = terminal): product of the cardinalities of
    /// the skipped levels **on the path** of the arc. A skipped value level
    /// has been collapsed with all its explicit values ⇒ factor `|Val(n)|`;
    /// a skipped presence level only collapses through its default arc
    /// (factor 1) and its value level `n+1` is off the path.
    fn multiplicateur(&self, level: Level, arc: u64, niveau_cible: Option<Level>) -> S {
        let borne = match niveau_cible {
            Some(niveau) => niveau.0,
            None => self.nombre_niveaux,
        };
        // Next level on the path of the arc: a value level is always followed;
        // a presence followed by the default arc skips the value level.
        let mut n = match self.keys.default(level) {
            Some(defaut) if arc == defaut => level.0 + 2,
            _ => level.0 + 1,
        };
        let mut produit = S::un();
        while n < borne {
            if self.keys.default(Level(n)).is_some() {
                // Skipped presence level: collapse through default only.
                n += 2;
            } else {
                produit = produit.fois(&S::depuis_cardinalite(self.keys.cardinality(Level(n))));
                n += 1;
            }
        }
        produit
    }
}

impl<S: SemiAnneau> Transduction for Pli<S> {
    type H = ();
    type S = S;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        S::FINGERPRINT
    }

    fn h0(&self) {}

    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }

    fn phi_aggr(&self, level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, S)>) -> S {
        let mut total = S::zero();
        for (arc, (niveau_cible, s)) in arcs {
            let multiplicateur = self.multiplicateur(level, *arc, *niveau_cible);
            total = total.plus(&multiplicateur.fois(s));
        }
        // Implicit default arc (R-default): its continuation is D_δ, of
        // weight `weight(false)` on the reference instance (A6).
        if let Some(defaut) = self.keys.default(level)
            && !arcs.contains_key(&defaut)
        {
            total = total.plus(&self.poids(false));
        }
        total
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
        // Identity copy: the fold does not transform the diagram.
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            crate::memoire::EffetMemoire::new(),
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
        let poids = self.poids(payload);
        (terminal, poids)
    }
}

/// Cardinal count of a diagram (denoted set): fold over `(ℕ, +, ×)`, exact
/// on Bool construction (A5). Raises the result of the synthesized channel.
pub fn compter<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    noeud: NodeId,
) -> Result<u64, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducteur::new(Pli::<Naturel>::new(schema.clone()));
    let (_, total) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(total.0)
}

/// Union of two diagrams with payload `bool` (construction merge, A5): the
/// idempotent union — **hash-consing is this Bool choice**: multiplicity is
/// destroyed at write time, only the cardinal is exact. ZDD style: the
/// terminal `⊤` is the member "all levels by default" (carried by the arc
/// `δ(ℓ)`), and a node of level higher than `ℓ` is the continuation of the
/// default arc of level `ℓ`.
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
    let (forme_a, forme_b) = (pool.resolve(a)?.clone(), pool.resolve(b)?.clone());
    match (forme_a, forme_b) {
        (Form::Terminal(pa), Form::Terminal(pb)) => {
            Ok(pool.intern(anchor, Form::Terminal(pa || pb)))
        }
        (Form::Node { level, arcs }, Form::Terminal(pb)) => {
            if pb {
                let vrai = pool.intern(anchor, Form::Terminal(true));
                union_avec_autre(pool, keys, anchor, level, arcs, vrai)
            } else {
                Ok(a)
            }
        }
        (Form::Terminal(pa), Form::Node { level, arcs }) => {
            if pa {
                let vrai = pool.intern(anchor, Form::Terminal(true));
                union_avec_autre(pool, keys, anchor, level, arcs, vrai)
            } else {
                Ok(b)
            }
        }
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
            if level_a == level_b {
                let mut arcs = arcs_a;
                for (arc, cible_b) in arcs_b {
                    match arcs.remove(&arc) {
                        Some(cible_a) => {
                            let uni = union(pool, keys, anchor, cible_a, cible_b)?;
                            arcs.insert(arc, uni);
                        }
                        None => {
                            arcs.insert(arc, cible_b);
                        }
                    }
                }
                Ok(pool.intern(
                    anchor,
                    Form::Node {
                        level: level_a,
                        arcs,
                    },
                ))
            } else if level_a < level_b {
                // Node b is the continuation of the default arc of a.
                union_avec_autre(pool, keys, anchor, level_a, arcs_a, b)
            } else {
                union_avec_autre(pool, keys, anchor, level_b, arcs_b, a)
            }
        }
    }
}

/// Union of a node of level `level` with another diagram `autre` (terminal
/// `⊤`, or deeper node): the path of `autre` goes through the default arc
/// `δ(level)` if the level has one (absent arcs equal δ); otherwise (value
/// level) it goes through **all** values of the level, explicit or not.
fn union_avec_autre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    level: Level,
    mut arcs: BTreeMap<u64, NodeId>,
    autre: NodeId,
) -> Result<NodeId, CoreError> {
    match keys.default(level) {
        Some(defaut) => {
            let cible = match arcs.remove(&defaut) {
                Some(cible) => union(pool, keys, anchor, cible, autre)?,
                None => autre,
            };
            arcs.insert(defaut, cible);
        }
        None => {
            let cardinalite = keys.cardinality(level);
            for v in 0..cardinalite {
                let cible = match arcs.remove(&v) {
                    Some(cible) => union(pool, keys, anchor, cible, autre)?,
                    None => autre,
                };
                arcs.insert(v, cible);
            }
        }
    }
    Ok(pool.intern(anchor, Form::Node { level, arcs }))
}
