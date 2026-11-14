//! Tests for `saturate` (M2-4, spec § 7, T2/T3): increasing fixed point,
//! termination, level restriction of the Statique policy, removal of the
//! levels stabilized by the AutoDetectee policy, and multi-iteration
//! increasing chain via `saturer`.
//!
//! The diagrams denote families of sets (ZDD style): the fixed point of
//! `X ⊕ τ(X)` with a step emitting the singleton `{a}` is the family
//! `{X, {a}}` — so we compare nodes (and cardinalities via `compter`), not
//! decompiled values.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::saturate::{BORNE_ITERATIONS, ParametresSaturation, PolitiqueSaturation, saturer};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the test identity transduction.
const FINGERPRINT_IDENTITE: u64 = 4001;
/// Fingerprint of the `EmetSingleton` step transduction.
const FINGERPRINT_EMET_SINGLETON: u64 = 4002;

type PoolRef = Arena<ReferenceController>;

/// Identity transduction: copies the diagram (union with itself).
#[derive(Clone)]
struct Identite;

impl Transduction for Identite {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_IDENTITE
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
    ) -> (NodeId, lisage_core::memoire::EffetMemoire<()>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memoire::EffetMemoire::new(),
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

/// Step transduction that always emits the given singleton, whatever the
/// input node: `τ(X) = {{ajout}}` for any non-terminal `X`. The union
/// `X ⊕ {{ajout}}` grows the family `X` until it contains `{ajout}`.
#[derive(Clone)]
struct EmetSingleton {
    singleton: NodeId,
}

impl Transduction for EmetSingleton {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_EMET_SINGLETON
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        _pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memoire::EffetMemoire<()>) {
        let _ = ctx;
        (self.singleton, lisage_core::memoire::EffetMemoire::new())
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        _payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(false)), ())
    }
}

/// Builds a fresh pool for the given schema.
fn nouveau_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of atoms in the pool.
fn compiler_ensemble(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    atomes: &[u64],
) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atomes.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// Cardinality of a diagram (number of members of the denoted family).
fn cardinal(pool: &PoolRef, schema: &Schema, noeud: NodeId) -> u64 {
    let mut sortie = nouveau_pool(schema);
    compter(pool, &mut sortie, Anchor(0), schema, noeud).unwrap()
}

/// M2-4, T2: `saturer` terminates and reaches a fixed point — with an identity
/// step, the fixed point is immediate (1 iteration) and `X ⊕ id(X) = X`.
#[test]
fn test_saturate_termine_et_point_fixe() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = nouveau_pool(&schema);
    let anchor = Anchor(0);
    let initial = compiler_ensemble(&mut pool, anchor, &schema, &[0]);

    let politique = PolitiqueSaturation::AutoDetectee;
    let mut params = ParametresSaturation {
        transduction: &Identite,
        politique: &politique,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let resultat = saturer(&mut params).unwrap();
    assert_eq!(
        resultat.iterations, 1,
        "identity: immediate fixed point (T2)"
    );
    assert_eq!(resultat.noeud, initial, "X ⊕ id(X) = X");
}

/// M2-4: both policies reach the same fixed point when `Statique` declares
/// all the levels of the schema.
#[test]
fn test_saturate_politiques_equivalentes() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    let niveaux: Vec<Level> = (0..schema.nombre_niveaux()).map(Level).collect();
    let politique_statique = PolitiqueSaturation::Statique(niveaux);
    let politique_auto = PolitiqueSaturation::AutoDetectee;

    let mut pool_s = nouveau_pool(&schema);
    let initial_s = compiler_ensemble(&mut pool_s, anchor, &schema, &[0]);
    let mut params = ParametresSaturation {
        transduction: &Identite,
        politique: &politique_statique,
        pool: &mut pool_s,
        keys: &keys,
        anchor,
        initial: initial_s,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let r_statique = saturer(&mut params).unwrap();

    let mut pool_a = nouveau_pool(&schema);
    let initial_a = compiler_ensemble(&mut pool_a, anchor, &schema, &[0]);
    let mut params = ParametresSaturation {
        transduction: &Identite,
        politique: &politique_auto,
        pool: &mut pool_a,
        keys: &keys,
        anchor,
        initial: initial_a,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let r_auto = saturer(&mut params).unwrap();

    assert_eq!(
        r_statique.noeud, initial_s,
        "statique: fixed point = initial"
    );
    assert_eq!(
        r_auto.noeud, initial_a,
        "auto-detected: fixed point = initial"
    );
    assert_eq!(r_statique.iterations, r_auto.iterations);
}

/// M2-4: the `Statique` policy really restricts the active levels — with a
/// step that emits a singleton at the root (level 0), excluding level 0 from
/// the static list leaves the fixed point unchanged, while `AutoDetectee` (all
/// levels active) reaches it with the singleton added. This test fails if the
/// `politique` parameter is ignored.
#[test]
fn test_saturate_statique_restreint_les_niveaux() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    // Statique policy excluding the root level 0.
    let niveaux_sans_racine: Vec<Level> = (1..schema.nombre_niveaux()).map(Level).collect();
    let politique_statique = PolitiqueSaturation::Statique(niveaux_sans_racine);
    let mut pool_s = nouveau_pool(&schema);
    let initial_s = compiler_ensemble(&mut pool_s, anchor, &schema, &[0]);
    let singleton_s = compiler_ensemble(&mut pool_s, anchor, &schema, &[2]);
    let etape_s = EmetSingleton {
        singleton: singleton_s,
    };
    let mut params = ParametresSaturation {
        transduction: &etape_s,
        politique: &politique_statique,
        pool: &mut pool_s,
        keys: &keys,
        anchor,
        initial: initial_s,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let r_statique = saturer(&mut params).unwrap();
    assert_eq!(
        r_statique.noeud, initial_s,
        "root level excluded: the step is not applied there, fixed point = initial"
    );
    assert_eq!(r_statique.iterations, 1);

    // AutoDetectee policy: all levels active, the singleton is reached.
    let politique_auto = PolitiqueSaturation::AutoDetectee;
    let mut pool_a = nouveau_pool(&schema);
    let initial_a = compiler_ensemble(&mut pool_a, anchor, &schema, &[0]);
    let singleton_a = compiler_ensemble(&mut pool_a, anchor, &schema, &[2]);
    let etape_a = EmetSingleton {
        singleton: singleton_a,
    };
    let attendu_a = union(&mut pool_a, &keys, anchor, initial_a, singleton_a).unwrap();
    let mut params = ParametresSaturation {
        transduction: &etape_a,
        politique: &politique_auto,
        pool: &mut pool_a,
        keys: &keys,
        anchor,
        initial: initial_a,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let r_auto = saturer(&mut params).unwrap();
    assert_eq!(
        r_auto.noeud, attendu_a,
        "all levels active: the fixed point contains the emitted singleton"
    );
    assert!(
        r_auto.iterations >= 2,
        "increasing chain: several iterations, got {}",
        r_auto.iterations
    );
}

/// M2-4, T2: `saturer` with a non-trivial step — increasing chain
/// `{{0}} → {{0},{2}}` by calling `saturer` (several iterations of the fixed
/// point loop, not a hand-rewritten loop).
#[test]
fn test_saturate_chaine_croissante_multi_iterations() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = nouveau_pool(&schema);
    let anchor = Anchor(0);

    let initial = compiler_ensemble(&mut pool, anchor, &schema, &[0]);
    let singleton_2 = compiler_ensemble(&mut pool, anchor, &schema, &[2]);
    let attendu = union(&mut pool, &keys, anchor, initial, singleton_2).unwrap();
    let etape = EmetSingleton {
        singleton: singleton_2,
    };
    let politique = PolitiqueSaturation::AutoDetectee;
    let mut params = ParametresSaturation {
        transduction: &etape,
        politique: &politique,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let resultat = saturer(&mut params).unwrap();
    assert!(
        resultat.iterations >= 2,
        "increasing chain: several iterations, got {}",
        resultat.iterations
    );
    assert!(resultat.iterations <= BORNE_ITERATIONS, "termination (T2)");
    assert_eq!(
        resultat.noeud, attendu,
        "fixed point = union of the initial and the singleton"
    );
    assert_eq!(cardinal(&pool, &schema, resultat.noeud), 2);
}
