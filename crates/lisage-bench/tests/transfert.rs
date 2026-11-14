//! PBT of non-local events (M2-5, T3): for any random map and
//! any pair of keys source < destination, the transfer compiled as a
//! level-memory transduction checks the semantics `m(a) -= 1;
//! m(b) += 1` (if `m(a) ≥ 1` and `m(b)+1` in the domain), and leaves the
//! other keys unchanged.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, NodeId, PolitiqueMemoire};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler, decompiler};
use lisage_core::transduction::Transducteur;
use lisage_core::transfert::TransfertNonLocal;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

/// Number of keys of the random map.
const NOMBRE_CLES: u64 = 3;
/// Size of the value domain.
const TAILLE_DOMAINE: u64 = 4;

type PoolRef = Arena<ReferenceController>;

/// `Map` schema with `NOMBRE_CLES` keys over the domain `{0..TAILLE_DOMAINE}`.
fn schema_test() -> Schema {
    Schema::Map(
        (0..NOMBRE_CLES).map(Atom).collect(),
        Domaine::nouveau((0..TAILLE_DOMAINE).map(Atom).collect()),
    )
}

fn nouveau_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a map `key ↦ value` into the pool.
fn compiler_map(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    entrees: &BTreeMap<u64, u64>,
) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Map(
            entrees
                .iter()
                .map(|(k, v)| (Value::Atom(Atom(*k)), Value::Atom(Atom(*v))))
                .collect(),
        ),
    )
    .unwrap()
}

/// Decompiles a node into a map `key ↦ value`.
fn decompiler_map(pool: &PoolRef, schema: &Schema, noeud: NodeId) -> BTreeMap<u64, u64> {
    match decompiler(pool, schema, noeud).unwrap() {
        Value::Map(map) => map
            .into_iter()
            .map(|(k, v)| match (k, v) {
                (Value::Atom(Atom(k)), Value::Atom(Atom(v))) => (k, v),
                _ => panic!("atoms expected"),
            })
            .collect(),
        _ => panic!("map expected"),
    }
}

proptest! {
    /// Arc semantics of the non-local transfer (T3) on a complete map
    /// (all keys present): `m(a) -= 1` if `m(a) ≥ 1`; `m(b) += 1`
    /// if moreover `m(b) + 1` is in the domain, otherwise `m(b)` unchanged.
    /// The other keys are always unchanged.
    #[test]
    fn test_transfert_semantique(
        valeurs in prop::collection::vec(0u64..TAILLE_DOMAINE, NOMBRE_CLES as usize),
        source in 0u64..NOMBRE_CLES,
        destination in 0u64..NOMBRE_CLES,
    ) {
        prop_assume!(source < destination);
        let entrees: BTreeMap<u64, u64> = (0..NOMBRE_CLES).zip(valeurs).collect();
        let schema = schema_test();
        let anchor = Anchor(0);
        let mut pool = nouveau_pool(&schema);
        let initial = compiler_map(&mut pool, anchor, &schema, &entrees);

        let transfert = TransfertNonLocal::new(
            &schema,
            usize::try_from(source).unwrap(),
            usize::try_from(destination).unwrap(),
        );
        let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
        let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

        let obtenu = decompiler_map(&pool, &schema, resultat);
        let mut attendu = entrees.clone();
        let valeur_source = entrees[&source];
        let valeur_destination = entrees[&destination];
        // The transfer is active only if the source has at least 1 unit.
        if valeur_source >= 1 {
            attendu.insert(source, valeur_source - 1);
            if valeur_destination + 1 < TAILLE_DOMAINE {
                attendu.insert(destination, valeur_destination + 1);
            }
        }
        prop_assert_eq!(obtenu, attendu);
    }
}
