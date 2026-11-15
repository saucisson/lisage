//! Operations on the inherited channel only (M1-6, spec § 4): `exists`,
//! `filter` (guard = preimage of a predicate) and `map` (image by a
//! function, relabeling). The inherited channel carries the **path** of the
//! map under construction; the verdict is resolved at the terminal
//! (spec § 4: "the verdict is resolved at the value level, but applied at
//! the presence level" — here the inherited channel carries it down to the
//! terminal).
//!
//! - [`Existe`]: fold over `(Bool, ∨, ∧)` (spec § 5), `S = BoolOu`.
//! - [`Filtre`]: restriction, `S` trivial, the output diagram is the
//!   filtered set (rejected members lead to `ε(false)`).
//! - [`Image`]: relabeling of the atoms of a set (schema `Atom`), `S`
//!   trivial, the arcs of level 0 are replaced by their image.
//!
//! The inherited channel carries the **set of keys present** on the path.
//! Predicates are therefore presence predicates, well-defined on the
//! reduced diagram; a verdict depending on the *values* at a collapsed
//! `val(k)` level requires the T-EXPAND of § 6.2 (delivered for `lookup`),
//! out of scope here.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::pli::{BoolOu, SemiAnneau};
use crate::schema::{PAS_LEVEL_MAP, PRESENT, Schema};
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Base of the fingerprint namspace of the guard/image family.
const FINGERPRINT_BASE_GARDE: u64 = 5000;

/// Set of keys present on the path of a map under construction.
pub type KeyPresentes = BTreeSet<u64>;

/// Presence predicate on a map (example: `p(m) = present(a)`).
pub type PredicatMap = Box<dyn Fn(&KeyPresentes) -> bool + Send + Sync>;

/// Transduction `exists(p)` on a set of maps (schema `Map`): fold over
/// `(Bool, ∨, ∧)`, the verdict is resolved at the terminal on the complete
/// path (spec § 5).
pub struct Existe {
    list_keys: Vec<u64>,
    predicat: PredicatMap,
}

impl Existe {
    /// `exists(p)` for the given `Map(list_keys, _)` schema; typed error if
    /// the schema is not a map.
    pub fn new(schema: &Schema, predicat: PredicatMap) -> Result<Self, CoreError> {
        let Schema::Map(list_keys, _) = schema else {
            return Err(CoreError::SchemaInexpected {
                expected: "Map",
                recu: form_schema(schema),
            });
        };
        Ok(Existe {
            list_keys: list_keys.iter().map(|atome| atome.0).collect(),
            predicat,
        })
    }
}

/// Transduction `filter(p)` (guard) on a set of maps: restriction, rejected
/// members lead to `ε(false)` (spec § 4, `f_filter` = preimage of `p`).
pub struct Filtre {
    list_keys: Vec<u64>,
    predicat: PredicatMap,
}

impl Filtre {
    /// `filter(p)` for the given `Map(list_keys, _)` schema; typed error if
    /// the schema is not a map.
    pub fn new(schema: &Schema, predicat: PredicatMap) -> Result<Self, CoreError> {
        let Schema::Map(list_keys, _) = schema else {
            return Err(CoreError::SchemaInexpected {
                expected: "Map",
                recu: form_schema(schema),
            });
        };
        Ok(Filtre {
            list_keys: list_keys.iter().map(|atome| atome.0).collect(),
            predicat,
        })
    }
}

/// Name of the shape of a schema (for typed errors).
fn form_schema(schema: &Schema) -> &'static str {
    match schema {
        Schema::Atom(_) => "Atom",
        Schema::Set(_) => "Set",
        Schema::Map(_, _) => "Map",
        Schema::Seq(_, _) => "Seq",
        Schema::Tag(_, _) => "Tag",
    }
}

/// Checks that all levels of the diagram are in the schema (otherwise
/// `disperser_path` would index out of bounds): typed `CoreError`.
fn valider_levels<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    node: NodeId,
    visite: &mut BTreeSet<NodeId>,
) -> Result<(), CoreError> {
    if !visite.insert(node) {
        return Ok(());
    }
    match pool.resolve(node)? {
        Form::Terminal(_) => Ok(()),
        Form::Node { level, arcs } => {
            if level.0 >= schema.number_levels() {
                return Err(CoreError::LevelHorsSchema(*level));
            }
            for cible in arcs.values() {
                valider_levels(pool, schema, *cible, visite)?;
            }
            Ok(())
        }
    }
}

/// Transduction `map(f)` on a set of atoms (schema `Atom`): relabeling, each
/// arc of level 0 is replaced by its image under `f` (spec § 4: "map,
/// filter, relabelings").
pub struct Image {
    fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>,
}

impl Image {
    /// Image under `f` of the atoms of a set (schema `Atom`).
    pub fn new(fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>) -> Self {
        Image { fonction }
    }
}

/// Common descendant: updates the set of present keys along arc `v` of
/// level `ℓ` (a presence level with the `present` arc adds the key; a value
/// level changes nothing).
fn disperser_path(list_keys: &[u64], level: Level, h: &KeyPresentes, arc: u64) -> KeyPresentes {
    let mut h = h.clone();
    if level.0.is_multiple_of(PAS_LEVEL_MAP) && arc == PRESENT {
        let index = (level.0 / PAS_LEVEL_MAP) as usize;
        // Guaranteed by `valider_levels` before the transduction.
        if let Some(&key) = list_keys.get(index) {
            h.insert(key);
        }
    }
    h
}

impl Transduction for Existe {
    type H = KeyPresentes;
    type S = BoolOu;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 1
    }

    fn h0(&self) -> KeyPresentes {
        KeyPresentes::new()
    }

    fn phi_disp(&self, level: Level, h: &KeyPresentes, arc: u64) -> KeyPresentes {
        disperser_path(&self.list_keys, level, h, arc)
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &KeyPresentes,
        arcs: &BTreeMap<u64, (Option<Level>, BoolOu)>,
    ) -> BoolOu {
        // (Bool, ∨, ∧): the A4 multiplier is ⊤ (|Val| ≥ 1), the fold is the
        // ∨ of the subtotals; the implicit default arc leads to
        // D_δ = ε(false), of weight false.
        arcs.values().fold(BoolOu(false), |acc, (_, s)| acc.plus(s))
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffetMemory<Self::M>) {
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            crate::memory::EffetMemory::new(),
        )
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &KeyPresentes,
    ) -> (NodeId, BoolOu) {
        let verdict = payload && (self.predicat)(h);
        (
            pool.intern(anchor, Form::Terminal(payload)),
            BoolOu(verdict),
        )
    }
}

impl Transduction for Filtre {
    type H = KeyPresentes;
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 2
    }

    fn h0(&self) -> KeyPresentes {
        KeyPresentes::new()
    }

    fn phi_disp(&self, level: Level, h: &KeyPresentes, arc: u64) -> KeyPresentes {
        disperser_path(&self.list_keys, level, h, arc)
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &KeyPresentes,
        _arcs: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffetMemory<Self::M>) {
        // (A2): if all children lead to ε(false), the node is ε(false)
        // (nodes produced by the runner are always resolvable).
        let tout_rejete = !ctx.arcs.is_empty()
            && ctx
                .arcs
                .values()
                .all(|(n, _)| matches!(pool.resolve(*n), Ok(Form::Terminal(false))));
        if tout_rejete {
            return (
                pool.intern(ctx.anchor, Form::Terminal(false)),
                crate::memory::EffetMemory::new(),
            );
        }
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            crate::memory::EffetMemory::new(),
        )
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &KeyPresentes,
    ) -> (NodeId, ()) {
        // Guard: the member is rejected (ε(false)) if the predicate fails.
        let garde = payload && (self.predicat)(h);
        (pool.intern(anchor, Form::Terminal(garde)), ())
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

    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }

    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffetMemory<Self::M>) {
        // Relabeling: arc v becomes f(v).
        let arcs = ctx
            .arcs
            .into_iter()
            .map(|(v, (n, _))| ((self.fonction)(v), n))
            .collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            crate::memory::EffetMemory::new(),
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
pub fn existe<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    node: NodeId,
    predicat: PredicatMap,
) -> Result<bool, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Existe::new(schema, predicat)?;
    let mut visite = BTreeSet::new();
    valider_levels(pool_input, schema, node, &mut visite)?;
    let mut runner = Transducteur::new(transduction);
    let (_, verdict) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(verdict.0)
}

/// `filter(p)` on the set of maps denoted by `node`: returns the diagram
/// of the filtered set (canonical, A3).
pub fn filtrer<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    node: NodeId,
    predicat: PredicatMap,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Filtre::new(schema, predicat)?;
    let mut visite = BTreeSet::new();
    valider_levels(pool_input, schema, node, &mut visite)?;
    let mut runner = Transducteur::new(transduction);
    let (result, _) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(result)
}

/// `map(f)` on the set of atoms denoted by `node` (schema `Atom`).
pub fn image<PIn, POut>(
    pool_input: &PIn,
    pool_output: &mut POut,
    anchor: Anchor,
    node: NodeId,
    fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducteur::new(Image::new(fonction));
    let (result, _) = runner.run(pool_input, pool_output, anchor, node)?;
    Ok(result)
}
