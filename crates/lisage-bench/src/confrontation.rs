//! Confrontation lisage ↔ témoins externes (ADR-0102, décision humaine du
//! 10 nov. 2026 : témoins en conteneurs Docker locaux, hors CI) : côté
//! lisage du harnais de mesures temps/mémoire sur les variantes BDD, MDD
//! et encodages de la fenêtre glissante. Ce module exécute **une charge une
//! seule fois** et rend une [`MesureConfrontation`] ; le binaire
//! `confrontation` la sérialise en une ligne JSON sur stdout ; l'orchestrateur
//! `python -m scripts bench-confrontation` lance les répétitions (un
//! processus neuf par mesure : la RSS pic du processus entier ne se divise
//! pas en répétitions in-process) et agrège.
//!
//! Les mêmes charges sont implémentées chez chaque témoin (CUDD/Sylvan pour
//! le booléen, libDDD pour les DDD de l'école de Paris, Meddly pour les MDD)
//! avec la même convention : même entrée, même objet construit, même valeur
//! de vérification croisée. Le protocole complet (équité, métriques,
//! reproduction) vit dans `docs/bench-confrontation-temoins-et-mesures.md`.
//!
//! Bloc mesuré, identique chez tous les outils : lecture de l'entrée +
//! construction du diagramme + comptage de vérification. L'initialisation
//! de la bibliothèque (manager, workers) est hors du bloc : elle varie d'un
//! outil à l'autre et n'est pas la charge comparée.

use std::collections::BTreeMap;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, Pool, PoolError};
use lisage_core::arena::Arena;
use lisage_core::atelier::{
    AtelierController, Encodage, Reductions, compter_chemins, compter_noeuds, construire_fenetre,
};
use lisage_core::bdd::{
    BddController, BddKeys, compiler_bdd, compter_noeuds as compter_noeuds_bdd,
};
use lisage_core::error::CoreError;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter as compter_pli, union};
use lisage_core::schema::{
    Domaine, ReferenceController, Schema, SchemaError, SchemaKeys, compiler,
};
use lisage_lang::value::{Atom, Value};

/// Ancre de mesure (une seule ancre par mesure, convention du harnais).
const ANCRE_MESURE: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `eat` (valeur interdite sur deux voisins circulairement).
const MANGE: u64 = 2;
/// Pas des clés de l'intercalage : clé `2i` = `pc_i`, clé `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Graine du LCG de la table de vérité déterministe (même LCG que la ligne
/// de base M1-8 : `benches/micro.rs` et `cudd_local.rs`).
pub const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplicateur du LCG (Knuth).
pub const MULTIPLICATEUR_LCG: u64 = 6_364_136_223_846_793_005;
/// Incrément du LCG.
pub const INCREMENT_LCG: u64 = 1_442_695_040_888_963_407;
/// Nombre maximal de variables acceptées par la charge `alea-lcg` (garde :
/// la table de vérité est `2^n` entrées en mémoire).
const VARIABLES_MAXIMALES_ALEA: u32 = 24;

/// Une mesure de confrontation : le résultat d'une charge exécutée une fois.
#[derive(Clone, PartialEq, Debug)]
pub struct MesureConfrontation {
    /// Outil mesuré : `lisage`, `cudd`, `sylvan`, `libddd`, `meddly`.
    pub outil: String,
    /// Variante de diagramme mesurée : `bdd`, `mdd`, `ddd`, `sdd`.
    pub variante: String,
    /// Charge exécutée : `alea-lcg`, `philosophes`, `fenetre`.
    pub charge: String,
    /// Paramètres de la charge, dans un ordre stable (sérialisés en objet).
    pub parametres: Vec<(String, String)>,
    /// Encodage de la charge `fenetre` uniquement (`absolu`/`relatif`).
    pub encodage: Option<String>,
    /// Temps du bloc mesuré, en millisecondes.
    pub temps_ms: f64,
    /// RSS maximale du processus, en kilo-octets (`VmHWM`).
    pub rss_max_ko: u64,
    /// Nœuds du diagramme construit, terminaux compris (convention
    /// documentée par outil dans le protocole).
    pub noeuds: u64,
    /// Valeur de vérification croisée : identique chez tous les outils pour
    /// la même charge (cardinal compté, satisfactions). Toute divergence
    /// entre outils invalide la mesure.
    pub verifie: u64,
}

/// Erreur typée du harnais de confrontation.
#[derive(Clone, PartialEq, Debug)]
pub enum ErreurConfrontation {
    /// La charge demandée n'existe pas.
    ChargeInconnue {
        /// Le nom reçu.
        charge: String,
    },
    /// Paramètres invalides pour la charge (nombre, type, bornes).
    ParametresInvalides {
        /// La charge concernée.
        charge: &'static str,
        /// Le détail de l'invalidité.
        detail: String,
    },
    /// Entrée illisible (fichier d'états, `/proc` absent).
    EntreeInvalide {
        /// Le détail de l'invalidité.
        detail: String,
    },
    /// La vérification croisée interne a échoué : la valeur comptée ne
    /// correspond pas à la valeur attendue.
    VerifieEchoue {
        /// La valeur attendue.
        attendu: u64,
        /// La valeur obtenue.
        obtenu: u64,
    },
    /// Erreur du cœur (pool, pli, atelier), cause préservée.
    Interne {
        /// L'erreur du cœur.
        source: CoreError,
    },
    /// Erreur de schéma (compilation d'une valeur hors domaine), cause
    /// préservée.
    Schema {
        /// L'erreur de schéma.
        source: SchemaError,
    },
}

impl std::fmt::Display for ErreurConfrontation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurConfrontation::ChargeInconnue { charge } => {
                write!(formatter, "charge inconnue: {charge}")
            }
            ErreurConfrontation::ParametresInvalides { charge, detail } => {
                write!(formatter, "paramètres invalides pour {charge}: {detail}")
            }
            ErreurConfrontation::EntreeInvalide { detail } => {
                write!(formatter, "entrée invalide: {detail}")
            }
            ErreurConfrontation::VerifieEchoue { attendu, obtenu } => {
                write!(
                    formatter,
                    "vérification croisée échouée: attendu {attendu}, obtenu {obtenu}"
                )
            }
            ErreurConfrontation::Interne { source } => {
                write!(formatter, "erreur du cœur: {source}")
            }
            ErreurConfrontation::Schema { source } => {
                write!(formatter, "erreur de schéma: {source}")
            }
        }
    }
}

impl std::error::Error for ErreurConfrontation {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErreurConfrontation::Interne { source } => Some(source),
            ErreurConfrontation::Schema { source } => Some(source),
            _ => None,
        }
    }
}

impl From<CoreError> for ErreurConfrontation {
    fn from(source: CoreError) -> Self {
        ErreurConfrontation::Interne { source }
    }
}

impl From<PoolError> for ErreurConfrontation {
    fn from(source: PoolError) -> Self {
        ErreurConfrontation::Interne {
            source: CoreError::Pool(source),
        }
    }
}

impl From<SchemaError> for ErreurConfrontation {
    fn from(source: SchemaError) -> Self {
        ErreurConfrontation::Schema { source }
    }
}

/// Échappe une chaîne pour JSON (même échappement minimal que
/// `lisage-liseuse::trace` : les valeurs mesurées sont un vocabulaire
/// contrôlé, le contrat reste correct).
fn echapper_json(chaine: &str) -> String {
    let mut sortie = String::with_capacity(chaine.len() + 2);
    for caractere in chaine.chars() {
        match caractere {
            '"' => sortie.push_str("\\\""),
            '\\' => sortie.push_str("\\\\"),
            '\n' => sortie.push_str("\\n"),
            '\r' => sortie.push_str("\\r"),
            '\t' => sortie.push_str("\\t"),
            caractere_bas if (caractere_bas as u32) < 0x20 => {
                sortie.push_str(&format!("\\u{:04x}", caractere_bas as u32));
            }
            caractere_simple => sortie.push(caractere_simple),
        }
    }
    sortie
}

/// Sérialise une mesure en une ligne JSON (sans le `\n` final). L'ordre des
/// clés est fixe et épinglé par test : `outil`, `variante`, `charge`,
/// `parametres`, `encodage` (fenêtre seulement), `temps_ms`, `rss_max_ko`,
/// `noeuds`, `verifie`.
pub fn serialiser_ligne(mesure: &MesureConfrontation) -> String {
    let mut ligne = String::with_capacity(256);
    ligne.push('{');
    ligne.push_str(&format!(
        "\"outil\":\"{}\",\"variante\":\"{}\",\"charge\":\"{}\"",
        echapper_json(&mesure.outil),
        echapper_json(&mesure.variante),
        echapper_json(&mesure.charge)
    ));
    ligne.push_str(",\"parametres\":{");
    for (indice, (cle, valeur)) in mesure.parametres.iter().enumerate() {
        if indice > 0 {
            ligne.push(',');
        }
        ligne.push_str(&format!(
            "\"{}\":\"{}\"",
            echapper_json(cle),
            echapper_json(valeur)
        ));
    }
    ligne.push('}');
    if let Some(encodage) = &mesure.encodage {
        ligne.push_str(&format!(",\"encodage\":\"{}\"", echapper_json(encodage)));
    }
    ligne.push_str(&format!(
        ",\"temps_ms\":{:.3},\"rss_max_ko\":{},\"noeuds\":{},\"verifie\":{}",
        mesure.temps_ms, mesure.rss_max_ko, mesure.noeuds, mesure.verifie
    ));
    ligne.push('}');
    ligne
}

/// RSS maximale du processus en kilo-octets (`VmHWM` de `/proc/self/status`).
/// Linux seulement, documenté dans le protocole : les exécutions locales et
/// les conteneurs des témoins sont Linux.
fn rss_max_ko() -> Result<u64, ErreurConfrontation> {
    const MARQUEUR: &str = "VmHWM:";
    let status = std::fs::read_to_string("/proc/self/status").map_err(|erreur| {
        ErreurConfrontation::EntreeInvalide {
            detail: format!("/proc/self/status illisible : {erreur}"),
        }
    })?;
    for ligne in status.lines() {
        if let Some(valeurs) = ligne.strip_prefix(MARQUEUR) {
            return valeurs
                .split_whitespace()
                .next()
                .and_then(|valeur| valeur.parse::<u64>().ok())
                .ok_or_else(|| ErreurConfrontation::EntreeInvalide {
                    detail: format!("ligne {MARQUEUR} inattendue : {ligne}"),
                });
        }
    }
    Err(ErreurConfrontation::EntreeInvalide {
        detail: "VmHWM absent de /proc/self/status (plateforme non Linux)".to_string(),
    })
}

/// Génère la table de vérité déterministe du LCG sur `n` variables (bit de
/// poids fort de l'état, même LCG que la ligne de base M1-8).
fn table_lcg(nombre_variables: u32) -> Vec<bool> {
    let mut etat = GRAINE_LCG;
    (0..(1usize << nombre_variables))
        .map(|_| {
            etat = etat
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            (etat >> 63) == 1
        })
        .collect()
}

/// Charge `alea-lcg` : BDD classique d'une table de vérité aléatoire
/// déterministe, construction par expansion de Shannon. La falsification
/// interne C6 est exécutée : le compte de nœuds de l'arène doit être celui
/// du témoin mini-BDD indépendant, et l'évaluation doit suivre la table.
fn charge_alea_lcg(nombre_variables: u32) -> Result<MesureConfrontation, ErreurConfrontation> {
    if nombre_variables == 0 || nombre_variables > VARIABLES_MAXIMALES_ALEA {
        return Err(ErreurConfrontation::ParametresInvalides {
            charge: "alea-lcg",
            detail: format!("n doit être dans 1..={VARIABLES_MAXIMALES_ALEA}"),
        });
    }
    let table = table_lcg(nombre_variables);
    let satisfait_attendu = table.iter().filter(|valeur| **valeur).count() as u64;
    let keys = BddKeys::new(nombre_variables);
    let mut pool = Arena::new(BddController::new(nombre_variables), Box::new(NoOpObserver));
    let debut = Instant::now();
    let racine = compiler_bdd(&mut pool, ANCRE_MESURE, &keys, &table);
    let noeuds = compter_noeuds_bdd(&pool, racine);
    let duree = debut.elapsed();
    // Falsification C6 : l'arène et le témoin mini-BDD indépendant doivent
    // compter le même nombre de nœuds sur la même table.
    let mut temoin = crate::temoin_bdd::MiniBdd::new();
    let racine_temoin = temoin.compiler_table(nombre_variables, &table);
    let noeuds_temoin = temoin.compter_noeuds(racine_temoin);
    if noeuds != noeuds_temoin {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu: noeuds_temoin,
            obtenu: noeuds,
        });
    }
    if temoin.evaluer(racine_temoin, 0) != table[0] {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu: table[0] as u64,
            obtenu: temoin.evaluer(racine_temoin, 0) as u64,
        });
    }
    Ok(MesureConfrontation {
        outil: "lisage".to_string(),
        variante: "bdd".to_string(),
        charge: "alea-lcg".to_string(),
        parametres: vec![("n".to_string(), nombre_variables.to_string())],
        encodage: None,
        temps_ms: duree.as_secs_f64() * 1_000.0,
        rss_max_ko: rss_max_ko()?,
        noeuds,
        verifie: satisfait_attendu,
    })
}

/// État étendu d'un philosophe : `pc_i` (pense/affamé/mange) et `left_i`
/// dérivée (libre/tenue/precedente), intercalés `pc_0 left_0 pc_1 …`.
type EtatEtendu = Vec<u64>;

/// Charge `philosophes` : R_n du fil rouge (docs/exemple-fil-rouge) lu depuis
/// le fichier d'états étendus partagé par tous les outils (première ligne
/// `n <n>`, puis un état étendu par ligne, valeurs séparées par des
/// espaces). Construction : union des maps compilées sur le schéma
/// intercalé ; le cardinal par pli `(ℕ,+,×)` doit être le nombre d'états
/// du fichier.
fn charge_philosophes(
    nombre_philosophes: u64,
    chemin: &str,
) -> Result<MesureConfrontation, ErreurConfrontation> {
    let contenu =
        std::fs::read_to_string(chemin).map_err(|erreur| ErreurConfrontation::EntreeInvalide {
            detail: format!("fichier d'états {chemin} illisible : {erreur}"),
        })?;
    let etats = lire_etats(nombre_philosophes, &contenu)?;
    let attendu = etats.len() as u64;
    let schema = Schema::Map(
        (0..PAS_CLE_INTERCALAGE * nombre_philosophes)
            .map(Atom)
            .collect(),
        Domaine::nouveau(vec![Atom(PENSE), Atom(1), Atom(MANGE)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let debut = Instant::now();
    let mut racine = pool.intern(ANCRE_MESURE, Form::Terminal(false));
    for etat in &etats {
        let map: BTreeMap<Value, Value> = etat
            .iter()
            .enumerate()
            .map(|(indice, valeur)| (Value::Atom(Atom(indice as u64)), Value::Atom(Atom(*valeur))))
            .collect();
        let membre = compiler(&mut pool, ANCRE_MESURE, &schema, &Value::Map(map))?;
        racine = union(&mut pool, &keys, ANCRE_MESURE, racine, membre)?;
    }
    let verifie = compter_pli(&pool, &mut sortie, ANCRE_MESURE, &schema, racine)?;
    let noeuds = compter_noeuds(&pool, racine)?;
    let duree = debut.elapsed();
    if verifie != attendu {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu,
            obtenu: verifie,
        });
    }
    Ok(MesureConfrontation {
        outil: "lisage".to_string(),
        variante: "mdd".to_string(),
        charge: "philosophes".to_string(),
        parametres: vec![("n".to_string(), nombre_philosophes.to_string())],
        encodage: None,
        temps_ms: duree.as_secs_f64() * 1_000.0,
        rss_max_ko: rss_max_ko()?,
        noeuds,
        verifie,
    })
}

/// Lit et valide le fichier d'états étendus : première ligne `n <n>`, puis
/// `2n` valeurs par ligne dans `{0, 1, 2}`.
fn lire_etats(
    nombre_philosophes: u64,
    contenu: &str,
) -> Result<Vec<EtatEtendu>, ErreurConfrontation> {
    let nombre_valeurs = PAS_CLE_INTERCALAGE as usize * nombre_philosophes as usize;
    let mut lignes = contenu.lines();
    let premiere = lignes
        .next()
        .ok_or_else(|| ErreurConfrontation::EntreeInvalide {
            detail: "fichier d'états vide".to_string(),
        })?;
    let champs_premiere: Vec<&str> = premiere.split_whitespace().collect();
    if champs_premiere.len() != 2 || champs_premiere[0] != "n" {
        return Err(ErreurConfrontation::EntreeInvalide {
            detail: format!("première ligne attendue « n <n> », reçue : {premiere}"),
        });
    }
    let n_declare: u64 =
        champs_premiere[1]
            .parse()
            .map_err(|_| ErreurConfrontation::EntreeInvalide {
                detail: format!("n illisible : {}", champs_premiere[1]),
            })?;
    if n_declare != nombre_philosophes {
        return Err(ErreurConfrontation::EntreeInvalide {
            detail: format!("n déclaré {n_declare} ≠ n demandé {nombre_philosophes}"),
        });
    }
    let mut etats = Vec::new();
    for (numero, ligne) in lignes.enumerate() {
        if ligne.trim().is_empty() {
            continue;
        }
        let etat: Result<Vec<u64>, _> = ligne
            .split_whitespace()
            .map(|valeur| valeur.parse::<u64>())
            .collect();
        let etat = etat.map_err(|_| ErreurConfrontation::EntreeInvalide {
            detail: format!("ligne {} : valeur illisible", numero + 2),
        })?;
        if etat.len() != nombre_valeurs {
            return Err(ErreurConfrontation::EntreeInvalide {
                detail: format!(
                    "ligne {} : {} valeurs, {} attendues",
                    numero + 2,
                    etat.len(),
                    nombre_valeurs
                ),
            });
        }
        if etat.iter().any(|valeur| *valeur > MANGE) {
            return Err(ErreurConfrontation::EntreeInvalide {
                detail: format!("ligne {} : valeur hors du domaine {{0,1,2}}", numero + 2),
            });
        }
        etats.push(etat);
    }
    Ok(etats)
}

/// Charge `fenetre` : la fenêtre glissante du fil rouge n° 2 dans l'encodage
/// demandé. Vérification : les chemins racine → terminaux doivent être
/// `taille × nb_bases` (une fenêtre par base, `taille` positions par
/// fenêtre), quel que soit l'encodage — seul le nombre de nœuds partagés
/// diffère (absolu : un nœud par base ; relatif : une seule forme, O(1) en
/// `nb_bases`).
fn charge_fenetre(
    taille: u64,
    nb_bases: u64,
    encodage: Encodage,
) -> Result<MesureConfrontation, ErreurConfrontation> {
    if taille == 0 || nb_bases == 0 {
        return Err(ErreurConfrontation::ParametresInvalides {
            charge: "fenetre",
            detail: "taille et nb_bases doivent être ≥ 1".to_string(),
        });
    }
    let attendu = taille.saturating_mul(nb_bases);
    let mut pool = Arena::new(
        AtelierController::nouveau(BddController::new(2), Reductions::sans_collapse()),
        Box::new(NoOpObserver),
    );
    let nom_encodage = match encodage {
        Encodage::Absolu => "absolu",
        Encodage::Relatif => "relatif",
    };
    let debut = Instant::now();
    let racine = construire_fenetre(&mut pool, ANCRE_MESURE, encodage, taille, nb_bases);
    let verifie = compter_chemins(&pool, racine)?;
    let noeuds = compter_noeuds(&pool, racine)?;
    let duree = debut.elapsed();
    if verifie != attendu {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu,
            obtenu: verifie,
        });
    }
    Ok(MesureConfrontation {
        outil: "lisage".to_string(),
        variante: "mdd".to_string(),
        charge: "fenetre".to_string(),
        parametres: vec![
            ("taille".to_string(), taille.to_string()),
            ("nb_bases".to_string(), nb_bases.to_string()),
        ],
        encodage: Some(nom_encodage.to_string()),
        temps_ms: duree.as_secs_f64() * 1_000.0,
        rss_max_ko: rss_max_ko()?,
        noeuds,
        verifie,
    })
}

/// Exécute une charge une fois et rend la mesure. Arguments :
/// `alea-lcg <n>`, `philosophes <n> <chemin>`,
/// `fenetre <taille> <nb_bases> [absolu|relatif]` (absolu par défaut).
pub fn executer(
    charge: &str,
    parametres: &[String],
) -> Result<MesureConfrontation, ErreurConfrontation> {
    match charge {
        "alea-lcg" => {
            let seul =
                parametres
                    .first()
                    .ok_or_else(|| ErreurConfrontation::ParametresInvalides {
                        charge: "alea-lcg",
                        detail: "attendu : alea-lcg <n>".to_string(),
                    })?;
            let nombre_variables =
                seul.parse::<u32>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: "alea-lcg",
                        detail: format!("n illisible : {seul}"),
                    })?;
            charge_alea_lcg(nombre_variables)
        }
        "philosophes" => {
            let (premier, second) = parametres.first().zip(parametres.get(1)).ok_or_else(|| {
                ErreurConfrontation::ParametresInvalides {
                    charge: "philosophes",
                    detail: "attendu : philosophes <n> <chemin>".to_string(),
                }
            })?;
            let nombre_philosophes =
                premier
                    .parse::<u64>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: "philosophes",
                        detail: format!("n illisible : {premier}"),
                    })?;
            charge_philosophes(nombre_philosophes, second)
        }
        "fenetre" => {
            let (premier, second) = parametres.first().zip(parametres.get(1)).ok_or_else(|| {
                ErreurConfrontation::ParametresInvalides {
                    charge: "fenetre",
                    detail: "attendu : fenetre <taille> <nb_bases> [absolu|relatif]".to_string(),
                }
            })?;
            let taille =
                premier
                    .parse::<u64>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: "fenetre",
                        detail: format!("taille illisible : {premier}"),
                    })?;
            let nb_bases =
                second
                    .parse::<u64>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: "fenetre",
                        detail: format!("nb_bases illisible : {second}"),
                    })?;
            let encodage = match parametres.get(2).map(String::as_str) {
                None | Some("absolu") => Encodage::Absolu,
                Some("relatif") => Encodage::Relatif,
                Some(autre) => {
                    return Err(ErreurConfrontation::ParametresInvalides {
                        charge: "fenetre",
                        detail: format!("encodage inconnu : {autre}"),
                    });
                }
            };
            charge_fenetre(taille, nb_bases, encodage)
        }
        autre => Err(ErreurConfrontation::ChargeInconnue {
            charge: autre.to_string(),
        }),
    }
}
