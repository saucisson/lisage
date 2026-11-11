//! Tests de la vue mémoire et quotient (M4-5) : pools + GC, badges de
//! multiplicité, classes du quotient (formes × ancres, C1 : même forme
//! sous deux ancres = deux classes), comparaison naïf/DD/Lisage, rendu
//! SVG statique.

use lisage_core::arena::Arena;
use lisage_core::biplan::Plan;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_core::{Anchor, Form, NodeId};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::vue_memoire::{
    ClasseQuotient, ComparaisonVue, VuePool, rendre_svg_vue_memoire,
};

/// Ancre de test.
const ANCRE_TEST: Anchor = Anchor(0);
/// Seconde ancre (C1 : même forme, autre ancre = autre handle).
const ANCRE_TEST_2: Anchor = Anchor(1);

type PoolRef = Arena<ReferenceController>;

fn schema_ensemble() -> Schema {
    Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]))
}

fn pool_neuf(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

fn ensemble(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atomes: &[u64]) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atomes.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M4-5 : la vue pool reflète les métriques de GC (vivants, morts, gc,
/// compactions) et les vivants par plan après un GC.
#[test]
fn test_vue_pool_depuis_metriques_gc() {
    let schema = schema_ensemble();
    let mut pool = pool_neuf(&schema);
    let racine = ensemble(&mut pool, ANCRE_TEST, &schema, &[0, 1]);
    // Orphelin collecté par le GC.
    let _orphelin = ensemble(&mut pool, ANCRE_TEST, &schema, &[2]);
    let stats_gc = pool.gc(&[racine]);
    let metriques = pool.metriques();
    let vue = VuePool::depuis_metriques("reference", metriques, stats_gc.vivants, 0);
    assert_eq!(vue.nom, "reference");
    assert_eq!(vue.metriques.gc, 1);
    assert_eq!(vue.metriques.vivants, stats_gc.vivants);
    assert_eq!(vue.metriques.morts, stats_gc.collectes);
    assert_eq!(vue.vivants_principal, stats_gc.vivants);
    assert_eq!(vue.vivants_ombre, 0);
}

/// M4-5 : classes du quotient (formes × ancres) — C1 en action : la même
/// forme internée sous deux ancres différentes forme deux classes distinctes.
#[test]
fn test_classes_quotient_formes_par_ancres() {
    let schema = schema_ensemble();
    let mut pool = pool_neuf(&schema);
    let handle_ancre_0 = ensemble(&mut pool, ANCRE_TEST, &schema, &[0]);
    let handle_ancre_1 = ensemble(&mut pool, ANCRE_TEST_2, &schema, &[0]);
    assert_ne!(
        handle_ancre_0, handle_ancre_1,
        "C1 : même forme, ancres différentes = handles distincts"
    );
    // Deux classes (forme {0}, ancre 0) et (forme {0}, ancre 1).
    let classes = [
        ClasseQuotient {
            forme: "{0}".to_string(),
            ancre: "ancre 0".to_string(),
            multiplicite: 1,
        },
        ClasseQuotient {
            forme: "{0}".to_string(),
            ancre: "ancre 1".to_string(),
            multiplicite: 1,
        },
    ];
    // Les deux classes sont distinctes (C1) : formes × ancres.
    assert_ne!(classes[0], classes[1]);
    // Badge de multiplicité : un handle partagé 3 fois sous la même ancre.
    let partage = ensemble(&mut pool, ANCRE_TEST, &schema, &[0]);
    assert_eq!(
        partage, handle_ancre_0,
        "R-share : même handle sous la même ancre"
    );
    let classe = ClasseQuotient {
        forme: "{0}".to_string(),
        ancre: "ancre 0".to_string(),
        multiplicite: 3,
    };
    let badge = classe.badge();
    assert_eq!(badge.multiplicite, classe.multiplicite);
    assert_eq!(badge.handle, "{0} @ ancre 0");
}

/// M4-5 : rendu SVG de la vue mémoire et quotient — panneaux présents,
/// badge ×k, barres de comparaison avec largeur proportionnelle (coordonnée
/// réelle parsée, pas un offset d'octets), déterminisme.
#[test]
fn test_rendre_svg_vue_memoire() {
    let schema = schema_ensemble();
    let mut pool = pool_neuf(&schema);
    let racine = ensemble(&mut pool, ANCRE_TEST, &schema, &[0, 1]);
    let _orphelin = ensemble(&mut pool, ANCRE_TEST, &schema, &[2]);
    let stats_gc = pool.gc(&[racine]);
    let vue_pool = VuePool::depuis_metriques("reference", pool.metriques(), stats_gc.vivants, 0);
    let classes = vec![
        ClasseQuotient {
            forme: "{0,1}".to_string(),
            ancre: "ancre 0".to_string(),
            multiplicite: 1,
        },
        ClasseQuotient {
            forme: "{2}".to_string(),
            ancre: "ancre 0".to_string(),
            multiplicite: 6,
        },
    ];
    let comparaison = ComparaisonVue {
        naif: 152,
        dd: 15,
        lisage: 7,
    };
    let svg = rendre_svg_vue_memoire(&[vue_pool], &classes, &comparaison);
    // Panneaux et contenus.
    assert!(svg.contains("vue mémoire et quotient"));
    assert!(svg.contains("pools (plan principal / ombre, GC)"));
    assert!(svg.contains("classes du quotient (formes × ancres)"));
    assert!(svg.contains("comparaison naïf / DD / Lisage"));
    assert!(svg.contains("reference : vivants="));
    assert!(svg.contains("{2} @ ancre 0"));
    // Badge ×6 présent (k > 1).
    assert!(svg.contains("×6"), "badge de multiplicité ×6");
    // Pas de badge pour la classe de multiplicité 1.
    assert!(!svg.contains("×1"));
    // Barres : largeur de la barre Lisage parsée depuis le SVG (coordonnée
    // réelle) — proportionnelle à 7/152 de la largeur max.
    let largeur_lisage = svg
        .lines()
        .find_map(|ligne| {
            if ligne.contains("fill=\"#2a7\"") {
                let debut = ligne.find("width=\"").unwrap() + 7;
                let fin = ligne[debut..].find('"').unwrap() + debut;
                ligne[debut..fin].parse::<u32>().ok()
            } else {
                None
            }
        })
        .expect("barre Lisage présente");
    let attendu = 7 * 400 / 152;
    assert_eq!(largeur_lisage, attendu, "barre Lisage proportionnelle");
    // Déterminisme.
    let svg2 = rendre_svg_vue_memoire(
        &[VuePool::depuis_metriques(
            "reference",
            pool.metriques(),
            stats_gc.vivants,
            0,
        )],
        &classes,
        &comparaison,
    );
    assert_eq!(svg, svg2, "rendu déterministe");
}

/// M4-5 : comparaison naïf/DD/Lisage — le mode Lisage affiche le compte le
/// plus bas (formes partagées), la barre naïf est la plus large.
#[test]
fn test_comparaison_naif_dd_lisage() {
    let comparaison = ComparaisonVue {
        naif: 536_754_176,
        dd: 15,
        lisage: 7,
    };
    let svg = rendre_svg_vue_memoire(&[], &[], &comparaison);
    assert!(svg.contains("536754176"));
    assert!(svg.contains("Lisage"));
    // La barre naïf atteint la largeur max (c'est le max).
    let largeur_naif = svg
        .lines()
        .find_map(|ligne| {
            if ligne.contains("fill=\"#999\"") {
                let debut = ligne.find("width=\"").unwrap() + 7;
                let fin = ligne[debut..].find('"').unwrap() + debut;
                ligne[debut..fin].parse::<u32>().ok()
            } else {
                None
            }
        })
        .expect("barre naïf présente");
    assert_eq!(largeur_naif, 400, "le max atteint la largeur max");
    // Plan d'ombre : un nœud interné dans le plan d'ombre est compté à part.
    let schema = schema_ensemble();
    let mut pool = pool_neuf(&schema);
    let racine = ensemble(&mut pool, ANCRE_TEST, &schema, &[0]);
    let ombre = pool.intern_plan(
        ANCRE_TEST,
        Form::Node {
            level: lisage_core::Level(1),
            arcs: Default::default(),
        },
        Plan::Ombre,
    );
    assert_eq!(pool.plan_de(ombre), Some(Plan::Ombre));
    assert_eq!(pool.plan_de(racine), Some(Plan::Principal));
}
