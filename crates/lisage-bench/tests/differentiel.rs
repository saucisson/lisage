//! Charte de test (M0-4) + différentiel A/B/C branché dès M1-7.
//!
//! Triptyque : voie A = interprète de référence (vérité), voie B = DD ordre
//! fixe, voie C = Lisage + planner. Convention de nommage `test_c<n>_*`.
//!
//! Les tests différentiels réels contre les voies B/C sont écrits **rouges**
//! avant l'implémentation du cœur (M1-1..M1-6) : ils sont `#[ignore]` tant
//! que la voie visée est factice, et dé-ignorés dans l'issue qui la livre.

use std::collections::{BTreeMap, BTreeSet};

use lisage_bench::differentiel::{voie_a_count, voie_b_count, voie_c_count};
use lisage_bench::{voie_a, voie_b, voie_c};
use lisage_lang::interpreter::{count, filter, lookup};
use lisage_lang::value::Value;
use proptest::prelude::*;

proptest! {
    /// C0 (factice) : les trois voies s'accordent sur n'importe quel entier.
    #[test]
    fn test_c0_triptyque_accord(entier in 0u32..1000) {
        let a = voie_a(entier);
        let b = voie_b(entier);
        let c = voie_c(entier);
        prop_assert_eq!(a, b, "voie A (référence) != voie B");
        prop_assert_eq!(a, c, "voie A (référence) != voie C");
    }
}

#[test]
fn test_c0_exemple_factice() {
    assert_eq!(voie_a(41), 42);
    assert_eq!(voie_b(41), voie_a(41));
    assert_eq!(voie_c(41), voie_a(41));
}

fn atome(valeur: u64) -> Value {
    Value::Atom(valeur)
}

fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(cle, valeur)| (atome(*cle), atome(*valeur)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Petit ensemble de maps engendré aléatoirement (univers {0,1} × clés {0,1}).
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

#[test]
#[ignore = "rouge (M1-7) : voie B factice jusqu'à M1-6 (#13)"]
fn test_c1_differentiel_count_voie_a_vs_b() {
    let ensemble = ensemble_aleatoire(vec![(1, 1, 1, 0), (1, 0, 1, 0)]);
    assert_eq!(voie_a_count(&ensemble), voie_b_count(&ensemble));
}

#[test]
#[ignore = "rouge (M1-7) : voie C factice jusqu'à M1-6 (#13)"]
fn test_c1_differentiel_count_voie_a_vs_c() {
    let ensemble = ensemble_aleatoire(vec![(1, 1, 1, 0), (1, 0, 1, 0)]);
    assert_eq!(voie_a_count(&ensemble), voie_c_count(&ensemble));
}

proptest! {
    /// Voie A (vérité) : le comptage explicite égale la cardinalité ; le
    /// différentiel contre B/C sera dé-ignoré quand les voies DD existeront.
    #[test]
    fn test_voie_a_count_egale_cardinalite(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let ensemble = ensemble_aleatoire(maps);
        prop_assert_eq!(voie_a_count(&ensemble), count(&ensemble));
        prop_assert_eq!(voie_a_count(&ensemble), ensemble.len() as u64);
    }

    /// Voie A : `filter` ne garde qu'un sous-ensemble (count monotone).
    #[test]
    fn test_voie_a_filter_count(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let ensemble = ensemble_aleatoire(maps);
        let garde = filter(&ensemble, |m| m != &map_de(&[(0, 1), (1, 1)]));
        prop_assert!(count(&garde) <= count(&ensemble));
    }

    /// Voie A : `lookup(k)` d'un membre m vaut m(k) (None si absente).
    #[test]
    fn test_voie_a_lookup(maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)) {
        let ensemble = ensemble_aleatoire(maps);
        let resultat = lookup(&ensemble, &atome(0));
        prop_assert_eq!(resultat.len(), ensemble.len());
        for membre in &ensemble {
            let attendu = match membre {
                Value::Map(map) => map.get(&atome(0)).cloned(),
                _ => None,
            };
            prop_assert_eq!(&resultat[membre], &attendu);
        }
    }
}
