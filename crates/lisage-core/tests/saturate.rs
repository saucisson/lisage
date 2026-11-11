//! Tests de `saturate` (M2-4, spec § 7, T2/T3) : point fixe croissant,
//! terminaison, restriction par niveaux de la politique Statique, retrait
//! des niveaux stabilisés par la politique AutoDetectee, et chaîne
//! croissante multi-itérations via `saturer`.
//!
//! Les diagrammes dénotent des familles d'ensembles (style ZDD) : le
//! point fixe de `X ⊕ τ(X)` avec une étape émettant le singleton `{a}` est
//! la famille `{X, {a}}` — on compare donc les nœuds (et les cardinaux
//! via `compter`), pas les valeurs décompilées.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::saturate::{BORNE_ITERATIONS, ParametresSaturation, PolitiqueSaturation, saturer};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::{Atom, Value};

/// Fingerprint de la transduction identité de test.
const FINGERPRINT_IDENTITE: u64 = 4001;
/// Fingerprint de la transduction d'étape `EmetSingleton`.
const FINGERPRINT_EMET_SINGLETON: u64 = 4002;

type PoolRef = Arena<ReferenceController>;

/// Transduction identité : copie le diagramme (union avec soi-même).
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

/// Transduction d'étape qui émet toujours le singleton donné, quel que soit
/// le nœud d'entrée : `τ(X) = {{ajout}}` pour tout `X` non terminal. L'union
/// `X ⊕ {{ajout}}` fait croître la famille `X` jusqu'à contenir `{ajout}`.
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

/// Construit un pool neuf pour le schéma donné.
fn nouveau_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compile un ensemble d'atomes dans le pool.
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

/// Cardinal d'un diagramme (nombre de membres de la famille dénotée).
fn cardinal(pool: &PoolRef, schema: &Schema, noeud: NodeId) -> u64 {
    let mut sortie = nouveau_pool(schema);
    compter(pool, &mut sortie, Anchor(0), schema, noeud).unwrap()
}

/// M2-4, T2 : `saturer` termine et atteint un point fixe — avec une étape
/// identité, le point fixe est immédiat (1 itération) et `X ⊕ id(X) = X`.
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
        "identité : point fixe immédiat (T2)"
    );
    assert_eq!(resultat.noeud, initial, "X ⊕ id(X) = X");
}

/// M2-4 : les deux politiques atteignent le même point fixe lorsque
/// `Statique` déclare tous les niveaux du schéma.
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
        "statique : point fixe = initial"
    );
    assert_eq!(
        r_auto.noeud, initial_a,
        "auto-détectée : point fixe = initial"
    );
    assert_eq!(r_statique.iterations, r_auto.iterations);
}

/// M2-4 : la politique `Statique` restreint réellement les niveaux actifs —
/// avec une étape qui émet un singleton à la racine (niveau 0), exclure le
/// niveau 0 de la liste statique laisse le point fixe inchangé, tandis que
/// `AutoDetectee` (tous niveaux actifs) l'atteint avec le singleton ajouté.
/// Ce test échoue si le paramètre `politique` est ignoré.
#[test]
fn test_saturate_statique_restreint_les_niveaux() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]));
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);

    // Politique Statique excluant le niveau racine 0.
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
        "niveau racine exclu : l'étape n'y est pas appliquée, point fixe = initial"
    );
    assert_eq!(r_statique.iterations, 1);

    // Politique AutoDetectee : tous les niveaux actifs, le singleton est
    // atteint.
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
        "tous niveaux actifs : le point fixe contient le singleton émis"
    );
    assert!(
        r_auto.iterations >= 2,
        "chaîne croissante : plusieurs itérations, got {}",
        r_auto.iterations
    );
}

/// M2-4, T2 : `saturer` avec une étape non triviale — chaîne croissante
/// `{{0}} → {{0},{2}}` en appelant `saturer` (plusieurs itérations de la
/// boucle de point fixe, pas une boucle réécrite à la main).
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
        "chaîne croissante : plusieurs itérations, got {}",
        resultat.iterations
    );
    assert!(resultat.iterations <= BORNE_ITERATIONS, "terminaison (T2)");
    assert_eq!(
        resultat.noeud, attendu,
        "point fixe = union de l'initial et du singleton"
    );
    assert_eq!(cardinal(&pool, &schema, resultat.noeud), 2);
}
