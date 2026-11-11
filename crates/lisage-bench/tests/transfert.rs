//! PBT des événements non locaux (M2-5, T3) : pour toute map aléatoire et
//! tout couple de clés source < destination, le transfert compilé en
//! transduction à mémoire de niveau vérifie la sémantique `m(a) -= 1;
//! m(b) += 1` (si `m(a) ≥ 1` et `m(b)+1` dans le domaine), et laisse les
//! autres clés inchangées.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, NodeId, PolitiqueMemoire};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler, decompiler};
use lisage_core::transduction::Transducteur;
use lisage_core::transfert::TransfertNonLocal;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

/// Nombre de clés de la map aléatoire.
const NOMBRE_CLES: u64 = 3;
/// Taille du domaine des valeurs.
const TAILLE_DOMAINE: u64 = 4;

type PoolRef = Arena<ReferenceController>;

/// Schéma `Map` à `NOMBRE_CLES` clés sur le domaine `{0..TAILLE_DOMAINE}`.
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

/// Compile une map `clé ↦ valeur` dans le pool.
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

/// Décompile un nœud en map `clé ↦ valeur`.
fn decompiler_map(pool: &PoolRef, schema: &Schema, noeud: NodeId) -> BTreeMap<u64, u64> {
    match decompiler(pool, schema, noeud).unwrap() {
        Value::Map(map) => map
            .into_iter()
            .map(|(k, v)| match (k, v) {
                (Value::Atom(Atom(k)), Value::Atom(Atom(v))) => (k, v),
                _ => panic!("atomes attendus"),
            })
            .collect(),
        _ => panic!("map attendue"),
    }
}

proptest! {
    /// Sémantique par arc du transfert non local (T3) sur map complète
    /// (toutes les clés présentes) : `m(a) -= 1` si `m(a) ≥ 1` ; `m(b) += 1`
    /// si de plus `m(b) + 1` est dans le domaine, sinon `m(b)` inchangé.
    /// Les autres clés sont toujours inchangées.
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

        let domaine = Domaine::nouveau((0..TAILLE_DOMAINE).map(Atom).collect());
        let transfert = TransfertNonLocal::new(
            usize::try_from(source).unwrap(),
            usize::try_from(destination).unwrap(),
            domaine,
        );
        let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
        let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

        let obtenu = decompiler_map(&pool, &schema, resultat);
        let mut attendu = entrees.clone();
        let valeur_source = entrees[&source];
        let valeur_destination = entrees[&destination];
        // Le transfert n'est actif que si la source a au moins 1 unité.
        if valeur_source >= 1 {
            attendu.insert(source, valeur_source - 1);
            if valeur_destination + 1 < TAILLE_DOMAINE {
                attendu.insert(destination, valeur_destination + 1);
            }
        }
        prop_assert_eq!(obtenu, attendu);
    }
}
