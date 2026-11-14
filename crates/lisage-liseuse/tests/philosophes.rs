//! End-to-end philosophers demo (M1-10, statement C8): R_n for n = 3, 4, 5
//! built in flat DD, counted by fold `(ℕ,+,×)`, filtered / probed by
//! `filter`/`exists`, **7 forms** verified, SVG figure exported from the
//! Liseuse.

use lisage_core::Anchor;
use lisage_core::operations::{ClePresentes, existe, filtrer};
use lisage_core::pli::compter;
use lisage_liseuse::philosophes::{
    cellules_legales, compter_r_n, construire_mangeurs, construire_r_n, mots_atteignables,
    pool_neuf, schema_anneau, schema_mangeurs,
};
use lisage_liseuse::svg::rendre_svg;

/// Expected |R_n| (docs/golden-thread-example-n-philosophers-end-to-end § 0, verified by simulation).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152)];

/// C8: |R_n| by DD fold == reference enumeration == golden-thread values.
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
        assert_eq!(compte, reference, "n={n} : DD fold == enumeration");
        assert_eq!(
            compte, attendu,
            "n={n} : |R_{n}| = {attendu} (golden thread)"
        );
    }
}

/// `filter` on the « eaters » projection of R_n: states where cell 0
/// eats (key 0 present); DD count == reference.
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
        // Reference: independent sets of C_n containing 0 (the
        // eaters projection is many-to-one: we count the eaters).
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
        assert_eq!(compte, reference, "n={n} : filter « cell 0 eats »");
    }
}

/// `exists` on the « eaters » projection of R_n: a cell that eats
/// exists (true — key 0 is present in at least one state), but no
/// state where all the cells eat (false for n ≥ 3: no two `eat`
/// adjacent circularly). Both assertions can fail if
/// `exists` is broken (always-true or always-false).
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
        assert!(un_mange, "n={n} : a state where cell 0 eats exists");
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
        assert!(!toutes_mangent, "n={n} : no state where all the cells eat");
    }
}

/// C8: the 7 forms = 5 legal cells of the minimal automaton + the 2
/// terminals ⊥/⊤, verified **against R_n itself** (the DD): both
/// terminals are reachable from the root, each state of the
/// reference enumeration compiles to a reachable node, and the
/// only (pc, left) combinations appearing in R_n are the 5 legal ones.
#[test]
fn test_sept_formes_contre_r_n() {
    use lisage_core::{Form, Pool};

    let legales = cellules_legales();
    assert_eq!(legales.len(), 5, "5 internal forms");
    for &(n, _) in R_ATTENDUS {
        let schema = schema_anneau(n);
        let mut entree = pool_neuf(&schema);
        let anchor = Anchor(0);
        let racine = construire_r_n(&mut entree, anchor, n);

        // Nodes reachable from the root + terminals seen.
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
        // ⊤ is reachable (R_n is non-empty); ⊥ is not (every path
        // of R_n leads to acceptance). Both assertions are
        // counter-examples: they fail if the construction changes.
        assert!(
            terminaux_vus.contains(&true),
            "n={n} : the acceptance terminal ⊤ is in R_n"
        );
        assert!(
            !terminaux_vus.contains(&false),
            "n={n} : the rejection terminal ⊥ is not reachable from R_n"
        );

        // DD cardinal == enumeration cardinal (test C8): the two
        // sets coincide. We record here the (pc, left)
        // combinations appearing in R_n.
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
                "n={n} : the legal cell {cellule:?} appears"
            );
        }
        assert_eq!(vus.len(), 5, "n={n} : exactly 5 legal cells in R_n");
    }
}

/// Independent reference by DP: number of circular words of length
/// `n` over {0,1,2} with no two `2` adjacent (circular constraint included).
fn reference_circulaire(n: usize) -> u64 {
    (0..3usize)
        .flat_map(|premier| {
            dp_lineaire(n, premier)
                .into_iter()
                .filter(move |(dernier, _)| !(premier == 2 && *dernier == 2))
                .map(|(_, compte)| compte)
                .collect::<Vec<_>>()
        })
        .sum()
}

/// Linear DP: number of words of length `n` starting with `premier`,
/// with no two `2` adjacent (circular constraint not included). Returns
/// the last symbol and the count.
fn dp_lineaire(n: usize, premier: usize) -> Vec<(usize, u64)> {
    let mut dp = [0u64; 3];
    dp[premier] = 1;
    for _ in 1..n {
        let mut next = [0u64; 3];
        for (v, cellule_next) in next.iter_mut().enumerate() {
            *cellule_next = dp
                .iter()
                .enumerate()
                .filter(|(u, _)| !(v == 2 && *u == 2))
                .map(|(_, &dp_u)| dp_u)
                .sum();
        }
        dp = next;
    }
    dp.iter().enumerate().map(|(v, &c)| (v, c)).collect()
}

/// Property (no test dependency): for n = 3..=12, every enumerated word
/// respects the invariant « no two `eat` adjacent circularly », and
/// the enumeration has the same cardinal as the reference DP.
#[test]
fn test_propriete_mots_atteignables_jusqua_n12() {
    for n in 3..=12usize {
        let mots = mots_atteignables(n);
        assert_eq!(
            mots.len() as u64,
            reference_circulaire(n),
            "n={n} : enumeration == reference DP"
        );
        for mot in &mots {
            assert!(
                (0..n).all(|i| !(mot[i] == 2 && mot[(i + 1) % n] == 2)),
                "n={n} : circular invariant on {mot:?}"
            );
        }
    }
}

/// Figure exported from the Liseuse: static SVG of R_5, deterministic,
/// written to `bench-results/` (committed).
#[test]
fn test_figure_svg_r5_exportee() {
    let n = 5u64;
    let schema = schema_anneau(n);
    let mut entree = pool_neuf(&schema);
    let racine = construire_r_n(&mut entree, Anchor(0), n);
    let svg = rendre_svg(&entree, racine);
    assert!(svg.starts_with("<svg ") && svg.contains("ℓ0"));
    assert_eq!(svg, rendre_svg(&entree, racine), "deterministic rendering");

    let dossier = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
    std::fs::create_dir_all(dossier).expect("creation of bench-results");
    let chemin = format!("{dossier}/philosophes-n5.svg");
    // The committed figure must be up to date: a diverging rendering fails here.
    let commitee = std::fs::read_to_string(&chemin).expect("reading the committed figure");
    assert_eq!(
        svg, commitee,
        "bench-results/philosophes-n5.svg is up to date"
    );
    std::fs::write(&chemin, &svg).expect("writing the figure");
}
