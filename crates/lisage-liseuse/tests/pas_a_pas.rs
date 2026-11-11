//! Tests de la Liseuse v1 — vue pas à pas (M2-6) : sérialisation JSONL v2
//! des nouveaux événements (`Attr`, `MemoHit`/`MemoMiss`, `Round`),
//! scrubber (replay de l'état à une position), différentiel avant/après
//! avec fantômes, et rendu SVG aux canaux colorés.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_core::PolitiqueMemoire;
use lisage_core::arena::Arena;
use lisage_core::observer::{CanalAttr, Observer, TraceEvent};
use lisage_core::pli::Pli;
use lisage_core::saturate::{ParametresSaturation, PolitiqueSaturation, saturer};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::Transducteur;
use lisage_core::{Anchor, NodeId, Pool};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::pas_a_pas::{DiffTrace, Scrubber, diff, legende_canaux, rendre_svg_pas_a_pas};
use lisage_liseuse::trace::{TRACE_VERSION, serialiser_evenement};

type PoolRef = Arena<ReferenceController>;

/// Observateur qui collecte les événements typés en mémoire.
#[derive(Clone, Default)]
struct Collecteur {
    evenements: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collecteur {
    fn new() -> Self {
        Self::default()
    }
    fn prendre(&self) -> Vec<TraceEvent> {
        self.evenements.lock().expect("mutex poisoned").clone()
    }
}

impl Observer for Collecteur {
    fn on_event(&self, event: &TraceEvent) {
        self.evenements
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

fn schema_ensemble() -> Schema {
    Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]))
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

/// M2-6 : le schéma de trace est versionné v2 et sérialise les nouveaux
/// événements pas à pas.
#[test]
fn test_serialisation_v2_evenements_pas_a_pas() {
    assert_eq!(TRACE_VERSION, 2);
    let attr = serialiser_evenement(&TraceEvent::Attr {
        canal: CanalAttr::Herite,
        noeud: NodeId(3),
        valeur: "Transfert(1)".to_string(),
    });
    assert_eq!(
        attr,
        "{\"v\":2,\"type\":\"Attr\",\"canal\":\"herite\",\"noeud\":3,\"valeur\":\"Transfert(1)\"}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::MemoHit { noeud: NodeId(7) }),
        "{\"v\":2,\"type\":\"MemoHit\",\"noeud\":7}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::MemoMiss { noeud: NodeId(8) }),
        "{\"v\":2,\"type\":\"MemoMiss\",\"noeud\":8}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::Round { numero: 2 }),
        "{\"v\":2,\"type\":\"Round\",\"numero\":2}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::Attr {
            canal: CanalAttr::Memoire,
            noeud: NodeId(1),
            valeur: "true".to_string(),
        }),
        "{\"v\":2,\"type\":\"Attr\",\"canal\":\"memoire\",\"noeud\":1,\"valeur\":\"true\"}"
    );
}

/// M2-6 : le runner émet `MemoMiss`/`MemoHit` et les `Attr` des canaux
/// hérité/synthétisé pendant une transduction observée.
#[test]
fn test_runner_emet_memo_et_attr() {
    let schema = schema_ensemble();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let racine = ensemble(&mut pool, anchor, &schema, &[0, 1]);

    let collecteur = Collecteur::new();
    let mut runner = Transducteur::avec_observateur(
        Pli::<lisage_core::pli::Naturel>::new(schema.clone()),
        PolitiqueMemoire::Aucune,
        Arc::new(collecteur.clone()),
    );
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut sortie, anchor, racine).unwrap();
    // Re-run pour déclencher des MemoHit.
    let _ = runner.run(&pool, &mut sortie, anchor, racine).unwrap();

    let evenements = collecteur.prendre();
    assert!(
        evenements
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoMiss { .. })),
        "au moins un MemoMiss"
    );
    assert!(
        evenements
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoHit { .. })),
        "au moins un MemoHit au second run"
    );
    assert!(
        evenements.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Herite,
                ..
            }
        )),
        "canal hérité émis"
    );
    assert!(
        evenements.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                ..
            }
        )),
        "canal synthétisé émis"
    );
}

/// Transduction identité (cloneable) pour le test de saturate.
#[derive(Clone)]
struct Identite;

impl lisage_core::transduction::Transduction for Identite {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();
    fn fingerprint(&self) -> u64 {
        9001
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: lisage_core::Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(
        &self,
        _level: lisage_core::Level,
        _h: &(),
        _arcs: &BTreeMap<u64, (Option<lisage_core::Level>, ())>,
    ) {
    }
    fn phi_out<P: lisage_core::Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: lisage_core::transduction::ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memoire::EffetMemoire<()>) {
        (
            pool.intern(
                ctx.anchor,
                lisage_core::Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memoire::EffetMemoire::new(),
        )
    }
    fn psi<P: lisage_core::Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (
            pool.intern(anchor, lisage_core::Form::Terminal(payload)),
            (),
        )
    }
}

/// M2-6 : `saturate` émet un `Round` par itération quand un observateur est
/// fourni.
#[test]
fn test_saturate_emet_rounds() {
    let schema = schema_ensemble();
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let initial = ensemble(&mut pool, anchor, &schema, &[0]);

    let collecteur = Collecteur::new();
    let niveaux: Vec<lisage_core::Level> = (0..schema.nombre_niveaux())
        .map(lisage_core::Level)
        .collect();
    let politique = PolitiqueSaturation::Statique(niveaux);
    let mut params = ParametresSaturation {
        transduction: &Identite,
        politique: &politique,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: Some(Arc::new(collecteur.clone())),
    };
    let resultat = saturer(&mut params).unwrap();
    let rounds: Vec<u32> = collecteur
        .prendre()
        .iter()
        .filter_map(|e| match e {
            TraceEvent::Round { numero } => Some(*numero),
            _ => None,
        })
        .collect();
    assert_eq!(rounds, vec![1], "identité : 1 round (point fixe immédiat)");
    assert_eq!(resultat.iterations, 1);
}

/// M2-6 : le scrubber rejoue l'état à une position donnée (nœuds, attributs,
/// compteurs memo, rounds).
#[test]
fn test_scrubber_replay_et_diff() {
    let evenements = vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: Some(lisage_core::Level(0)),
        },
        TraceEvent::Attr {
            canal: CanalAttr::Herite,
            noeud: NodeId(1),
            valeur: "0".to_string(),
        },
        TraceEvent::MemoMiss { noeud: NodeId(1) },
        TraceEvent::Round { numero: 1 },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: Some(lisage_core::Level(1)),
        },
        TraceEvent::MemoHit { noeud: NodeId(1) },
        TraceEvent::Round { numero: 2 },
    ];
    let mut scrubber = Scrubber::new(evenements);
    assert_eq!(scrubber.longueur(), 7);

    scrubber.aller_a(3);
    let etat3 = scrubber.etat();
    assert_eq!(etat3.noeuds, BTreeSet::from([NodeId(1)]));
    assert_eq!(etat3.attributs.len(), 1);
    assert_eq!(etat3.memo_misses, 1);
    assert_eq!(etat3.memo_hits, 0);
    assert_eq!(
        etat3.rounds, 0,
        "le Round d'index 3 n'est pas encore rejoué"
    );
    scrubber.aller_a(4);
    assert_eq!(scrubber.etat().rounds, 1);

    scrubber.aller_a(100); // borné à la longueur
    assert_eq!(scrubber.position(), 7);
    let etat7 = scrubber.etat();
    assert_eq!(etat7.noeuds, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert_eq!(etat7.memo_hits, 1);
    assert_eq!(etat7.rounds, 2);

    // Différentiel avant/après depuis la fin : le nœud 2 est un fantôme
    // à la position 3, pas d'ajout.
    let diff = scrubber.diff_vers(3);
    assert!(diff.ajoutes.is_empty());
    assert_eq!(diff.fantomes, BTreeSet::from([NodeId(2)]));
    // Diff inverse : le nœud 2 est un fantôme.
    let mut retour = Scrubber::new(vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: None,
        },
    ]);
    retour.aller_a(2);
    let diff_inv = retour.diff_vers(1);
    assert!(diff_inv.ajoutes.is_empty());
    assert_eq!(diff_inv.fantomes, BTreeSet::from([NodeId(2)]));
}

/// M2-6 : le rendu SVG pas à pas colore les canaux, marque les fantômes en
/// pointillés, trace la frise avec les rounds, et est déterministe.
#[test]
fn test_rendu_svg_pas_a_pas_canaux_et_fantomes() {
    let schema = schema_ensemble();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let racine = ensemble(&mut pool, anchor, &schema, &[0, 1]);

    let collecteur = Collecteur::new();
    let mut runner = Transducteur::avec_observateur(
        Pli::<lisage_core::pli::Naturel>::new(schema.clone()),
        PolitiqueMemoire::Aucune,
        Arc::new(collecteur.clone()),
    );
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut sortie, anchor, racine).unwrap();
    let mut evenements = collecteur.prendre();
    evenements.insert(0, TraceEvent::Round { numero: 1 });
    // Le runner n'observe pas le pool : on ajoute les NodeCreated du
    // diagramme d'entrée pour que l'état ait des nœuds.
    let mut pile = vec![racine];
    let mut deja_vu = BTreeSet::new();
    while let Some(n) = pile.pop() {
        if !deja_vu.insert(n) {
            continue;
        }
        evenements.push(TraceEvent::NodeCreated {
            node: n,
            level: None,
        });
        if let Ok(lisage_core::Form::Node { arcs, .. }) = pool.resolve(n) {
            for cible in arcs.values() {
                pile.push(*cible);
            }
        }
    }

    let mut scrubber = Scrubber::new(evenements);
    scrubber.aller_a(scrubber.longueur());
    let diff_vide = DiffTrace::default();
    let svg = rendre_svg_pas_a_pas(&pool, racine, &scrubber, &diff_vide);
    // Canaux colorés présents (hérité bleu, synthétisé orange).
    let legende = legende_canaux();
    assert!(svg.contains(legende["herite"]), "canal hérité coloré");
    assert!(
        svg.contains(legende["synthetise"]),
        "canal synthétisé coloré"
    );
    // Frise : barre de round en violet + position courante.
    assert!(svg.contains(legende["memoire"]), "barre de round / mémoire");
    assert!(svg.contains("pas à pas"), "tableau de bord en titre");

    // Fantôme : diff avec un état vide au début → tous les nœuds fantômes
    // ne sont pas dans le diff (ajouts), testons l'inverse : diff vers 0.
    let diff_fantomes = scrubber.diff_vers(0);
    assert!(
        !diff_fantomes.fantomes.is_empty(),
        "nœuds présents = fantômes si on revient à 0"
    );
    let svg_fantomes = rendre_svg_pas_a_pas(&pool, racine, &scrubber, &diff_fantomes);
    assert!(
        svg_fantomes.contains("stroke-dasharray=\"4 3\""),
        "fantômes en pointillés"
    );

    // Déterminisme.
    let svg2 = rendre_svg_pas_a_pas(&pool, racine, &scrubber, &diff_vide);
    assert_eq!(svg, svg2, "rendu déterministe");
}

/// Propriété du scrubber (générateur LCG déterministe, sans proptest) :
/// le replay est monotone par préfixe (l'état à k est inclus dans l'état à
/// k+1 pour les nœuds et les compteurs) et le diff est antisymétrique
/// (`diff(a→b).ajoutes == diff(b→a).fantomes` et inversement).
#[test]
fn test_scrubber_proprietes_prefixe_monotone_et_diff_antisymetrique() {
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
        let longueur = 1 + (suivant() % 40) as usize;
        let evenements: Vec<TraceEvent> = (0..longueur)
            .map(|_| {
                let noeud = NodeId(1 + (suivant() % 6) as u32);
                match suivant() % 5 {
                    0 => TraceEvent::NodeCreated {
                        node: noeud,
                        level: None,
                    },
                    1 => TraceEvent::Attr {
                        canal: CanalAttr::Herite,
                        noeud,
                        valeur: "x".to_string(),
                    },
                    2 => TraceEvent::MemoHit { noeud },
                    3 => TraceEvent::MemoMiss { noeud },
                    _ => TraceEvent::Round {
                        numero: 1 + (suivant() % 3) as u32,
                    },
                }
            })
            .collect();
        let mut scrubber = Scrubber::new(evenements);
        let mut precedent = scrubber.etat();
        for position in 1..=scrubber.longueur() {
            scrubber.aller_a(position);
            let etat = scrubber.etat();
            assert!(
                precedent.noeuds.iter().all(|n| etat.noeuds.contains(n)),
                "monotonie des nœuds à la position {position}"
            );
            assert!(precedent.memo_hits <= etat.memo_hits);
            assert!(precedent.memo_misses <= etat.memo_misses);
            assert!(precedent.rounds <= etat.rounds);
            assert!(precedent.attributs.len() <= etat.attributs.len());
            // Antisymétrie du diff entre positions k-1 et k.
            let diff_avant = diff(&precedent, &etat);
            let diff_arriere = diff(&etat, &precedent);
            assert_eq!(diff_avant.ajoutes, diff_arriere.fantomes);
            assert_eq!(diff_avant.fantomes, diff_arriere.ajoutes);
            precedent = etat;
        }
    }
}
