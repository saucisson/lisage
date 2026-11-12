//! Tests du harnais de confrontation (ADR-0102) : valeurs de vérification
//! épinglées (LCG, fil rouge R_n, fenêtre glissante), erreurs typées,
//! sérialisation JSON déterministe. Les mesures temps/mémoire ne sont pas
//! épinglées (elles dépendent de la machine) : seules les valeurs
//! *sémantiques* le sont — c'est la falsification explicite du harnais.

use std::path::PathBuf;

use lisage_bench::confrontation::{
    Charge, ErreurConfrontation, MesureConfrontation, OUTIL, Variante, executer, serialiser_ligne,
};
use lisage_core::atelier::Encodage;

/// `think`.
const PENSE: u64 = 0;
/// `eat`.
const MANGE: u64 = 2;
/// Fork tenue par le philosophe courant (il mange).
const TENUE: u64 = 1;
/// Fork tenue par le philosophe précédent (il mange).
const PRECEDENTE: u64 = 2;
/// |R_n| du fil rouge pour n = 3..7 (vérifiés par simulation, M4-4).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Nœuds partagés de R_3 (atelier didactique, fragment 1 : 33 nœuds).
const NOEUDS_R3: u64 = 33;
/// Chemins de la fenêtre de test : 10 bases × 4 positions.
const CHEMINS_FENETRE: u64 = 40;
/// Nœuds de la fenêtre absolue (atelier didactique, fragment 2).
const NOEUDS_FENETRE_ABSOLUE: u64 = 12;
/// Nœuds de la fenêtre relative : une seule forme partagée, O(1).
const NOEUDS_FENETRE_RELATIVE: u64 = 3;
/// Satisfactions de la table LCG pour n = 4 (épinglé, déterministe).
const SATISFACTIONS_LCG_N4: u64 = 7;
/// Satisfactions de la table LCG pour n = 12 (épinglé, déterministe —
/// même LCG que la ligne de base M1-8).
const SATISFACTIONS_LCG_N12: u64 = 2074;

/// Fourchette `left_i` dérivée du mot `pc` (fil rouge § 3).
fn fourchette(compteur_programme: &[u64], nombre: usize, indice: usize) -> u64 {
    if compteur_programme[indice] == MANGE {
        TENUE
    } else if compteur_programme[(indice + nombre - 1) % nombre] == MANGE {
        PRECEDENTE
    } else {
        0
    }
}

/// Énumère les états étendus de R_n : mots circulaires sans voisins `mange`
/// adjacents, forks dérivées, intercalés `pc_0 left_0 pc_1 …`. Même
/// algorithme que l'orchestrateur Python (`scripts/confrontation.py`) : la
/// cohérence génération ↔ pins est vérifiée par test dédié.
fn etats_etendus(nombre: usize) -> Vec<Vec<u64>> {
    let mut mots = Vec::new();
    let mut mot = vec![PENSE; nombre];
    loop {
        let circulaire_ok = (0..nombre)
            .all(|indice| !(mot[indice] == MANGE && mot[(indice + 1) % nombre] == MANGE));
        if circulaire_ok {
            mots.push(mot.clone());
        }
        let mut indice = nombre;
        loop {
            if indice == 0 {
                return mots
                    .into_iter()
                    .map(|mot| {
                        (0..nombre)
                            .flat_map(|i| [mot[i], fourchette(&mot, nombre, i)])
                            .collect()
                    })
                    .collect();
            }
            indice -= 1;
            if mot[indice] < MANGE {
                mot[indice] += 1;
                break;
            }
            mot[indice] = PENSE;
        }
    }
}

/// Écrit le fichier d'états étendus partagé (même format que l'orchestrateur
/// Python : première ligne `n <n>`, puis un état par ligne) et rend son
/// chemin.
fn ecrire_fichier_etats(nombre: u64) -> PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "confrontation-test-philosophes-{nombre}-{}.txt",
        std::process::id()
    ));
    let etats = etats_etendus(nombre as usize);
    let mut contenu = format!("n {nombre}\n");
    for etat in etats {
        let ligne: Vec<String> = etat.iter().map(|valeur| valeur.to_string()).collect();
        contenu.push_str(&ligne.join(" "));
        contenu.push('\n');
    }
    std::fs::write(&chemin, contenu).expect("fichier d'états temporaire écrit");
    chemin
}

/// Charge `alea-lcg` : satisfactions épinglées (LCG déterministe), C6
/// vérifié en interne par le runner (mini-BDD indépendant), mesure rendue
/// avec la variante `bdd`.
#[test]
fn test_alea_lcg_satisfactions_epinglees() {
    for (n, satisfait) in [(4, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let mesure = executer(&Charge::AleaLcg, &[n.to_string()], None).expect("charge exécutée");
        assert_eq!(OUTIL, "lisage");
        assert_eq!(mesure.variante, Variante::Bdd);
        assert_eq!(mesure.charge, Charge::AleaLcg);
        assert_eq!(mesure.verifie, satisfait, "n={n} : satisfactions épinglées");
        assert!(mesure.noeuds > 0, "n={n} : diagramme non vide");
        assert!(mesure.temps_ms.is_finite(), "n={n} : temps mesuré");
        assert!(mesure.rss_max_ko > 0, "n={n} : VmHWM relevée");
    }
}

/// Charge `alea-lcg` : bornes refusées (zéro et trop grand), erreur typée.
#[test]
fn test_alea_lcg_bornes_refusees() {
    for n in [0, 25] {
        let erreur =
            executer(&Charge::AleaLcg, &[n.to_string()], None).expect_err("bornes refusées");
        assert!(matches!(
            erreur,
            ErreurConfrontation::ParametresInvalides { charge, .. } if charge == "alea-lcg"
        ));
    }
}

/// Charge `philosophes` : |R_n| épinglé sur n = 3..7, nœuds de R_3 épinglés
/// (33), cardinal par pli == nombre d'états du fichier.
#[test]
fn test_philosophes_fil_rouge_epingle() {
    for &(nombre, attendu) in R_ATTENDUS {
        let chemin = ecrire_fichier_etats(nombre);
        let parametres = vec![nombre.to_string(), chemin.display().to_string()];
        let mesure = executer(&Charge::Philosophes, &parametres, None).expect("charge exécutée");
        assert_eq!(mesure.verifie, attendu, "n={nombre} : |R_n| épinglé");
        assert_eq!(mesure.variante, Variante::Mdd);
        if nombre == 3 {
            assert_eq!(mesure.noeuds, NOEUDS_R3, "R_3 : 33 nœuds partagés");
        }
        assert!(mesure.rss_max_ko > 0);
    }
}

/// Charge `philosophes` : fichier invalide (vide, première ligne non
/// conforme, longueur d'état incohérente, valeur hors domaine) → erreur
/// typée, jamais avalée.
#[test]
fn test_philosophes_entrees_invalides() {
    let cas = [
        "",
        "x 3\n",
        "n 3\n0 0\n",
        "n 2\n0 0 0 0 0 0\n",
        "n 3\n0 0 0 0 0 7\n",
    ];
    for (numero, contenu) in cas.iter().enumerate() {
        let chemin = std::env::temp_dir().join(format!(
            "confrontation-test-invalide-{numero}-{}.txt",
            std::process::id()
        ));
        std::fs::write(&chemin, contenu).expect("fichier invalide écrit");
        let parametres = vec!["3".to_string(), chemin.display().to_string()];
        let erreur = executer(&Charge::Philosophes, &parametres, None);
        assert!(erreur.is_err(), "entrée invalide rejetée : {contenu:?}");
        assert!(
            matches!(erreur, Err(ErreurConfrontation::EntreeInvalide { .. })),
            "erreur typée EntreeInvalide, pas d'avalage : {erreur:?}"
        );
    }
}

/// Charge `fenetre` : mêmes chemins dans les deux encodages (40 = 10×4),
/// nœuds 12 (absolu) contre 3 (relatif) — le partage relatif se voit à
/// l'œil nu (atelier didactique, fragment 2).
#[test]
fn test_fenetre_encodages_epingles() {
    let absolu =
        executer(&Charge::Fenetre, &["4".to_string(), "10".to_string()], None).expect("absolu");
    let relatif = executer(
        &Charge::Fenetre,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        None,
    )
    .expect("relatif");
    assert_eq!(absolu.verifie, CHEMINS_FENETRE);
    assert_eq!(relatif.verifie, CHEMINS_FENETRE);
    assert_eq!(absolu.noeuds, NOEUDS_FENETRE_ABSOLUE);
    assert_eq!(relatif.noeuds, NOEUDS_FENETRE_RELATIVE);
    assert_eq!(absolu.encodage, Some(Encodage::Absolu));
    assert_eq!(relatif.encodage, Some(Encodage::Relatif));
}

/// Charge inconnue (bord du binaire : analyse en enum, `None`) et
/// paramètres manquants : erreurs typées.
#[test]
fn test_charges_inconnues_refusees() {
    assert_eq!(Charge::depuis_nom("inexistante"), None);
    assert_eq!(Charge::depuis_nom("alea-lcg"), Some(Charge::AleaLcg));
    let erreur = executer(&Charge::AleaLcg, &[], None).expect_err("paramètres manquants");
    assert!(matches!(
        erreur,
        ErreurConfrontation::ParametresInvalides { charge, .. } if charge == "alea-lcg"
    ));
}

/// Sérialisation JSON : une ligne, ordre des clés fixe, encodage présent
/// seulement pour la fenêtre. Épinglé caractère par caractère.
#[test]
fn test_serialisation_ligne_epinglee() {
    let mesure = MesureConfrontation {
        variante: Variante::Mdd,
        charge: Charge::Fenetre,
        parametres: vec![
            ("taille".to_string(), "4".to_string()),
            ("nb_bases".to_string(), "10".to_string()),
        ],
        encodage: Some(Encodage::Relatif),
        temps_ms: 1.25,
        rss_max_ko: 4096,
        noeuds: 3,
        verifie: 40,
    };
    let attendu = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"fenetre\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\"encodage\":\"relatif\",\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialiser_ligne(&mesure), attendu);
    let sans_encodage = MesureConfrontation {
        encodage: None,
        ..mesure
    };
    let attendu_sans = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"fenetre\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialiser_ligne(&sans_encodage), attendu_sans);
}

/// La génération d'états étendus du harnais (même algorithme que
/// l'orchestrateur Python) couvre les pins du fil rouge : |R_n| pour
/// n = 3..7, chaque état à 2n valeurs dans {0,1,2}.
#[test]
fn test_etats_etendus_coherents_avec_fil_rouge() {
    for &(nombre, attendu) in R_ATTENDUS {
        let etats = etats_etendus(nombre as usize);
        assert_eq!(etats.len() as u64, attendu, "n={nombre} : |R_n| épinglé");
        assert!(etats.iter().all(|etat| {
            etat.len() == 2 * nombre as usize && etat.iter().all(|valeur| *valeur <= MANGE)
        }));
    }
}

/// Narration (ADR-0100 décision 6) : chaque charge émet un événement
/// `Attr`/`Synthétise` portant le récapitulatif — épinglé par un
/// observateur collecteur.
#[test]
fn test_narration_charge_epinglee() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{CanalAttr, Observer, TraceEvent};

    #[derive(Clone)]
    struct Collecteur {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let collecteur = Collecteur {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let mesure =
        executer(&Charge::AleaLcg, &["4".to_string()], Some(&collecteur)).expect("charge exécutée");
    let attendu = format!(
        "charge=alea-lcg noeuds={} verifie={}",
        mesure.noeuds, SATISFACTIONS_LCG_N4
    );
    let events = collecteur.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                valeur,
                ..
            } if *valeur == attendu
        )),
        "l'événement de narration porte le récapitulatif de la charge"
    );
}
