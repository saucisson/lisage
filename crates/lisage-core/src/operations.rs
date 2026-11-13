//! Operations on the inherited channel only (M1-6, spec § 4): `exists`,
//! `filter` (guard = preimage of a predicate) and `map` (image by a
//! function, relabeling). The inherited channel carries the **path** of the
//! map under construction; the verdict is resolved at the terminal
//! (spec § 4: "the verdict is resolved at the value level, but applied at
//! the presence level" — here the inherited channel carries it to the
//! terminal).
//!
//! - [`Exists`] : fold over `(Bool, ∨, ∧)` (spec § 5), `S = BoolOr`.
//! - [`Filter`] : restriction, `S` trivial, the output diagram is
//!   the filtered set (rejected members lead to `ε(false)`).
//! - [`Image`]: relabeling of the atoms of a set (schema `Atom`),
//!   trivial `S`, the level-0 edges are replaced by their image.
//!
//! The inherited channel carries the **set of keys present** on the path.
//! The predicates are therefore presence predicates, well defined on the
//! reduced diagram; a verdict depending on the *values* on a collapsed
//! level `val(k)` requires the T-EXPAND of § 6.2 (delivered for `lookup`),
//! out of scope here.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::fold::{BoolOr, SemiRing};
use crate::schema::{PAS_LEVEL_MAP, PRESENT, Schema};
use crate::transduction::{ContextPhiOut, Transducer, Transduction};

/// Base of the fingerprint namespace of the guard/image family.
const FINGERPRINT_BASE_GARDE: u64 = 5000;

/// Set of keys present on the path of a map under construction.
pub type KeyPresent = BTreeSet<u64>;

/// Presence predicate on a map (example: `p(m) = present(a)`).
pub type PredicateMap = Box<dyn Fn(&KeyPresent) -> bool + Send + Sync>;

/// Transduction `exists(p)` on a set of maps (schema `Map`):
/// fold over `(Bool, ∨, ∧)`, the verdict is resolved at the terminal on the path
/// complet (spec § 5).
pub struct Exists {
    list_keys: Vec<u64>,
    predicate: PredicateMap,
}

impl Exists {
    /// `exists(p)` for the given `Map(list_keys, _)` schema; typed error
    /// if the schema is not a map.
    pub fn new(schema: &Schema, predicate: PredicateMap) -> Result<Self, CoreError> {
        let Schema::Map(list_keys, _) = schema else {
            return Err(CoreError::SchemaUnexpected {
                expected: "Map",
                recu: form_schema(schema),
            });
        };
        Ok(Exists {
            list_keys: list_keys.iter().map(|atom| atom.0).collect(),
            predicate,
        })
    }
}

/// Transduction `filter(p)` (guard) on a set of maps: restriction,
/// rejected members lead to `ε(false)` (spec § 4, `f_filter` = preimage
/// of `p`).
pub struct Filter {
    list_keys: Vec<u64>,
    predicate: PredicateMap,
}

impl Filter {
    /// `filter(p)` for the given `Map(list_keys, _)` schema; typed error
    /// if the schema is not a map.
    pub fn new(schema: &Schema, predicate: PredicateMap) -> Result<Self, CoreError> {
        let Schema::Map(list_keys, _) = schema else {
            return Err(CoreError::SchemaUnexpected {
                expected: "Map",
                recu: form_schema(schema),
            });
        };
        Ok(Filter {
            list_keys: list_keys.iter().map(|atom| atom.0).collect(),
            predicate,
        })
    }
}

/// Name of the form of a schema (for typed errors).
fn form_schema(schema: &Schema) -> &'static str {
    match schema {
        Schema::Atom(_) => "Atom",
        Schema::Set(_) => "Set",
        Schema::Map(_, _) => "Map",
        Schema::Seq(_, _) => "Seq",
        Schema::Tag(_, _) => "Tag",
    }
}

/// Validates that all levels of the diagram are in the schema (otherwise
/// `scatter_path` would index out of bounds): typed `CoreError`.
fn validate_levels<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    node: NodeId,
    visit: &mut BTreeSet<NodeId>,
) -> Result<(), CoreError> {
    if !visit.insert(node) {
        return Ok(());
    }
    match pool.resolve(node)? {
        Form::Terminal(_) => Ok(()),
        Form::Node { level, edges } => {
            if level.0 >= schema.number_levels() {
                return Err(CoreError::LevelOutSchema(*level));
            }
            for target in edges.values() {
                validate_levels(pool, schema, *target, visit)?;
            }
            Ok(())
        }
    }
}

/// Transduction `map(f)` on a set of atoms (schema `Atom`):
/// relabeling, each level-0 edge is replaced by its image under `f`
/// (spec § 4: "map, filter, relabelings").
pub struct Image {
    function: Box<dyn Fn(u64) -> u64 + Send + Sync>,
}

impl Image {
    /// Image under `f` of the atoms of a set (schema `Atom`).
    pub fn new(function: Box<dyn Fn(u64) -> u64 + Send + Sync>) -> Self {
        Image { function }
    }
}

/// Common descendant: updates the set of keys present along edge `v` of
/// level `ℓ` (a presence level with the `present` edge adds the key;
/// a value level changes nothing).
fn scatter_path(list_keys: &[u64], level: Level, h: &KeyPresent, edge: u64) -> KeyPresent {
    let mut h = h.clone();
    if level.0.is_multiple_of(PAS_LEVEL_MAP) && edge == PRESENT {
        let index = (level.0 / PAS_LEVEL_MAP) as usize;
        // Guaranteed by `validate_levels` before the transduction.
        if let Some(&key) = list_keys.get(index) {
            h.insert(key);
        }
    }
    h
}

impl Transduction for Exists {
    type H = KeyPresent;
    type S = BoolOr;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 1
    }

    fn h0(&self) -> KeyPresent {
        KeyPresent::new()
    }

    fn phi_dispatch(&self, level: Level, h: &KeyPresent, edge: u64) -> KeyPresent {
        scatter_path(&self.list_keys, level, h, edge)
    }

    fn phi_aggregate(
        &self,
        _level: Level,
        _h: &KeyPresent,
        edges: &BTreeMap<u64, (Option<Level>, BoolOr)>,
    ) -> BoolOr {
        // (Bool, ∨, ∧) : the A4 multiplier is ⊤ (|Val| ≥ 1), the fold
        // is the ∨ of the subtotals; the implicit default edge leads to
        // D_δ = ε(false), of weight false.
        edges.values().fold(BoolOr(false), |acc, (_, s)| acc.add(s))
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
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
        h: &KeyPresent,
    ) -> (NodeId, BoolOr) {
        let verdict = payload && (self.predicate)(h);
        (
            pool.intern(anchor, Form::Terminal(payload)),
            BoolOr(verdict),
        )
    }
}

impl Transduction for Filter {
    type H = KeyPresent;
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 2
    }

    fn h0(&self) -> KeyPresent {
        KeyPresent::new()
    }

    fn phi_dispatch(&self, level: Level, h: &KeyPresent, edge: u64) -> KeyPresent {
        scatter_path(&self.list_keys, level, h, edge)
    }

    fn phi_aggregate(
        &self,
        _level: Level,
        _h: &KeyPresent,
        _edges: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
        // (A2): if all children lead to ε(false), the node is ε(false)
        // (nodes produced by the runner are always resolvable).
        let all_rejected = !ctx.edges.is_empty()
            && ctx
                .edges
                .values()
                .all(|(n, _)| matches!(pool.resolve(*n), Ok(Form::Terminal(false))));
        if all_rejected {
            return (
                pool.intern(ctx.anchor, Form::Terminal(false)),
                crate::memory::EffectMemory::new(),
            );
        }
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
        h: &KeyPresent,
    ) -> (NodeId, ()) {
        // Keeps: the member is rejected (ε(false)) if the predicate fails.
        let keeps = payload && (self.predicate)(h);
        (pool.intern(anchor, Form::Terminal(keeps)), ())
    }
}

impl Transduction for Image {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 3
    }

    fn h0(&self) {}

    fn phi_dispatch(&self, _level: Level, h: &(), _edge: u64) {
        let _ = h;
    }

    fn phi_aggregate(&self, _level: Level, _h: &(), _edges: &BTreeMap<u64, (Option<Level>, ())>) {}

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
        // Relabeling: edge v becomes f(v).
        let edges = ctx
            .edges
            .into_iter()
            .map(|(v, (n, _))| ((self.function)(v), n))
            .collect();
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
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

/// `exists(p)` on the set of maps denoted by `node` (schema `Map`).
pub fn exists<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    node: NodeId,
    predicate: PredicateMap,
) -> Result<bool, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Exists::new(schema, predicate)?;
    let mut visit = BTreeSet::new();
    validate_levels(pool_input, schema, node, &mut visit)?;
    let mut runner = Transducer::new(transduction);
    let (_, verdict) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(verdict.0)
}

/// `filter(p)` on the set of maps denoted by `node`: returns the
/// diagram of the filtered set (canonical, A3).
pub fn filter<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    node: NodeId,
    predicate: PredicateMap,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Filter::new(schema, predicate)?;
    let mut visit = BTreeSet::new();
    validate_levels(pool_input, schema, node, &mut visit)?;
    let mut runner = Transducer::new(transduction);
    let (result, _) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(result)
}

/// `map(f)` on the set of atoms denoted by `node` (schema `Atom`).
pub fn image<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    node: NodeId,
    function: Box<dyn Fn(u64) -> u64 + Send + Sync>,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducer::new(Image::new(function));
    let (result, _) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(result)
}
