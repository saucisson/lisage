//! Différentiel du pli (M1-6, spec § 5, A4) : voie A = interprète de
//! référence (`lisage_lang::interpreter::count`), voie DD = `Pli<Naturel>`
//! (count sur `(ℕ,+,×)` avec expansion A4 des niveaux collapsés) sur
//! l'ensemble de maps compilé par union. Triptyque C1.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::interpreter::count;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn atome(valeur: u64) -> Value {
    Value::Atom(Atom(valeur))
}

/// Petit ensemble de maps engendré aléatoirement (clés {0,1} × valeurs {0,1}).
fn ensemble_aleatoire(maps: Vec<(u64, u64, u64, u64)>) -> BTreeSet<Value> {
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

/// Compile un ensemble de maps en un seul diagramme (union des membres).
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

proptest! {
    /// C1 différentiel : count DD (voie B, pli `(ℕ,+,×)` + A4) == count
    /// interprète (voie A) sur un ensemble aléatoire de maps. Les ensembles
    /// aléatoires produisent des niveaux collapsés : le chemin A4 est
    /// couvert par la PBT.
    #[test]
    fn test_c1_differentiel_count_voie_a_vs_dd(
        maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)
    ) {
        let schema = Schema::Map(
            vec![Atom(0), Atom(1)],
            Domaine::nouveau(vec![Atom(0), Atom(1)]),
        );
        let ensemble = ensemble_aleatoire(maps);
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
        let dd = compter(&entree, &mut sortie, anchor, &schema, racine).unwrap();
        prop_assert_eq!(dd, count(&ensemble), "voie DD (pli A4) != voie A (count)");
    }
}
