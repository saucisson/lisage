//! Tests de M3-2 (énoncé C3) : réordonner = convertir — `Conversion<T>`
//! compilée en transpositions adjacentes à coût prévisible (inversions),
//! et contrat typé étendu aux encodages via les `Royaume<T, O, E, V>`.

use lisage_lang::Atom;
use lisage_plan::conversion::{Conversion, Encodage, ErreurConversion, MarqueEncodage, Royaume};
use lisage_plan::ordre::{DomaineAtomes, MarqueOrdre, Order, TypeOrdre};

/// Marque portant l'ordre canonique sur un domaine d'atomes.
struct MarqueCanonique;

impl MarqueOrdre<DomaineAtomes> for MarqueCanonique {
    fn ordre(descripteur: &DomaineAtomes) -> Order<DomaineAtomes> {
        Order::canonique(descripteur)
    }
}

/// Marque portant l'ordre inversé sur un domaine d'atomes.
struct MarqueInverse;

impl MarqueOrdre<DomaineAtomes> for MarqueInverse {
    fn ordre(descripteur: &DomaineAtomes) -> Order<DomaineAtomes> {
        let mut pi = descripteur.cles_canoniques();
        pi.reverse();
        Order::nouveau(descripteur, pi).expect("l'ordre inversé est une permutation valide")
    }
}

/// Marque d'encodage : balayage d'univers.
struct MarqueBalayage;

impl MarqueEncodage for MarqueBalayage {
    fn encodage() -> Encodage {
        Encodage::BalayageUnivers
    }
}

/// Marque d'encodage : compilation énumérative.
struct MarqueEnumeration;

impl MarqueEncodage for MarqueEnumeration {
    fn encodage() -> Encodage {
        Encodage::Enumeration
    }
}

/// Nombre d'inversions entre deux séquences (distance de Kendall tau),
/// calculé indépendamment de `Conversion`.
fn inversions<T: TypeOrdre>(source: &Order<T>, cible: &Order<T>) -> usize {
    let sequence = cible.sequence();
    let mut compte = 0;
    for i in 0..sequence.len() {
        for j in i + 1..sequence.len() {
            if source.position(&sequence[i]) > source.position(&sequence[j]) {
                compte += 1;
            }
        }
    }
    compte
}

/// M3-2 : une conversion entre ordres alignés est l'identité (coût nul).
#[test]
fn test_conversion_identite_cout_nul() {
    let domaine = DomaineAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let ordre = Order::canonique(&domaine);
    let conversion = Conversion::nouveau(&ordre, &ordre.clone()).unwrap();
    assert!(conversion.est_identite());
    assert_eq!(conversion.cout(), 0);
    assert!(conversion.transpositions().is_empty());
}

/// M3-2 : réordonner = convertir — la conversion est compilée en
/// transpositions adjacentes dont le nombre est le coût prévisible.
#[test]
fn test_conversion_transpositions_adjacentes() {
    let domaine = DomaineAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let source = Order::canonique(&domaine);
    let cible = Order::nouveau(&domaine, vec![Atom(2), Atom(1), Atom(0)]).unwrap();
    let conversion = Conversion::nouveau(&source, &cible).unwrap();
    // Inversion totale de 3 éléments : 3 inversions, 3 transpositions.
    assert_eq!(conversion.cout(), 3);
    assert_eq!(conversion.transpositions().len(), 3);
    for (i, j) in conversion.transpositions() {
        assert_eq!(*j, i + 1, "transposition adjacente");
    }
    // Appliquer les transpositions à la cible redonne la source.
    let mut sequence = cible.sequence().to_vec();
    for (i, j) in conversion.transpositions() {
        sequence.swap(*i, *j);
    }
    assert_eq!(sequence, source.sequence());
}

/// M3-2 : deux ordres de domaines différents sont inconvertibles.
#[test]
fn test_conversion_ordres_inconvertibles() {
    let domaine_a = DomaineAtomes(vec![Atom(0), Atom(1)]);
    let domaine_b = DomaineAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let ordre_a = Order::canonique(&domaine_a);
    let ordre_b = Order::canonique(&domaine_b);
    assert_eq!(
        Conversion::nouveau(&ordre_a, &ordre_b),
        Err(ErreurConversion::OrdresInconvertibles)
    );
}

/// M3-2 : contrat typé étendu aux encodages — le royaume expose son ordre
/// et son encodage, et `reordonner` exige une conversion compatible avec
/// les marques source et cible.
#[test]
fn test_royaume_reordonner_contrat_type() {
    let domaine = DomaineAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let source = MarqueCanonique::ordre(&domaine);
    let cible = MarqueInverse::ordre(&domaine);
    let conversion = Conversion::nouveau(&source, &cible).unwrap();

    let royaume = Royaume::<DomaineAtomes, MarqueCanonique, MarqueBalayage, u32>::nouveau(7);
    assert_eq!(*royaume.valeur(), 7);
    assert_eq!(
        Royaume::<DomaineAtomes, MarqueCanonique, MarqueBalayage, u32>::encodage(),
        Encodage::BalayageUnivers
    );
    assert_eq!(
        Royaume::<DomaineAtomes, MarqueCanonique, MarqueEnumeration, u32>::encodage(),
        Encodage::Enumeration
    );

    // Conversion compatible : la valeur passe dans le royaume cible.
    let converti = royaume
        .reordonner::<MarqueInverse>(&domaine, &conversion)
        .unwrap();
    assert_eq!(*converti.valeur(), 7);
    assert_eq!(conversion.cout(), 3, "coût prévisible exposé");

    // Conversion incompatible (mauvaise source) : erreur typée.
    let mauvaise = Conversion::nouveau(&cible, &source).unwrap();
    assert_eq!(
        royaume
            .reordonner::<MarqueInverse>(&domaine, &mauvaise)
            .err(),
        Some(ErreurConversion::RoyaumeIncompatible)
    );
}

/// Propriété (LCG déterministe) : pour tous ordres source/cible sur le
/// même domaine, le coût de la conversion égale le nombre d'inversions
/// calculé indépendamment, et les transpositions appliquées à la cible
/// redonnent la source.
#[test]
fn test_propriete_cout_egal_inversions_et_transpositions_exactes() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _cas in 0..50 {
        let taille = 1 + (suivant() % 8) as usize;
        let domaine = DomaineAtomes((0..taille).map(|i| Atom(i as u64)).collect());
        let source = Order::canonique(&domaine);
        // Cible : permutation aléatoire (Fisher-Yates déterministe).
        let mut pi: Vec<Atom> = (0..taille).map(|i| Atom(i as u64)).collect();
        for i in (1..taille).rev() {
            let j = (suivant() as usize) % (i + 1);
            pi.swap(i, j);
        }
        let cible = Order::nouveau(&domaine, pi).unwrap();

        let conversion = Conversion::nouveau(&source, &cible).unwrap();
        assert_eq!(
            conversion.cout(),
            inversions(&source, &cible),
            "coût = inversions (Kendall tau)"
        );
        let mut sequence = cible.sequence().to_vec();
        for (i, j) in conversion.transpositions() {
            sequence.swap(*i, *j);
        }
        assert_eq!(sequence, source.sequence(), "transpositions exactes");
        assert_eq!(conversion.est_identite(), source == cible);
    }
}
