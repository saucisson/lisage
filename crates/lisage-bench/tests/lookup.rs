//! Différentiel lookup (M1-5) : voie A = interprète de référence
//! (`lisage_lang::interpreter::lookup`), voie DD = transduction `lookup`
//! de `lisage-core` sur la map compilée. Triptyque C1, spec § 6.

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

/// Suit le chemin de la map dans le diagramme de sortie et renvoie le
/// payload du terminal atteint.
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
                        Some(_) => noeud = *arcs.get(&PRESENT).expect("arc présent"),
                        // Arc par défaut implicite ⇒ D_δ = ε(None) ; arc
                        // explicite (sous-arbre greffé) ⇒ on le suit.
                        None => match arcs.get(&ABSENT) {
                            Some(&suite) => noeud = suite,
                            None => return None,
                        },
                    }
                } else {
                    let Some(Value::Atom(valeur)) = map.get(&cle) else {
                        panic!("clé présente avec valeur atomique")
                    };
                    noeud = *arcs.get(&valeur.0).expect("arc valeur");
                }
            }
        }
    }
}

proptest! {
    /// C1 différentiel : lookup DD (voie B) == lookup interprète (voie A)
    /// sur une map aléatoire à clés {0,1} et valeurs {0,1}.
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

        // Voie A : interprète de référence sur l'ensemble {map}.
        let ensemble = BTreeSet::from([valeur.clone()]);
        let attendu = lookup(&ensemble, &atome(cle))[&valeur].clone();
        let attendu_opt = match attendu {
            Some(Value::Atom(a)) => Some(a.0),
            _ => None,
        };
        prop_assert_eq!(dd, attendu_opt, "voie DD != voie A pour lookup");
    }

    /// C1 différentiel, cas collapsé : domaine valeur de cardinalité 1 ⇒
    /// le niveau val(k) est R-collapsé à la compilation, `φ_out` le
    /// ré-explicite (T-EXPAND + greffe). La voie DD doit toujours égaler
    /// la voie A.
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
        prop_assert_eq!(dd, attendu_opt, "voie DD != voie A (cas collapsé)");
    }
}
