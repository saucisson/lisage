//! PBT des schémas de référence (M1-3) : round-trip compilation →
//! décompilation, et canonicité (valeurs distinctes ⇒ nœuds distincts).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::Anchor;
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler, decompiler};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;

fn domaine(valeurs: Vec<u64>) -> Domaine {
    Domaine::nouveau(valeurs.into_iter().map(Atom).collect())
}

fn atome(v: u64) -> Value {
    Value::Atom(Atom(v))
}

fn schema_ensemble() -> BoxedStrategy<(Schema, Value)> {
    (1u8..5, prop::collection::vec(0u8..5, 0..6))
        .prop_map(|(taille, elements)| {
            let domaine = domaine((0..u64::from(taille)).collect());
            let ensemble: BTreeSet<Value> = elements
                .into_iter()
                .filter(|&e| u64::from(e) < u64::from(taille))
                .map(|e| atome(u64::from(e)))
                .collect();
            (Schema::Set(domaine), Value::Set(ensemble))
        })
        .boxed()
}

fn schema_map() -> BoxedStrategy<(Schema, Value)> {
    (
        prop::collection::vec(0u8..4, 1..4),
        prop::collection::vec((0u8..4, 0u8..3), 0..4),
    )
        .prop_map(|(cles, paires)| {
            let cles: Vec<Atom> = cles.into_iter().map(|c| Atom(u64::from(c))).collect();
            let domaine_v = domaine(vec![0, 1, 2]);
            let map: BTreeMap<Value, Value> = paires
                .into_iter()
                .filter(|(c, _)| cles.contains(&Atom(u64::from(*c))))
                .map(|(c, v)| (atome(u64::from(c)), atome(u64::from(v))))
                .collect();
            (Schema::Map(cles, domaine_v), Value::Map(map))
        })
        .boxed()
}

fn schema_atome() -> BoxedStrategy<(Schema, Value)> {
    (1u8..4, 0u8..4)
        .prop_map(|(taille, v)| {
            let domaine = domaine((0..u64::from(taille)).collect());
            let valeur = atome(u64::from(v) % u64::from(taille));
            (Schema::Atom(domaine), valeur)
        })
        .boxed()
}

/// Deux valeurs valides pour le même schéma.
fn schema_et_deux_valeurs() -> BoxedStrategy<(Schema, Value, Value)> {
    prop_oneof![
        (
            1u8..5,
            prop::collection::vec(0u8..5, 0..6),
            prop::collection::vec(0u8..5, 0..6)
        )
            .prop_map(|(taille, ea, eb)| {
                let domaine = domaine((0..u64::from(taille)).collect());
                let mk = |elements: Vec<u8>| {
                    Value::Set(
                        elements
                            .into_iter()
                            .filter(|&e| u64::from(e) < u64::from(taille))
                            .map(|e| atome(u64::from(e)))
                            .collect(),
                    )
                };
                (Schema::Set(domaine), mk(ea), mk(eb))
            }),
        (
            prop::collection::vec(0u8..4, 1..4),
            prop::collection::vec((0u8..4, 0u8..3), 0..4),
            prop::collection::vec((0u8..4, 0u8..3), 0..4),
        )
            .prop_map(|(cles, pa, pb)| {
                let cles: Vec<Atom> = cles.into_iter().map(|c| Atom(u64::from(c))).collect();
                let domaine_v = domaine(vec![0, 1, 2]);
                let cles_filtre = cles.clone();
                let mk = move |paires: Vec<(u8, u8)>| {
                    Value::Map(
                        paires
                            .into_iter()
                            .filter(|(c, _)| cles_filtre.contains(&Atom(u64::from(*c))))
                            .map(|(c, v)| (atome(u64::from(c)), atome(u64::from(v))))
                            .collect(),
                    )
                };
                (Schema::Map(cles, domaine_v), mk(pa), mk(pb))
            }),
        (1u8..4, 0u8..4, 0u8..4).prop_map(|(taille, va, vb)| {
            let domaine = domaine((0..u64::from(taille)).collect());
            (
                Schema::Atom(domaine),
                atome(u64::from(va) % u64::from(taille)),
                atome(u64::from(vb) % u64::from(taille)),
            )
        }),
    ]
    .boxed()
}

proptest! {
    /// Round-trip : décompiler(compiler(v)) = v pour les trois schémas bornés.
    #[test]
    fn test_round_trip_compilation((schema, valeur) in prop_oneof![schema_ensemble(), schema_map(), schema_atome()]) {
        let mut pool = Arena::new(ReferenceController::new(schema.clone()), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let racine = compiler(&mut pool, anchor, &schema, &valeur).unwrap();
        prop_assert_eq!(decompiler(&pool, &schema, racine).unwrap(), valeur);
    }

    /// Canonicité (C1) : deux valeurs distinctes compilent vers des nœuds
    /// distincts ; la même valeur compile deux fois vers le même nœud.
    #[test]
    fn test_canonicite_compilation((schema, valeur_a, valeur_b) in schema_et_deux_valeurs()) {
        let mut pool = Arena::new(ReferenceController::new(schema.clone()), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let racine_a1 = compiler(&mut pool, anchor, &schema, &valeur_a).unwrap();
        let racine_a2 = compiler(&mut pool, anchor, &schema, &valeur_a).unwrap();
        let racine_b = compiler(&mut pool, anchor, &schema, &valeur_b).unwrap();
        prop_assert_eq!(racine_a1, racine_a2);
        if valeur_a != valeur_b {
            prop_assert_ne!(racine_a1, racine_b, "valeurs distinctes ⇒ nœuds distincts (C1)");
        }
    }
}

/// M2-1, T5 : propriété — pour toute séquence aléatoire sur un domaine borné,
/// compiler puis décompiler redonne la séquence (round-trip), et deux
/// séquences égales compilent vers le même nœud (canonicité).
#[test]
fn test_t5_seq_round_trip_propriete() {
    use lisage_core::arena::Arena;
    use lisage_core::observer::NoOpObserver;
    use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler, decompiler};
    use lisage_lang::value::{Atom, Value};
    use proptest::prelude::*;

    proptest!(|(domaine_taille in 1u64..=4,
                 longueur_max in 1u32..=4,
                 elements in prop::collection::vec(0u64..4, 0..5))| {
        let domaine = Domaine::nouveau((0..domaine_taille).map(Atom).collect());
        let schema = Schema::Seq(domaine.clone(), longueur_max);
        let mut pool = Arena::new(ReferenceController::new(schema.clone()), Box::new(NoOpObserver));
        let anchor = lisage_algebra::interface::Anchor(0);
        // Tronque à la longueur max et filtre dans le domaine.
        let seq: Vec<Value> = elements.iter()
            .take(longueur_max as usize)
            .filter(|&&v| v < domaine_taille)
            .map(|&v| Value::Atom(Atom(v)))
            .collect();
        let valeur = Value::Seq(seq.clone());
        let racine = compiler(&mut pool, anchor, &schema, &valeur).unwrap();
        let retour = decompiler(&pool, &schema, racine).unwrap();
        prop_assert_eq!(retour, valeur.clone(), "round-trip Seq");
        // Canonicité.
        let racine2 = compiler(&mut pool, anchor, &schema, &valeur).unwrap();
        prop_assert_eq!(racine, racine2, "canonicité Seq");
    });
}
