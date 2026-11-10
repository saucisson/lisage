//! Tests de `lookup(k)` (M1-5, spec § 6) : modes cherche/extrait/stop,
//! fragment de DD synthétisé, T-EXPAND + greffe quand le niveau valeur de
//! la clé a été collapsé.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, PRESENT, ReferenceController, Schema, compiler};
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

/// Exécute `lookup(cle)` sur la map compilée et renvoie (résultat, fragment).
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

/// Suit le chemin de la map dans le diagramme de sortie et renvoie le
/// payload du terminal atteint (None si la clé suivie est absente de la
/// map : l'arc par défaut mène à D_δ = ε(None)).
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
                        None => return None,
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

/// Map à deux clés sur V = {0,1} : lookup retrouve la valeur des deux clés.
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
        "le fragment porte l'arc valeur 1 vers ε(Some(1))"
    );

    let (sortie, resultat, _) =
        executer_lookup(&schema, &liste_cles, &domaine_valeurs, &valeur, Atom(1));
    assert_eq!(
        payload_terminal(&sortie, resultat, &liste_cles, map),
        Some(0),
        "lookup(b) = 0"
    );
}

/// Clé absente de la map : le terminal vaut None (D_δ du pool de sortie).
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
        "lookup(b) = None, clé absente"
    );
    assert!(fragment.is_empty(), "pas de fragment sans Stop");
}

/// Domaine valeur de cardinalité 1 : le niveau val(k) est collapsé à la
/// compilation ; φ_out le ré-explicite (T-EXPAND) et greffe Some(v).
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
        "T-EXPAND : lookup(a) = 0 malgré le niveau valeur collapsé"
    );
    assert!(
        fragment.is_empty(),
        "pas de Stop dans le cas collapsé : la valeur vit dans le DD (T-EXPAND)"
    );
}
