//! Operations on the inherited channel only (M1-6, spec § 4): `exists`,
//! `filter` (guard = preimage of a predicate) and `map` (image by a
//! function, relabeling). The inherited channel carries the **path** of the
//! map under construction; the verdict is resolved at the terminal (spec
//! § 4: "the verdict is resolved at the value level, but applied at the
//! presence level" — here the inherited channel carries it to the terminal).
//!
//! - [`Existe`]: fold over `(Bool, ∨, ∧)` (spec § 5), `S = BoolOu`.
//! - [`Filtre`]: restriction, trivial `S`, the output diagram is the
//!   filtered set (rejected members lead to `ε(false)`).
//! - [`Image`]: relabeling of the atoms of a set (schema `Atom`), trivial
//!   `S`, the arcs of level 0 are replaced by their image.
//!
//! The inherited channel carries the **set of keys present** on the path.
//! Predicates are therefore presence predicates, well defined on the reduced
//! diagram; a verdict depending on the *values* on a collapsed `val(k)`
//! level requires the T-EXPAND of § 6.2 (delivered for `lookup`), out of
//! scope here.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::pli::{BoolOu, SemiAnneau};
use crate::schema::{PAS_NIVEAU_MAP, PRESENT, Schema};
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Base of the fingerprint namespace of the guard/image family.
const FINGERPRINT_BASE_GARDE: u64 = 5000;

/// Set of keys present on the path of a map under construction.
pub type ClePresentes = BTreeSet<u64>;

/// Presence predicate on a map (example: `p(m) = present(a)`).
pub type PredicatMap = Box<dyn Fn(&ClePresentes) -> bool + Send + Sync>;

/// Transduction `exists(p)` on a set of maps (schema `Map`): fold over
/// `(Bool, ∨, ∧)`, the verdict is resolved at the terminal on the complete
/// path (spec § 5).
pub struct Existe {
    liste_cles: Vec<u64>,
    predicat: PredicatMap,
}

impl Existe {
    /// `exists(p)` for the given `Map(liste_cles, _)` schema; typed error if
    /// the schema is not a map.
    pub fn new(schema: &Schema, predicat: PredicatMap) -> Result<Self, CoreError> {
        let Schema::Map(liste_cles, _) = schema else {
            return Err(CoreError::SchemaInattendu {
                attendu: "Map",
                recu: forme_schema(schema),
            });
        };
        Ok(Existe {
            liste_cles: liste_cles.iter().map(|atome| atome.0).collect(),
            predicat,
        })
    }
}

/// Transduction `filter(p)` (guard) on a set of maps: restriction, rejected
/// members lead to `ε(false)` (spec § 4, `f_filter` = preimage of `p`).
pub struct Filtre {
    liste_cles: Vec<u64>,
    predicat: PredicatMap,
}

impl Filtre {
    /// `filter(p)` for the given `Map(liste_cles, _)` schema; typed error if
    /// the schema is not a map.
    pub fn new(schema: &Schema, predicat: PredicatMap) -> Result<Self, CoreError> {
        let Schema::Map(liste_cles, _) = schema else {
            return Err(CoreError::SchemaInattendu {
                attendu: "Map",
                recu: forme_schema(schema),
            });
        };
        Ok(Filtre {
            liste_cles: liste_cles.iter().map(|atome| atome.0).collect(),
            predicat,
        })
    }
}

/// Name of a schema shape (for typed errors).
fn forme_schema(schema: &Schema) -> &'static str {
    match schema {
        Schema::Atom(_) => "Atom",
        Schema::Set(_) => "Set",
        Schema::Map(_, _) => "Map",
        Schema::Seq(_, _) => "Seq",
        Schema::Tag(_, _) => "Tag",
    }
}

/// Validates that all levels of the diagram are in the schema (otherwise
/// `disperser_chemin` would index out of bounds): typed `CoreError`.
fn valider_niveaux<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    noeud: NodeId,
    visite: &mut BTreeSet<NodeId>,
) -> Result<(), CoreError> {
    if !visite.insert(noeud) {
        return Ok(());
    }
    match pool.resolve(noeud)? {
        Form::Terminal(_) => Ok(()),
        Form::Node { level, arcs } => {
            if level.0 >= schema.nombre_niveaux() {
                return Err(CoreError::NiveauHorsSchema(*level));
            }
            for cible in arcs.values() {
                valider_niveaux(pool, schema, *cible, visite)?;
            }
            Ok(())
        }
    }
}

/// Transduction `map(f)` on a set of atoms (schema `Atom`): relabeling, each
/// arc of level 0 is replaced by its image under `f` (spec § 4: "map, filter,
/// relabelings").
pub struct Image {
    fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>,
}

impl Image {
    /// Image under `f` of the atoms of a set (schema `Atom`).
    pub fn new(fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>) -> Self {
        Image { fonction }
    }
}

/// Common descendant: updates the set of keys present along arc `v` of level
/// `ℓ` (a presence level with the arc `present` adds the key; a value level
/// changes nothing).
fn disperser_chemin(liste_cles: &[u64], level: Level, h: &ClePresentes, arc: u64) -> ClePresentes {
    let mut h = h.clone();
    if level.0.is_multiple_of(PAS_NIVEAU_MAP) && arc == PRESENT {
        let index = (level.0 / PAS_NIVEAU_MAP) as usize;
        // Guaranteed by `valider_niveaux` before the transduction.
        if let Some(&cle) = liste_cles.get(index) {
            h.insert(cle);
        }
    }
    h
}

impl Transduction for Existe {
    type H = ClePresentes;
    type S = BoolOu;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 1
    }

    fn h0(&self) -> ClePresentes {
        ClePresentes::new()
    }

    fn phi_disp(&self, level: Level, h: &ClePresentes, arc: u64) -> ClePresentes {
        disperser_chemin(&self.liste_cles, level, h, arc)
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &ClePresentes,
        arcs: &BTreeMap<u64, (Option<Level>, BoolOu)>,
    ) -> BoolOu {
        // (Bool, ∨, ∧): the multiplier A4 is ⊤ (|Val| ≥ 1), the fold is the
        // ∨ of the subtotals; the implicit default arc leads to D_δ =
        // ε(false), of false weight.
        arcs.values().fold(BoolOu(false), |acc, (_, s)| acc.plus(s))
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
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
        h: &ClePresentes,
    ) -> (NodeId, BoolOu) {
        let verdict = payload && (self.predicat)(h);
        (
            pool.intern(anchor, Form::Terminal(payload)),
            BoolOu(verdict),
        )
    }
}

impl Transduction for Filtre {
    type H = ClePresentes;
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 2
    }

    fn h0(&self) -> ClePresentes {
        ClePresentes::new()
    }

    fn phi_disp(&self, level: Level, h: &ClePresentes, arc: u64) -> ClePresentes {
        disperser_chemin(&self.liste_cles, level, h, arc)
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &ClePresentes,
        _arcs: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
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
                crate::memoire::EffetMemoire::new(),
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
            crate::memoire::EffetMemoire::new(),
        )
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &ClePresentes,
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
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
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
            crate::memoire::EffetMemoire::new(),
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

/// `exists(p)` on the set of maps denoted by `noeud` (schema `Map`).
pub fn existe<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    noeud: NodeId,
    predicat: PredicatMap,
) -> Result<bool, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Existe::new(schema, predicat)?;
    let mut visite = BTreeSet::new();
    valider_niveaux(pool_entree, schema, noeud, &mut visite)?;
    let mut runner = Transducteur::new(transduction);
    let (_, verdict) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(verdict.0)
}

/// `filter(p)` on the set of maps denoted by `noeud`: returns the diagram of
/// the filtered set (canonical, A3).
pub fn filtrer<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    noeud: NodeId,
    predicat: PredicatMap,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Filtre::new(schema, predicat)?;
    let mut visite = BTreeSet::new();
    valider_niveaux(pool_entree, schema, noeud, &mut visite)?;
    let mut runner = Transducteur::new(transduction);
    let (resultat, _) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(resultat)
}

/// `map(f)` on the set of atoms denoted by `noeud` (schema `Atom`).
pub fn image<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    noeud: NodeId,
    fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducteur::new(Image::new(fonction));
    let (resultat, _) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(resultat)
}
