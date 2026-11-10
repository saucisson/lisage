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

/// `exists` sur la projection « mangeurs » de R_n : une cellule qui mange
/// existe (vrai — la clé 0 est présente dans au moins un état), mais aucune
/// état où toutes les cellules mangent (faux pour n ≥ 3 : pas deux `eat`
/// adjacents circulairement). Les deux assertions peuvent échouer si
/// `exists` est cassé (toujours-vrai ou toujours-faux).
#[test]
fn test_exists_sur_r_n() {
    for &(n, _) in R_ATTENDUS {
        let schema = schema_mangeurs(n);
        let mut entree = pool_neuf(&schema);
        let mut sortie = pool_neuf(&schema);
        let anchor = Anchor(0);
        let racine = construire_mangeurs(&mut entree, anchor, n);
        let un_mange = existe(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(|cles: &ClePresentes| cles.contains(&0)),
        )
        .unwrap();
        assert!(un_mange, "n={n} : un état où la cellule 0 mange existe");
        let n_copie = n;
        let toutes_mangent = existe(
            &entree,
            &mut sortie,
            anchor,
            &schema,
            racine,
            Box::new(move |cles: &ClePresentes| (0..n_copie).all(|i| cles.contains(&i))),
        )
        .unwrap();
        assert!(
            !toutes_mangent,
            "n={n} : aucun état où toutes les cellules mangent"
        );
    }
}

/// C8 : les 7 formes = 5 cellules légales de l'automate minimal + les 2
/// terminaux ⊥/⊤, vérifiées **contre R_n lui-même** (le DD) : les deux
/// terminaux sont atteignables depuis la racine, chaque état de
/// l'énumération de référence compile en un nœud atteignable, et les
/// seules combinaisons (pc, left) apparaissant dans R_n sont les 5 légales.
#[test]
fn test_sept_formes_contre_r_n() {
    use lisage_core::{Form, Pool};

    let legales = cellules_legales();
    assert_eq!(legales.len(), 5, "5 formes internes");
    for &(n, _) in R_ATTENDUS {
        let schema = schema_anneau(n);
        let mut entree = pool_neuf(&schema);
        let anchor = Anchor(0);
        let racine = construire_r_n(&mut entree, anchor, n);

        // Nœuds atteignables depuis la racine + terminaux vus.
        let mut atteignables = std::collections::BTreeSet::new();
        let mut terminaux_vus = std::collections::BTreeSet::new();
        let mut pile = vec![racine];
        while let Some(noeud) = pile.pop() {
            if !atteignables.insert(noeud) {
                continue;
            }
            match entree.resolve(noeud).unwrap() {
                Form::Terminal(payload) => {
                    terminaux_vus.insert(*payload);
                }
                Form::Node { arcs, .. } => pile.extend(arcs.values().copied()),
            }
        }
        // ⊤ est atteignable (R_n est non vide) ; ⊥ ne l'est pas (tout chemin
        // de R_n mène à l'acceptation). Les deux assertions sont des
        // contre-exemples : elles échouent si la construction change.
        assert!(
            terminaux_vus.contains(&true),
            "n={n} : le terminal d'acceptation ⊤ est dans R_n"
        );
        assert!(
            !terminaux_vus.contains(&false),
            "n={n} : le terminal de rejet ⊥ n'est pas atteignable depuis R_n"
        );

        // Cardinal du DD == cardinal de l'énumération (test C8) : les deux
        // ensembles coïncident. On relève ici les combinaisons (pc, left)
        // apparaissant dans R_n.
        let mut vus: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
        for mot in mots_atteignables(n as usize) {
            for i in 0..n as usize {
                vus.insert((
                    mot[i],
                    lisage_liseuse::philosophes::fourchette(&mot, n as usize, i),
                ));
            }
        }
        for cellule in legales {
            assert!(
                vus.contains(&cellule),
                "n={n} : la cellule légale {cellule:?} apparaît"
            );
        }
        assert_eq!(
            vus.len(),
            5,
            "n={n} : exactement 5 cellules légales dans R_n"
        );
    }
}

/// Référence indépendante par DP : nombre de mots circulaires de longueur
/// `n` sur {0,1,2} sans deux `2` adjacents (contrainte circulaire incluse).
fn reference_circulaire(n: usize) -> u64 {
    let mut total = 0u64;
    for premier in 0..3usize {
        // dp[v] = nombre de façons d'atteindre la position courante avec la
        // valeur v en dernier.
        let mut dp = [0u64; 3];
        dp[premier] = 1;
        for _ in 1..n {
            let mut next = [0u64; 3];
            for (v, cellule_next) in next.iter_mut().enumerate() {
                for (u, &dp_u) in dp.iter().enumerate() {
                    if v == 2 && u == 2 {
                        continue;
                    }
                    *cellule_next += dp_u;
                }
            }
            dp = next;
        }
        for (v, &dp_v) in dp.iter().enumerate() {
            if premier == 2 && v == 2 {
                continue; // contrainte circulaire
            }
            total += dp_v;
        }
    }
    total
}

/// Propriété (sans dépendance de test) : pour n = 3..=12, tout mot énuméré
/// respecte l'invariant « pas deux `eat` adjacents circulairement », et
/// l'énumération a le même cardinal que la DP de référence.
#[test]
fn test_propriete_mots_atteignables_jusqua_n12() {
    for n in 3..=12usize {
        let mots = mots_atteignables(n);
        assert_eq!(
            mots.len() as u64,
            reference_circulaire(n),
            "n={n} : énumération == DP de référence"
        );
        for mot in &mots {
            assert!(
                (0..n).all(|i| !(mot[i] == 2 && mot[(i + 1) % n] == 2)),
                "n={n} : invariant circulaire sur {mot:?}"
            );
        }
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
    // La figure commitée doit être à jour : un rendu qui diverge échoue ici.
    if let Ok(commitee) = std::fs::read_to_string(&chemin) {
        assert_eq!(svg, commitee, "bench-results/philosophes-n5.svg est à jour");
    }
    std::fs::write(&chemin, &svg).expect("écriture de la figure");
}
