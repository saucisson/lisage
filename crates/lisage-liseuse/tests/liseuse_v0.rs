//! Tests de la Liseuse v0 (M1-9) : sérialiseur JSONL versionné et rendu SVG
//! statique du diagramme (niveaux = clés).

use std::collections::BTreeSet;

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::svg::rendre_svg;
use lisage_liseuse::trace::{JsonlObserver, TRACE_VERSION, serialiser_evenement};

type PoolRef = Arena<ReferenceController>;

fn schema_ensemble() -> Schema {
    Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]))
}

fn ensemble(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    atomes: &[u64],
) -> lisage_core::NodeId {
    let valeur = Value::Set(
        atomes
            .iter()
            .map(|a| Value::Atom(Atom(*a)))
            .collect::<BTreeSet<_>>(),
    );
    compiler(pool, anchor, schema, &valeur).unwrap()
}

/// Le JSONL est versionné, une ligne = un événement, et la trace est
/// déterministe (deux constructions identiques ⇒ même trace).
#[test]
fn test_trace_jsonl_versionnee_et_deterministe() {
    let schema = schema_ensemble();
    let construire = |observateur: &JsonlObserver| {
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(observateur.clone()),
        );
        ensemble(&mut pool, Anchor(0), &schema, &[0])
    };
    let observateur_a = JsonlObserver::new();
    let observateur_b = JsonlObserver::new();
    let racine_a = construire(&observateur_a);
    let racine_b = construire(&observateur_b);
    assert_eq!(racine_a, racine_b, "C1 : même construction ⇒ même racine");
    assert_eq!(
        observateur_a.lignes(),
        observateur_b.lignes(),
        "trace déterministe (rejouable à l'identique)"
    );

    let lignes = observateur_a.lignes();
    assert!(!lignes.is_empty());
    for ligne in &lignes {
        assert!(
            ligne.starts_with(&format!("{{\"v\":{TRACE_VERSION},")),
            "chaque ligne porte la version du schéma : {ligne}"
        );
        assert!(ligne.ends_with('}'));
    }
    // Le terminal d'acceptation est créé en premier.
    assert!(
        lignes[0].contains("\"type\":\"NodeCreated\"") && lignes[0].contains("\"level\":null"),
        "terminal = niveau null : {}",
        lignes[0]
    );
    // Le nœud de niveau 0 existe dans la trace.
    assert!(
        lignes.iter().any(|l| l.contains("\"level\":0")),
        "un nœud de niveau 0 a été créé"
    );
}

/// Sérialisation exacte (golden) d'un événement de chaque type.
#[test]
fn test_serialiser_evenement_golden() {
    use lisage_core::observer::{ReductionRule, TraceEvent};
    use lisage_core::{ControllerId, Handle, Level, NodeId};

    assert_eq!(
        serialiser_evenement(&TraceEvent::NodeCreated {
            node: NodeId(3),
            level: Some(Level(2))
        }),
        "{\"v\":1,\"type\":\"NodeCreated\",\"node\":3,\"level\":2}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::NodeShared { node: NodeId(7) }),
        "{\"v\":1,\"type\":\"NodeShared\",\"node\":7}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level: Level(1)
        }),
        "{\"v\":1,\"type\":\"Reduction\",\"rule\":\"R-collapse\",\"level\":1}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::DiagramBuilt {
            handle: Handle {
                controller: ControllerId(1),
                anchor: Anchor(0),
                node: NodeId(4)
            },
            nodes: 5
        }),
        "{\"v\":1,\"type\":\"DiagramBuilt\",\"controller\":1,\"anchor\":0,\"node\":4,\"nodes\":5}"
    );
}

/// Le rendu SVG v0 est déterministe et montre niveaux, nœuds et terminaux.
#[test]
fn test_svg_statique_deterministe() {
    let schema = schema_ensemble();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let racine = ensemble(&mut pool, anchor, &schema, &[0, 1]);

    let svg = rendre_svg(&pool, racine);
    let svg2 = rendre_svg(&pool, racine);
    assert_eq!(svg, svg2, "rendu déterministe (rejouable à l'identique)");
    assert!(svg.starts_with("<svg "), "document SVG");
    assert!(svg.trim_end().ends_with("</svg>"));
    // Niveaux = clés : étiquette ℓ0 pour le niveau 0.
    assert!(svg.contains("ℓ0"), "le niveau 0 est étiqueté");
    // Terminal d'acceptation ⊤ rendu (⊥ est éliminé par R-default sur cet
    // ensemble : l'arc « atome absent » vers ⊥ est implicite).
    assert!(svg.contains('⊤'), "terminal d'acceptation rendu");
    // Un arc au moins.
    assert!(svg.contains("<line "), "au moins un arc rendu");
}

/// Le SVG échappe le XML (pas de `<` brut hors balises) — test via un payload
/// booléen standard : on vérifie surtout la structure équilibrée.
#[test]
fn test_svg_structure_equilibree() {
    let schema = schema_ensemble();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let racine = ensemble(&mut pool, Anchor(0), &schema, &[1]);
    let svg = rendre_svg(&pool, racine);
    let ouvertures = svg.matches("<rect ").count();
    let fermetures_rect = svg.matches("/>").count();
    assert!(ouvertures >= 2, "nœud + terminal rendus");
    assert!(
        fermetures_rect >= ouvertures,
        "balises auto-fermantes présentes"
    );
}
