//! End-to-end philosophers demo (M1-10, statement C8): R_n for n = 3, 4, 5
//! built in flat DD, counted by fold `(ℕ,+,×)`, filtered / probed by
//! `filter`/`exists`, **7 forms** verified, SVG figure exported from the
//! Liseuse.

use lisage_core::Anchor;
use lisage_core::fold::count;
use lisage_core::operations::{PresenceKeys, exists, filter};
use lisage_liseuse::philosophers::{
    build_eaters, build_r_n, cells_legal, count_r_n, fresh_pool, schema_eaters, schema_ring,
    words_reachable,
};
use lisage_liseuse::svg::render_svg;

/// Expected |R_n| (docs/golden-thread-example-n-philosophers-end-to-end § 0, verified by simulation).
const R_EXPECTEDS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152)];

/// C8: |R_n| by DD fold == reference enumeration == golden-thread values.
#[test]
fn test_c8_cardinals_r3_r4_r5() {
    for &(n, expected) in R_EXPECTEDS {
        let schema = schema_ring(n);
        let mut input = fresh_pool(&schema);
        let mut output = fresh_pool(&schema);
        let anchor = Anchor(0);
        let root = build_r_n(&mut input, anchor, n);
        let count = count_r_n(&input, &mut output, anchor, n, root);
        let reference = words_reachable(n as usize).len() as u64;
        assert_eq!(count, reference, "n={n} : DD fold == enumeration");
        assert_eq!(
            count, expected,
            "n={n} : |R_{n}| = {expected} (golden thread)"
        );
    }
}

/// `filter` on the « eaters » projection of R_n: states where cell 0
/// eats (key 0 present); DD count == reference.
#[test]
fn test_filter_cell_zero_eats() {
    for &(n, _) in R_EXPECTEDS {
        let schema = schema_eaters(n);
        let mut input = fresh_pool(&schema);
        let mut output = fresh_pool(&schema);
        let anchor = Anchor(0);
        let root = build_eaters(&mut input, anchor, n);
        let filter = filter(
            &input,
            &mut output,
            anchor,
            &schema,
            root,
            Box::new(|keys: &PresenceKeys| keys.contains(&0)),
        )
        .unwrap();
        let mut output2 = fresh_pool(&schema);
        let count = count(&output, &mut output2, anchor, &schema, filter).unwrap();
        // Reference: independent sets of C_n containing 0 (the
        // eaters projection is many-to-one: we count the eaters).
        let reference = words_reachable(n as usize)
            .iter()
            .map(|mot| {
                (0..n)
                    .filter(|&i| mot[i as usize] == 2)
                    .collect::<std::collections::BTreeSet<u64>>()
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|eaters| eaters.contains(&0))
            .count() as u64;
        assert_eq!(count, reference, "n={n} : filter « cell 0 eats »");
    }
}

/// `exists` on the « eaters » projection of R_n: a cell that eats
/// exists (true — key 0 is present in at least one state), but no
/// state where all the cells eat (false for n ≥ 3: no two `eat`
/// adjacent circularly). Both assertions can fail if
/// `exists` is broken (always-true or always-false).
#[test]
fn test_exists_on_r_n() {
    for &(n, _) in R_EXPECTEDS {
        let schema = schema_eaters(n);
        let mut input = fresh_pool(&schema);
        let mut output = fresh_pool(&schema);
        let anchor = Anchor(0);
        let root = build_eaters(&mut input, anchor, n);
        let one_eats = exists(
            &input,
            &mut output,
            anchor,
            &schema,
            root,
            Box::new(|keys: &PresenceKeys| keys.contains(&0)),
        )
        .unwrap();
        assert!(one_eats, "n={n} : a state where cell 0 eats exists");
        let n_copy = n;
        let all_eat = exists(
            &input,
            &mut output,
            anchor,
            &schema,
            root,
            Box::new(move |keys: &PresenceKeys| (0..n_copy).all(|i| keys.contains(&i))),
        )
        .unwrap();
        assert!(!all_eat, "n={n} : no state where all the cells eat");
    }
}

/// C8: the 7 forms = 5 legal cells of the minimal automaton + the 2
/// terminals ⊥/⊤, verified **against R_n itself** (the DD): both
/// terminals are reachable from the root, each state of the
/// reference enumeration compiles to a reachable node, and the
/// only (pc, left) combinations appearing in R_n are the 5 legal ones.
#[test]
fn test_seven_forms_against_r_n() {
    use lisage_core::{Form, Pool};

    let legal = cells_legal();
    assert_eq!(legal.len(), 5, "5 internal forms");
    for &(n, _) in R_EXPECTEDS {
        let schema = schema_ring(n);
        let mut input = fresh_pool(&schema);
        let anchor = Anchor(0);
        let root = build_r_n(&mut input, anchor, n);

        // Nodes reachable from the root + terminals seen.
        let mut reachable = std::collections::BTreeSet::new();
        let mut terminals_seen = std::collections::BTreeSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !reachable.insert(node) {
                continue;
            }
            match input.resolve(node).unwrap() {
                Form::Terminal(payload) => {
                    terminals_seen.insert(payload);
                }
                Form::Node { arcs, .. } => stack.extend(arcs.values().copied()),
            }
        }
        // ⊤ is reachable (R_n is not-empty); ⊥ is not (every path
        // of R_n leads to acceptance). Both assertions are
        // counter-examples: they fail if the construction changes.
        assert!(
            terminals_seen.contains(&true),
            "n={n} : the acceptance terminal ⊤ is in R_n"
        );
        assert!(
            !terminals_seen.contains(&false),
            "n={n} : the rejection terminal ⊥ is not reachable from R_n"
        );

        // DD cardinality == enumeration cardinality (test C8): the two
        // sets coincide. We record here the (pc, left)
        // combinations appearing in R_n.
        let mut seen: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
        for mot in words_reachable(n as usize) {
            for i in 0..n as usize {
                seen.insert((
                    mot[i],
                    lisage_liseuse::philosophers::fourchette(&mot, n as usize, i),
                ));
            }
        }
        for cell in legal {
            assert!(
                seen.contains(&cell),
                "n={n} : the legal cell {cell:?} appears"
            );
        }
        assert_eq!(seen.len(), 5, "n={n} : exactly 5 legal cells in R_n");
    }
}

/// Independent reference by DP: number of circular words of length
/// `n` over {0,1,2} with no two `2` adjacent (circular constraint included).
fn reference_circulaire(n: usize) -> u64 {
    (0..3usize)
        .flat_map(|first| {
            dp_linear(n, first)
                .into_iter()
                .filter(move |(last, _)| !(first == 2 && *last == 2))
                .map(|(_, count)| count)
                .collect::<Vec<_>>()
        })
        .sum()
}

/// Linear DP: number of words of length `n` starting with `first`,
/// with no two `2` adjacent (circular constraint not included). Returns
/// the last symbol and the count.
fn dp_linear(n: usize, first: usize) -> Vec<(usize, u64)> {
    let mut dp = [0u64; 3];
    dp[first] = 1;
    for _ in 1..n {
        let mut next = [0u64; 3];
        for (v, cell_next) in next.iter_mut().enumerate() {
            *cell_next = dp
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
/// the enumeration has the same cardinality as the reference DP.
#[test]
fn test_property_words_reachable_until_n12() {
    for n in 3..=12usize {
        let words = words_reachable(n);
        assert_eq!(
            words.len() as u64,
            reference_circulaire(n),
            "n={n} : enumeration == reference DP"
        );
        for mot in &words {
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
fn test_figure_svg_r5_exported() {
    let n = 5u64;
    let schema = schema_ring(n);
    let mut input = fresh_pool(&schema);
    let root = build_r_n(&mut input, Anchor(0), n);
    let svg = render_svg(&input, root);
    assert!(svg.starts_with("<svg ") && svg.contains("ℓ0"));
    assert_eq!(svg, render_svg(&input, root), "deterministic rendering");

    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
    std::fs::create_dir_all(directory).expect("creation of bench-results");
    let path = format!("{directory}/philosophers-n5.svg");
    // The committed figure must be up to date: a diverging rendering fails here.
    let commitee = std::fs::read_to_string(&path).expect("reading the committed figure");
    assert_eq!(
        svg, commitee,
        "bench-results/philosophers-n5.svg is up to date"
    );
    std::fs::write(&path, &svg).expect("writing the figure");
}

/// MEM-17: the degree histogram on the reference instance (R_5) confronts
/// the expected mass at 1-4 arcs with the measured one, and justifies the
/// C boundary of MEM-9 (exact classes cover the observed degrees, the
/// catch-all stays marginal).
#[test]
fn test_histogramme_degres_instance_reference_r5() {
    let n = 5u64;
    let schema = schema_ring(n);
    let mut pool = fresh_pool(&schema);
    let root = build_r_n(&mut pool, Anchor(0), n);
    let _ = count_r_n(&pool, &mut fresh_pool(&schema), Anchor(0), n, root);
    let histogram = pool.histogram();
    let internal: u64 = histogram
        .iter()
        .filter(|h| h.class >= 1)
        .map(|h| h.live)
        .sum();
    assert!(internal > 0, "R_5 has internal nodes");
    let mass_1_4: u64 = histogram
        .iter()
        .filter(|h| (1..=4).contains(&h.class))
        .map(|h| h.live)
        .sum();
    let catch_all = pool.class_map().catch_all;
    let mass_catch_all: u64 = histogram
        .iter()
        .filter(|h| h.class == catch_all)
        .map(|h| h.live)
        .sum();
    let share_1_4 = mass_1_4 as f64 / internal as f64;
    let share_catch_all = mass_catch_all as f64 / internal as f64;
    // Expected: the mass sits at 1-4 arcs on the reference instance.
    assert!(
        share_1_4 >= 0.90,
        "mass at degrees 1-4: {:.1}% (expected ≥ 90%), histogram: {:?}",
        share_1_4 * 100.0,
        histogram
    );
    // The C boundary of MEM-9 is justified: the catch-all class beyond the
    // exact window stays marginal on the reference instance.
    assert!(
        share_catch_all <= 0.10,
        "catch-all share: {:.1}% (C boundary justified if ≤ 10%), histogram: {:?}",
        share_catch_all * 100.0,
        histogram
    );
}
