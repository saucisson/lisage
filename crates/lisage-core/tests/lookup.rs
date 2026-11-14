//! Tests for `lookup(k)` (M1-5, spec § 6): search/extract/stop modes,
//! synthesized DD fragment, T-EXPAND + graft when the key's value level was
//! collapsed.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{ABSENT, Domaine, PRESENT, ReferenceController, Schema, compiler};
use lisage_core::transduction::Transducteur;
use lisage_lang::value::{Atom, Value};

type PoolSortie = Arena<LookupController>;

fn atome(valeur: u64) -> Value {
    Value::Atom(Atom(valeur))
}

fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(cle, valeur)| (atome(*cle), atome(*valeur)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Runs `lookup(cle)` on the compiled map and returns (result, fragment).
fn executer_lookup(
    schema: &Schema,
    liste_cles: &[Atom],
    domaine_valeurs: &Domaine,
    valeur: &Value,
    cle: Atom,
) -> (PoolSortie, NodeId, BTreeMap<u64, NodeId>) {
    let mut entree = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut sortie = Arena::new(
        LookupController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let anchor = Anchor(0);
    let racine = compiler(&mut entree, anchor, schema, valeur).unwrap();
    let mut runner = Transducteur::new(Lookup::new(
        cle,
        liste_cles.to_vec(),
        domaine_valeurs.clone(),
    ));
    let (resultat, fragment) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
    (sortie, resultat, fragment)
}

/// Follows the map's path in the output diagram and returns the payload of
/// the reached terminal (None if the followed key is absent from the map:
/// the default arc leads to D_δ = ε(None)).
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
                        Some(_) => noeud = *arcs.get(&PRESENT).expect("present arc"),
                        // Implicit default arc ⇒ D_δ = ε(None); explicit arc
                        // (grafted subtree) ⇒ follow it.
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

/// Two-key map over V = {0,1}: lookup finds the value of both keys.
#[test]
fn test_lookup_cles_presentes() {
    let liste_cles = vec![Atom(0), Atom(1)];
    let domaine_valeurs = Domaine::nouveau(vec![Atom(0), Atom(1)]);
    let schema = Schema::Map(liste_cles.clone(), domaine_valeurs.clone());
    let valeur = map_de(&[(0, 1), (1, 0)]);
    let Value::Map(ref map) = valeur else {
        panic!("map")
    };

    let (sortie, resultat, fragment) =
        executer_lookup(&schema, &liste_cles, &domaine_valeurs, &valeur, Atom(0));
    assert_eq!(
        payload_terminal(&sortie, resultat, &liste_cles, map),
        Some(1),
        "lookup(a) = 1"
    );
    assert!(
        fragment.contains_key(&1),
        "the fragment carries the value arc 1 to ε(Some(1))"
    );

    let (sortie, resultat, _) =
        executer_lookup(&schema, &liste_cles, &domaine_valeurs, &valeur, Atom(1));
    assert_eq!(
        payload_terminal(&sortie, resultat, &liste_cles, map),
        Some(0),
        "lookup(b) = 0"
    );
}

/// Key absent from the map: the terminal is None (D_δ of the output pool).
#[test]
fn test_lookup_cle_absente() {
    let liste_cles = vec![Atom(0), Atom(1)];
    let domaine_valeurs = Domaine::nouveau(vec![Atom(0), Atom(1)]);
    let schema = Schema::Map(liste_cles.clone(), domaine_valeurs.clone());
    let valeur = map_de(&[(0, 1)]);
    let Value::Map(ref map) = valeur else {
        panic!("map")
    };

    let (sortie, resultat, fragment) =
        executer_lookup(&schema, &liste_cles, &domaine_valeurs, &valeur, Atom(1));
    assert_eq!(
        payload_terminal(&sortie, resultat, &liste_cles, map),
        None,
        "lookup(b) = None, absent key"
    );
    assert!(fragment.is_empty(), "no fragment without Stop");
}

/// Value domain of cardinality 1: the val(k) level is collapsed at compile
/// time; φ_out re-explicits it (T-EXPAND) and grafts Some(v).
#[test]
fn test_lookup_niveau_valeur_collapsed_t_expand() {
    let liste_cles = vec![Atom(0)];
    let domaine_valeurs = Domaine::nouveau(vec![Atom(0)]);
    let schema = Schema::Map(liste_cles.clone(), domaine_valeurs.clone());
    let valeur = map_de(&[(0, 0)]);
    let Value::Map(ref map) = valeur else {
        panic!("map")
    };

    let (sortie, resultat, fragment) =
        executer_lookup(&schema, &liste_cles, &domaine_valeurs, &valeur, Atom(0));
    assert_eq!(
        payload_terminal(&sortie, resultat, &liste_cles, map),
        Some(0),
        "T-EXPAND: lookup(a) = 0 despite the collapsed value level"
    );
    assert!(
        fragment.is_empty(),
        "no Stop in the collapsed case: the value lives in the DD (T-EXPAND)"
    );
}
