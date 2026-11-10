//! Tests des schémas de référence (M1-3) : compilation balayage d'univers
//! (un niveau de présence par clé, δ = absent, sans marqueur).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{
    ABSENT, Domaine, PRESENT, ReferenceController, Schema, SchemaError, compiler, decompiler,
};
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
    let noeud_valeur_a = arcs[&PRESENT];
    let Form::Node { level, arcs } = pool.resolve(noeud_valeur_a).unwrap() else {
        panic!("nœud de valeur attendu");
    };
    assert_eq!(*level, Level(1));
    assert_eq!(arcs.len(), 1);
    let noeud_presence_b = arcs[&PRESENT];
    let Form::Node { level, arcs } = pool.resolve(noeud_presence_b).unwrap() else {
        panic!("nœud de présence attendu");
    };
    assert_eq!(*level, Level(2));
    let noeud_valeur_b = arcs[&PRESENT];
    let Form::Node { level, arcs } = pool.resolve(noeud_valeur_b).unwrap() else {
        panic!("nœud de valeur attendu");
    };
    assert_eq!(*level, Level(3));
    assert_eq!(arcs, &BTreeMap::from([(ABSENT, terminal)]));
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
    let noeud_valeur_b = pool.intern(
        anchor,
        Form::Node {
            level: Level(3),
            arcs: BTreeMap::from([(ABSENT, terminal)]),
        },
    );
    assert_eq!(
        pool.resolve(racine).unwrap(),
        &Form::Node {
            level: Level(2),
            arcs: BTreeMap::from([(PRESENT, noeud_valeur_b)]),
        }
    );
}

/// Erreurs typées : type incompatible, atome hors domaine.
#[test]
fn test_erreurs_typees() {
    let schema = Schema::Set(domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let erreur = compiler(&mut pool, anchor, &schema, &atome(0)).unwrap_err();
    assert_eq!(
        erreur,
        SchemaError::TypeMismatch {
            attendu: "ensemble",
            recu: "atome"
        }
    );
    let hors = Value::Set(BTreeSet::from([atome(9)]));
    let erreur = compiler(&mut pool, anchor, &schema, &hors).unwrap_err();
    assert_eq!(erreur, SchemaError::HorsDomaine(Atom(9)));
}

/// Schéma Atom(D) : un seul niveau, Val = D.
#[test]
fn test_schema_atome() {
    let schema = Schema::Atom(domaine(&[0, 1, 2]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let racine = compiler(&mut pool, anchor, &schema, &atome(2)).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    assert_eq!(
        pool.resolve(racine).unwrap(),
        &Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(2, terminal)]),
        }
    );
}

/// L'ensemble vide compile vers le terminal d'acceptation ⊤ (chemin vide),
/// et l'ensemble plein vers la chaîne complète des présences.
#[test]
fn test_ensemble_vide_et_plein() {
    let schema = Schema::Set(domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let vide = Value::Set(BTreeSet::new());
    let racine_vide = compiler(&mut pool, anchor, &schema, &vide).unwrap();
    assert_eq!(pool.resolve(racine_vide).unwrap(), &Form::Terminal(true));
    let plein = Value::Set(BTreeSet::from([atome(0), atome(1)]));
    let racine_plein = compiler(&mut pool, anchor, &schema, &plein).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    let noeud_presence_b = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(PRESENT, terminal)]),
        },
    );
    assert_eq!(
        pool.resolve(racine_plein).unwrap(),
        &Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(PRESENT, noeud_presence_b)]),
        }
    );
}

/// La validation d'une map distingue la clé hors domaine de la valeur hors
/// domaine (l'erreur porte l'atome fautif).
#[test]
fn test_erreur_valeur_hors_domaine_map() {
    let schema = Schema::Map(vec![Atom(0)], domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let map = Value::Map(BTreeMap::from([(atome(0), atome(9))]));
    let erreur = compiler(&mut pool, anchor, &schema, &map).unwrap_err();
    assert_eq!(erreur, SchemaError::HorsDomaine(Atom(9)));
    let map = Value::Map(BTreeMap::from([(atome(7), atome(0))]));
    let erreur = compiler(&mut pool, anchor, &schema, &map).unwrap_err();
    assert_eq!(erreur, SchemaError::HorsDomaine(Atom(7)));
}

/// Round-trip : décompiler(compiler(v)) = v.
#[test]
fn test_round_trip() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let map = Value::Map(BTreeMap::from([(atome(0), atome(1))]));
    let racine = compiler(&mut pool, anchor, &schema, &map).unwrap();
    assert_eq!(decompiler(&pool, &schema, racine).unwrap(), map);

    let schema = Schema::Set(domaine(&[0, 1, 2]));
    let mut pool = arena(schema.clone());
    let ensemble = Value::Set(BTreeSet::from([atome(0), atome(2)]));
    let racine = compiler(&mut pool, anchor, &schema, &ensemble).unwrap();
    assert_eq!(decompiler(&pool, &schema, racine).unwrap(), ensemble);

    let schema = Schema::Atom(domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let racine = compiler(&mut pool, anchor, &schema, &atome(1)).unwrap();
    assert_eq!(decompiler(&pool, &schema, racine).unwrap(), atome(1));
}

/// C1 appliqué aux schémas : deux valeurs distinctes compilent vers des
/// nœuds distincts ; la même valeur compile deux fois vers le même nœud.
#[test]
fn test_canonicite_compilation() {
    let schema = Schema::Set(domaine(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let a = Value::Set(BTreeSet::from([atome(0)]));
    let b = Value::Set(BTreeSet::from([atome(1)]));
    let racine_a1 = compiler(&mut pool, anchor, &schema, &a).unwrap();
    let racine_a2 = compiler(&mut pool, anchor, &schema, &a).unwrap();
    let racine_b = compiler(&mut pool, anchor, &schema, &b).unwrap();
    assert_eq!(racine_a1, racine_a2);
    assert_ne!(racine_a1, racine_b);
}

/// M2-1 : Seq — compilation par positions et round-trip.
#[test]
fn test_seq_compilation_round_trip() {
    let schema = Schema::Seq(domaine(&[0, 1, 2]), 3);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);

    // Séquence vide.
    let seq_vide = Value::Seq(vec![]);
    let racine = compiler(&mut pool, anchor, &schema, &seq_vide).unwrap();
    assert_eq!(decompiler(&pool, &schema, racine).unwrap(), seq_vide);

    // Séquence [1, 2].
    let seq = Value::Seq(vec![atome(1), atome(2)]);
    let racine = compiler(&mut pool, anchor, &schema, &seq).unwrap();
    assert_eq!(decompiler(&pool, &schema, racine).unwrap(), seq);

    // Séquence pleine [0, 1, 2].
    let seq_pleine = Value::Seq(vec![atome(0), atome(1), atome(2)]);
    let racine = compiler(&mut pool, anchor, &schema, &seq_pleine).unwrap();
    assert_eq!(decompiler(&pool, &schema, racine).unwrap(), seq_pleine);
}

/// M2-1 : Seq — canonicité (même séquence = même nœud).
#[test]
fn test_seq_canonicite() {
    let schema = Schema::Seq(domaine(&[0, 1]), 2);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let seq = Value::Seq(vec![atome(0), atome(1)]);
    let r1 = compiler(&mut pool, anchor, &schema, &seq).unwrap();
    let r2 = compiler(&mut pool, anchor, &schema, &seq).unwrap();
    assert_eq!(r1, r2);
    let autre = Value::Seq(vec![atome(1), atome(0)]);
    let r3 = compiler(&mut pool, anchor, &schema, &autre).unwrap();
    assert_ne!(r1, r3);
}

/// M2-1 : Seq — erreur si longueur > max.
#[test]
fn test_seq_trop_longue() {
    let schema = Schema::Seq(domaine(&[0, 1]), 2);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let seq = Value::Seq(vec![atome(0), atome(1), atome(0)]);
    assert!(compiler(&mut pool, anchor, &schema, &seq).is_err());
}

/// M2-1 : Seq — erreur si atome hors domaine.
#[test]
fn test_seq_hors_domaine() {
    let schema = Schema::Seq(domaine(&[0, 1]), 2);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let seq = Value::Seq(vec![atome(5)]);
    assert!(matches!(
        compiler(&mut pool, anchor, &schema, &seq),
        Err(SchemaError::HorsDomaine(_))
    ));
}
