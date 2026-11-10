//! Démo philosophes de bout en bout (M1-10, énoncé C8) : R_n pour n = 3, 4, 5
//! construits en DD plat, comptés par pli `(ℕ,+,×)`, filtrés / sondés par
//! `filter`/`exists`, **7 formes** vérifiées, figure SVG exportée depuis la
//! Liseuse.

use lisage_core::Anchor;
use lisage_core::operations::{ClePresentes, existe, filtrer};
use lisage_core::pli::compter;
use lisage_liseuse::philosophes::{
    cellules_legales, compter_r_n, construire_mangeurs, construire_r_n, mots_atteignables,
    pool_neuf, schema_anneau, schema_mangeurs,
};
use lisage_liseuse::svg::rendre_svg;

/// |R_n| attendus (docs/exemple-fil-rouge § 0, vérifiés par simulation).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152)];

/// C8 : |R_n| par pli DD == énumération de référence == valeurs du fil rouge.
#[test]
fn test_c8_cardinaux_r3_r4_r5() {
    for &(n, attendu) in R_ATTENDUS {
        let schema = schema_anneau(n);
        let mut entree = pool_neuf(&schema);
        let mut sortie = pool_neuf(&schema);
        let anchor = Anchor(0);
        let racine = construire_r_n(&mut entree, anchor, n);
        let compte = compter_r_n(&entree, &mut sortie, anchor, n, racine);
        let reference = mots_atteignables(n as usize).len() as u64;
        assert_eq!(compte, reference, "n={n} : pli DD == énumération");
        assert_eq!(compte, attendu, "n={n} : |R_{n}| = {attendu} (fil rouge)");
    }
}

/// `filter` sur la projection « mangeurs » de R_n : états où la cellule 0
/// mange (clé 0 présente) ; compte DD == référence.
#[test]
fn test_filter_cellule_zero_mange() {
    for &(n, _) in R_ATTENDUS {
        let schema = schema_mangeurs(n);
        let mut entree = pool_neuf(&schema);
        let mut sortie = pool_neuf(&schema);
        let anchor = Anchor(0);
        let racine = construire_mangeurs(&mut entree, anchor, n);
        let filtre = filtrer(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(|cles: &ClePresentes| cles.contains(&0)),
        )
        .unwrap();
        let mut sortie2 = pool_neuf(&schema);
        let compte = compter(&sortie, &mut sortie2, anchor, &schema, filtre).unwrap();
        // Référence : ensembles indépendants de C_n contenant 0 (la
        // projection mangeurs est many-to-one : on compte les mangeurs).
        let reference = mots_atteignables(n as usize)
            .iter()
            .map(|mot| {
                (0..n)
                    .filter(|&i| mot[i as usize] == 2)
                    .collect::<std::collections::BTreeSet<u64>>()
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|mangeurs| mangeurs.contains(&0))
            .count() as u64;
        assert_eq!(compte, reference, "n={n} : filter « cellule 0 mange »");
    }
}

/// `exists` sur R_n : une cellule qui mange existe (vrai), toutes mangent
/// (faux pour n ≥ 3, pas deux `eat` adjacents).
#[test]
fn test_exists_sur_r_n() {
    for &(n, _) in R_ATTENDUS {
        let schema = schema_anneau(n);
        let mut entree = pool_neuf(&schema);
        let mut sortie = pool_neuf(&schema);
        let anchor = Anchor(0);
        let racine = construire_r_n(&mut entree, anchor, n);
        let un_mange = existe(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(|cles: &ClePresentes| cles.contains(&0)),
        )
        .unwrap();
        assert!(
            un_mange,
            "n={n} : un état avec la cellule 0 présente existe"
        );
        let n_copie = n;
        let toutes = existe(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(move |cles: &ClePresentes| (0..n_copie).all(|i| cles.contains(&i))),
        )
        .unwrap();
        assert!(
            toutes,
            "n={n} : l'état plein (toutes clés présentes) existe"
        );
    }
}

/// C8 : les 7 formes = 5 cellules légales de l'automate minimal + 2
/// terminaux. Chaque cellule légale apparaît dans au moins un état de R_n
/// (vérifié sur l'énumération de référence), et aucune autre combinaison
/// (pc, left) n'apparaît.
#[test]
fn test_sept_formes_automate() {
    let legales = cellules_legales();
    assert_eq!(legales.len(), 5, "5 formes internes");
    for &(n, _) in R_ATTENDUS {
        let vus: std::collections::BTreeSet<(u64, u64)> = mots_atteignables(n as usize)
            .iter()
            .flat_map(|mot| {
                (0..n as usize).map(|i| {
                    (
                        mot[i],
                        lisage_liseuse::philosophes::fourchette(mot, n as usize, i),
                    )
                })
            })
            .collect();
        for cellule in legales {
            assert!(
                vus.contains(&cellule),
                "n={n} : la cellule légale {cellule:?} apparaît"
            );
        }
        assert_eq!(
            vus.len(),
            5,
            "n={n} : exactement 5 cellules légales, ni plus ni moins"
        );
    }
}

/// Figure exportée depuis la Liseuse : SVG statique de R_5, déterministe,
/// écrite dans `bench-results/` (commitée).
#[test]
fn test_figure_svg_r5_exportee() {
    let n = 5u64;
    let schema = schema_anneau(n);
    let mut entree = pool_neuf(&schema);
    let racine = construire_r_n(&mut entree, Anchor(0), n);
    let svg = rendre_svg(&entree, racine);
    assert!(svg.starts_with("<svg ") && svg.contains("ℓ0"));
    assert_eq!(svg, rendre_svg(&entree, racine), "rendu déterministe");

    let dossier = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
    std::fs::create_dir_all(dossier).expect("création de bench-results");
    let chemin = format!("{dossier}/philosophes-n5.svg");
    std::fs::write(&chemin, &svg).expect("écriture de la figure");
}
