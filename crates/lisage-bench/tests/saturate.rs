//! PBT de `saturate` (M2-4, T2) : invariant de point fixe — `saturer`
//! appliqué à son propre résultat termine en **une** itération avec le
//! même nœud (idempotence), et le cardinal du point fixe est supérieur ou
//! égal à celui de l'initial (chaîne croissante pour l'inclusion des
//! familles dénotées).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::compter;
use lisage_core::saturate::{ParametresSaturation, PolitiqueSaturation, saturer};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

/// Taille du domaine d'atomes des schémas aléatoires.
const TAILLE_DOMAINE: u64 = 3;
/// Fingerprint de la transduction d'étape PBT.
const FINGERPRINT_ETAPE: u64 = 4003;

type PoolRef = Arena<ReferenceController>;

/// Transduction d'étape : émet toujours le singleton `{ajout}` quel que soit
/// le nœud d'entrée (terminaux → ⊥).
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
        FINGERPRINT_ETAPE
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

/// Schéma `Set` sur le domaine `{0, 1, 2}`.
fn schema_test() -> Schema {
    Schema::Set(Domaine::nouveau((0..TAILLE_DOMAINE).map(Atom).collect()))
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

proptest! {
    /// Invariant de point fixe (T2) : soit `R = saturer(τ, D)` ; alors
    /// `saturer(τ, R)` termine en 1 itération avec le même nœud `R`
    /// (idempotence), et le cardinal de `R` est supérieur ou égal à celui
    /// de `D` (chaîne croissante).
    #[test]
    fn test_saturate_point_fixe_idempotent(
        initial in prop::collection::vec(0u64..TAILLE_DOMAINE, 0..TAILLE_DOMAINE as usize),
        ajout in 0u64..TAILLE_DOMAINE,
    ) {
        let schema = schema_test();
        let keys = SchemaKeys::new(schema.clone());
        let anchor = Anchor(0);
        let niveaux: Vec<Level> = (0..schema.nombre_niveaux()).map(Level).collect();
        let politique = PolitiqueSaturation::Statique(niveaux);

        let mut pool = nouveau_pool(&schema);
        let noeud_initial = compiler_ensemble(&mut pool, anchor, &schema, &initial);
        let singleton = compiler_ensemble(&mut pool, anchor, &schema, &[ajout]);
        let etape = EmetSingleton { singleton };

        let mut params = ParametresSaturation {
            transduction: &etape,
            politique: &politique,
            pool: &mut pool,
            keys: &keys,
            anchor,
            initial: noeud_initial,
            nombre_niveaux: schema.nombre_niveaux(),
            observateur: None,
        };
        let premier = saturer(&mut params).unwrap();

        // Chaîne croissante : le point fixe contient la famille initiale.
        let cardinal_initial = cardinal(&pool, &schema, noeud_initial);
        let cardinal_fixe = cardinal(&pool, &schema, premier.noeud);
        prop_assert!(
            cardinal_fixe >= cardinal_initial,
            "point fixe ⊇ initial : {cardinal_fixe} >= {cardinal_initial}"
        );

        // Idempotence : saturer appliqué au résultat termine en 1 itération
        // avec le même nœud.
        let mut params = ParametresSaturation {
            transduction: &etape,
            politique: &politique,
            pool: &mut pool,
            keys: &keys,
            anchor,
            initial: premier.noeud,
            nombre_niveaux: schema.nombre_niveaux(),
            observateur: None,
        };
        let second = saturer(&mut params).unwrap();
        prop_assert_eq!(second.iterations, 1, "point fixe : 1 itération");
        prop_assert_eq!(second.noeud, premier.noeud, "idempotence du point fixe");
    }
}
