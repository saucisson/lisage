//! Tests des schémas de référence (M1-3) : compilation balayage d'univers
//! (un niveau de présence par clé, δ = absent, sans marqueur) et
//! décompilation inverse.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, PRESENT, ReferenceController, Schema, SchemaError, compiler};
use lisage_lang::value::{Atom, Value};

fn domaine(valeurs: &[u64]) -> Domaine {
    Domaine::nouveau(valeurs.iter().map(|&v| Atom(v)).collect())
}

fn arena(schema: Schema) -> Arena<ReferenceController> {
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

fn atome(v: u64) -> Value {
    Value::Atom(Atom(v))
}

/// Exemple de la spec § 1 : K = {a, b}, V = {0, 1}, la map {a↦1, b↦0}
/// compile en chaîne de présences : k_a? → val(a)=1 → k_b? → val(b)=0 → ⊤.
#[test]
fn test_compilation_map_exemple_spec() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let map = Value::Map(BTreeMap::from([(atome(0), atome(1)), (atome(1), atome(0))]));
    let racine = compiler(&mut pool, anchor, &schema, &map).unwrap();

    let terminal = pool.intern(anchor, Form::Terminal(true));
    let Form::Node { level, arcs } = pool.resolve(racine).unwrap() else {
        panic!("la racine doit être un nœud de présence");
    };
    assert_eq!(*level, Level(0));
    let noeud_val_a = arcs[&PRESENT];
    let Form::Node { level, arcs } = pool.resolve(noeud_val_a).unwrap() else {
        panic!("nœud de valeur attendu");
    };
    assert_eq!(*level, Level(1));
    assert_eq!(arcs.len(), 1);
    let noeud_pres_b = arcs[&1];
    let Form::Node { level, arcs } = pool.resolve(noeud_pres_b).unwrap() else {
        panic!("nœud de présence attendu");
    };
    assert_eq!(*level, Level(2));
    let noeud_val_b = arcs[&PRESENT];
    let Form::Node { level, arcs } = pool.resolve(noeud_val_b).unwrap() else {
        panic!("nœud de valeur attendu");
    };
    assert_eq!(*level, Level(3));
    assert_eq!(arcs, &BTreeMap::from([(0, terminal)]));
}

/// R-collapse saute les niveaux de présence des éléments absents (style
/// ZDD) : {a} sur {a, b} se compile en un seul nœud k_a? → présent → ⊤.
#[test]
fn test_ensemble_absents_collapses() {
    let schema = Schema::Set(domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let ensemble = Value::Set(BTreeSet::from([atome(0)]));
    let racine = compiler(&mut pool, anchor, &schema, &ensemble).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    assert_eq!(
        pool.resolve(racine).unwrap(),
        &Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(PRESENT, terminal)]),
        }
    );
}

/// La map vide compile vers ⊤ ; une clé absente saute son niveau de présence.
#[test]
fn test_map_vide_et_cle_absente() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let vide = Value::Map(BTreeMap::new());
    let racine = compiler(&mut pool, anchor, &schema, &vide).unwrap();
    assert_eq!(pool.resolve(racine).unwrap(), &Form::Terminal(true));

    // {b ↦ 0} : le niveau de présence de a est sauté (R-collapse).
    let map = Value::Map(BTreeMap::from([(atome(1), atome(0))]));
    let racine = compiler(&mut pool, anchor, &schema, &map).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    let noeud_val_b = pool.intern(
        anchor,
        Form::Node {
            level: Level(3),
            arcs: BTreeMap::from([(0, terminal)]),
        },
    );
    assert_eq!(
        pool.resolve(racine).unwrap(),
        &Form::Node {
            level: Level(2),
            arcs: BTreeMap::from([(PRESENT, noeud_val_b)]),
        }
    );
}

/// Erreurs typées : type incompatible, atome hors domaine.
#[test]
fn test_erreurs_typees() {
    let schema = Schema::Set(domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let err = compiler(&mut pool, anchor, &schema, &atome(0)).unwrap_err();
    assert_eq!(
        err,
        SchemaError::TypeMismatch {
            attendu: "ensemble",
            recu: "atome"
        }
    );
    let hors = Value::Set(BTreeSet::from([atome(9)]));
    let err = compiler(&mut pool, anchor, &schema, &hors).unwrap_err();
    assert_eq!(err, SchemaError::HorsDomaine(Atom(9)));
}
