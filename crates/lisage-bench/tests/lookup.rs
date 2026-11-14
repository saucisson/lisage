//! Lookup differential (M1-5): path A = reference interpreter
//! (`lisage_lang::interpreter::lookup`), DD path = `lookup` transduction
//! of `lisage-core` on the compiled map. Triptych C1, spec § 6.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{ABSENT, Domaine, PRESENT, ReferenceController, Schema, compiler};
use lisage_core::transduction::Transducteur;
use lisage_lang::interpreter::lookup;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolSortie = Arena<LookupController>;

fn atome(valeur: u64) -> Value {
    Value::Atom(Atom(valeur))
}

/// Follows the map's path in the output diagram and returns the
/// payload of the reached terminal.
fn payload_terminal(
    pool: &PoolSortie,
    mut noeud: NodeId,
    liste_cles: &[Atom],
    map: &BTreeMap<Value, Value>,
) -> Option<u64> {
    loop {
        match pool.resolve(noeud).unwrap() {
            Form::Terminal(payload) => return *payload,
            Form::Node { level, arcs } => {
                let index = (level.0 / 2) as usize;
                let cle = Value::Atom(liste_cles[index]);
                if level.0.is_multiple_of(2) {
                    match map.get(&cle) {
                        Some(_) => noeud = *arcs.get(&PRESENT).expect("arc present"),
                        // Implicit default arc ⇒ D_δ = ε(None); explicit
                        // arc (grafted subtree) ⇒ follow it.
                        None => match arcs.get(&ABSENT) {
                            Some(&suite) => noeud = suite,
                            None => return None,
                        },
                    }
                } else {
                    let Some(Value::Atom(valeur)) = map.get(&cle) else {
                        panic!("key present with atomic value")
                    };
                    noeud = *arcs.get(&valeur.0).expect("arc valeur");
                }
            }
        }
    }
}

proptest! {
    /// C1 differential: DD lookup (path B) == interpreter lookup (path A)
    /// on a random map with keys {0,1} and values {0,1}.
    #[test]
    fn test_c1_differentiel_lookup_voie_a_vs_dd(
        paires in prop::collection::vec((0u64..2, 0u64..2), 0..5),
        cle in 0u64..2,
    ) {
        let liste_cles = vec![Atom(0), Atom(1)];
        let domaine_valeurs = Domaine::nouveau(vec![Atom(0), Atom(1)]);
        let schema = Schema::Map(liste_cles.clone(), domaine_valeurs.clone());
        let map: BTreeMap<Value, Value> = paires
            .iter()
            .map(|(k, v)| (atome(*k), atome(*v)))
            .collect();
        let valeur = Value::Map(map.clone());

        let mut entree = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut sortie = Arena::new(
            LookupController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let racine = compiler(&mut entree, anchor, &schema, &valeur).unwrap();
        let cle_atome = Atom(cle);
        let mut runner = Transducteur::new(Lookup::new(
            cle_atome,
            liste_cles.clone(),
            domaine_valeurs.clone(),
        ));
        let (resultat, _) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
        let dd = payload_terminal(&sortie, resultat, &liste_cles, &map);

        // Path A: reference interpreter on the set {map}.
        let ensemble = BTreeSet::from([valeur.clone()]);
        let attendu = lookup(&ensemble, &atome(cle))[&valeur].clone();
        let attendu_opt = match attendu {
            Some(Value::Atom(a)) => Some(a.0),
            _ => None,
        };
        prop_assert_eq!(dd, attendu_opt, "DD path != path A for lookup");
    }

    /// C1 differential, collapsed case: value domain of cardinal 1 ⇒
    /// the val(k) level is R-collapsed at compilation, `φ_out`
    /// re-explicits it (T-EXPAND + graft). The DD path must still equal
    /// path A.
    #[test]
    fn test_c1_differentiel_lookup_collapsed_voie_a_vs_dd(
        paires in prop::collection::vec((0u64..2, 0u64..1), 0..5),
        cle in 0u64..2,
    ) {
        let liste_cles = vec![Atom(0), Atom(1)];
        let domaine_valeurs = Domaine::nouveau(vec![Atom(0)]);
        let schema = Schema::Map(liste_cles.clone(), domaine_valeurs.clone());
        let map: BTreeMap<Value, Value> = paires
            .iter()
            .map(|(k, v)| (atome(*k), atome(*v)))
            .collect();
        let valeur = Value::Map(map.clone());

        let mut entree = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut sortie = Arena::new(
            LookupController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let racine = compiler(&mut entree, anchor, &schema, &valeur).unwrap();
        let cle_atome = Atom(cle);
        let mut runner = Transducteur::new(Lookup::new(
            cle_atome,
            liste_cles.clone(),
            domaine_valeurs.clone(),
        ));
        let (resultat, _) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
        let dd = payload_terminal(&sortie, resultat, &liste_cles, &map);

        let ensemble = BTreeSet::from([valeur.clone()]);
        let attendu = lookup(&ensemble, &atome(cle))[&valeur].clone();
        let attendu_opt = match attendu {
            Some(Value::Atom(a)) => Some(a.0),
            _ => None,
        };
        prop_assert_eq!(dd, attendu_opt, "DD path != path A (collapsed case)");
    }
}
