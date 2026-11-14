//! Differentials of the inherited-channel-only operations (M1-6, spec § 4):
//! path A = reference interpreter (`lisage_lang::interpreter`),
//! DD path = `exists`/`filter`/`map` transductions of `lisage-core`.
//! Triptych C1. For filter/map, the equality is structural (C1: same
//! set ⇒ same node in the same pool).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::operations::{ClePresentes, existe, filtrer, image};
use lisage_core::pli::union;
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::interpreter::{exists, filter, map};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn atome(valeur: u64) -> Value {
    Value::Atom(Atom(valeur))
}

/// Small set of maps generated randomly (keys {0,1} × values {0,1}).
fn ensemble_maps_aleatoire(maps: Vec<(u64, u64, u64, u64)>) -> BTreeSet<Value> {
    maps.into_iter()
        .map(|(a, b, c, d)| {
            let mut map = BTreeMap::new();
            if a == 1 {
                map.insert(atome(0), atome(b));
            }
            if c == 1 {
                map.insert(atome(1), atome(d));
            }
            Value::Map(map)
        })
        .collect()
}

fn compiler_ensemble(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    ensemble: &BTreeSet<Value>,
) -> NodeId {
    let keys = SchemaKeys::new(schema.clone());
    let mut resultat = pool.intern(anchor, Form::Terminal(false));
    for membre in ensemble {
        let n = compiler(pool, anchor, schema, membre).unwrap();
        resultat = union(pool, &keys, anchor, resultat, n).unwrap();
    }
    resultat
}

/// Presence predicate, DD side: `p(m) = present(a)`.
fn predicat_dd(cles: &ClePresentes) -> bool {
    cles.contains(&0)
}

/// Same predicate, path A side (reference interpreter).
fn predicat_lang(valeur: &Value) -> bool {
    match valeur {
        Value::Map(map) => map.contains_key(&atome(0)),
        _ => false,
    }
}

proptest! {
    /// C1 differential: exists DD (pli `(Bool,∨,∧)`) == exists interpreter.
    #[test]
    fn test_c1_differentiel_exists_voie_a_vs_dd(
        maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)
    ) {
        let schema = Schema::Map(
            vec![Atom(0), Atom(1)],
            Domaine::nouveau(vec![Atom(0), Atom(1)]),
        );
        let ensemble = ensemble_maps_aleatoire(maps);
        let mut entree = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut sortie = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);
        let dd = existe(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(predicat_dd),
        )
        .unwrap();
        prop_assert_eq!(dd, exists(&ensemble, predicat_lang), "DD path != path A (exists)");
    }

    /// C1 differential: filter DD == set filtered by path A,
    /// compared structurally (C1: same set ⇒ same node).
    #[test]
    fn test_c1_differentiel_filter_voie_a_vs_dd(
        maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)
    ) {
        let schema = Schema::Map(
            vec![Atom(0), Atom(1)],
            Domaine::nouveau(vec![Atom(0), Atom(1)]),
        );
        let ensemble = ensemble_maps_aleatoire(maps);
        let mut entree = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut sortie = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);
        let dd = filtrer(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(predicat_dd),
        )
        .unwrap();
        let attendu_ensemble = filter(&ensemble, predicat_lang);
        let attendu = compiler_ensemble(&mut sortie, anchor, &schema, &attendu_ensemble);
        prop_assert_eq!(dd, attendu, "DD path != path A (filter), C1 structural");
    }

    /// C1 differential: map(f) DD (relabeling) == map interpreter on a
    /// random set of atoms, compared structurally.
    #[test]
    fn test_c1_differentiel_map_voie_a_vs_dd(
        atomes in prop::collection::btree_set(0u64..3, 0..4)
    ) {
        let schema = Schema::Atom(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]));
        let ensemble: BTreeSet<Value> = atomes.iter().map(|v| atome(*v)).collect();
        let mut entree = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut sortie = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);
        let dd = image(&entree, &mut sortie, anchor, racine, Box::new(|v| (v + 1) % 3)).unwrap();
        let attendu_ensemble = map(&ensemble, |v| match v {
            Value::Atom(a) => atome((a.0 + 1) % 3),
            autre => autre.clone(),
        });
        let attendu = compiler_ensemble(&mut sortie, anchor, &schema, &attendu_ensemble);
        prop_assert_eq!(dd, attendu, "DD path != path A (map), C1 structural");
    }
}
