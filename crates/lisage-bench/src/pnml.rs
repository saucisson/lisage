//! Parseur PNML et conversion vers `lang/` (M5-2, énoncés C3/C4).
//!
//! C'est un parseur de **format externe** (PNML, le standard d'échange
//! des réseaux de Petri de la Model Checking Contest), pas du langage
//! Lisage : il produit des [`Value`] du niveau 1, consommables par
//! l'interprète de référence (voie A du triptyque). Aucun nouveau
//! dépendance : le XML est parsé à la main, en lecture seule.
//!
//! Sous-ensemble PNML supporté (suffisant pour les modèles MCC
//! "place/transition" usuels) : `<pnml><net><place id><initialMarking>
//! <text>n</text>`, `<transition id>`, `<arc id source target>`. Les
//! annotations de nom (`<name><text>…</text></name>`) sont lues quand
//! présentes.

use std::collections::{BTreeMap, BTreeSet};

use lisage_lang::value::{Atom, Value};

/// Erreur typée du parseur PNML.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PnmlError {
    /// XML mal formé (balise non fermée, attribut sans valeur, etc.).
    XmlMalForme { position: usize, detail: String },
    /// Élément PNML inattendu à cet endroit.
    ElementInattendu {
        attendu: &'static str,
        trouve: String,
    },
    /// Attribut obligatoire absent.
    AttributManquant {
        element: &'static str,
        attribut: &'static str,
    },
    /// Valeur numérique invalide.
    NombreInvalide {
        element: &'static str,
        texte: String,
    },
    /// Arc vers un nœud inexistant.
    ArcVersInconnu { source: String, target: String },
}

impl std::fmt::Display for PnmlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PnmlError::XmlMalForme { position, detail } => {
                write!(
                    formatter,
                    "XML mal formé à la position {position}: {detail}"
                )
            }
            PnmlError::ElementInattendu { attendu, trouve } => {
                write!(formatter, "élément attendu {attendu}, trouvé <{trouve}>")
            }
            PnmlError::AttributManquant { element, attribut } => {
                write!(formatter, "attribut {attribut} manquant sur <{element}>")
            }
            PnmlError::NombreInvalide { element, texte } => {
                write!(formatter, "nombre invalide dans <{element}>: {texte:?}")
            }
            PnmlError::ArcVersInconnu { source, target } => {
                write!(formatter, "arc de {source:?} vers {target:?} inconnu")
            }
        }
    }
}

impl std::error::Error for PnmlError {}

// ---------------------------------------------------------------------------
// XML minimal (lecture seule)
// ---------------------------------------------------------------------------

/// Nœud XML : élément avec attributs et enfants, ou texte.
#[derive(Clone, Debug, PartialEq, Eq)]
enum XmlNoeud {
    Element {
        nom: String,
        attributs: BTreeMap<String, String>,
        enfants: Vec<XmlNoeud>,
    },
    Texte(String),
}

/// Analyse un document XML en arbre de [`XmlNoeud`].
fn parser_xml(document: &str) -> Result<XmlNoeud, PnmlError> {
    let octets: Vec<char> = document.chars().collect();
    let mut position = 0;
    let racine = parser_element(&octets, &mut position)?;
    // Sauter la prologue `<?xml …?>` déjà gérée ; ignorer le texte
    // résiduel (espaces) après la racine.
    sauter_espaces(&octets, &mut position);
    if position != octets.len() {
        return Err(PnmlError::XmlMalForme {
            position,
            detail: "contenu après l'élément racine".to_string(),
        });
    }
    Ok(racine)
}

fn sauter_espaces(octets: &[char], position: &mut usize) {
    while *position < octets.len() && octets[*position].is_whitespace() {
        *position += 1;
    }
}

fn parser_element(octets: &[char], position: &mut usize) -> Result<XmlNoeud, PnmlError> {
    sauter_espaces(octets, position);
    if *position >= octets.len() || octets[*position] != '<' {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "élément attendu".to_string(),
        });
    }
    // Prologue ou commentaire : on les saute.
    if *position + 1 < octets.len()
        && (octets[*position + 1] == '?' || octets[*position + 1] == '!')
    {
        sauter_jusqua(octets, position, '>');
        return parser_element(octets, position);
    }
    *position += 1; // '<'
    let nom = lire_nom(octets, position)?;
    let mut attributs = BTreeMap::new();
    loop {
        sauter_espaces(octets, position);
        if *position >= octets.len() {
            return Err(PnmlError::XmlMalForme {
                position: *position,
                detail: format!("balise <{nom}> non fermée"),
            });
        }
        if octets[*position] == '/' {
            *position += 1;
            attendre(octets, position, '>')?;
            return Ok(XmlNoeud::Element {
                nom,
                attributs,
                enfants: Vec::new(),
            });
        }
        if octets[*position] == '>' {
            *position += 1;
            break;
        }
        let cle = lire_nom(octets, position)?;
        sauter_espaces(octets, position);
        attendre(octets, position, '=')?;
        sauter_espaces(octets, position);
        let valeur = lire_chaine(octets, position)?;
        attributs.insert(cle, valeur);
    }
    // Enfants jusqu'à la balise fermante.
    let mut enfants = Vec::new();
    loop {
        sauter_espaces(octets, position);
        if *position >= octets.len() {
            return Err(PnmlError::XmlMalForme {
                position: *position,
                detail: format!("fin de document dans <{nom}>"),
            });
        }
        if octets[*position] == '<' {
            if *position + 1 < octets.len() && octets[*position + 1] == '/' {
                *position += 2;
                let nom_fermeture = lire_nom(octets, position)?;
                if nom_fermeture != nom {
                    return Err(PnmlError::XmlMalForme {
                        position: *position,
                        detail: format!("fermeture </{nom_fermeture}> pour <{nom}>"),
                    });
                }
                attendre(octets, position, '>')?;
                return Ok(XmlNoeud::Element {
                    nom,
                    attributs,
                    enfants,
                });
            }
            enfants.push(parser_element(octets, position)?);
        } else {
            let debut = *position;
            while *position < octets.len() && octets[*position] != '<' {
                *position += 1;
            }
            let texte: String = octets[debut..*position].iter().collect();
            if !texte.trim().is_empty() {
                enfants.push(XmlNoeud::Texte(texte.trim().to_string()));
            }
        }
    }
}

fn lire_nom(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let debut = *position;
    while *position < octets.len()
        && (octets[*position].is_alphanumeric()
            || octets[*position] == '_'
            || octets[*position] == '-'
            || octets[*position] == ':'
            || octets[*position] == '.')
    {
        *position += 1;
    }
    if *position == debut {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "nom attendu".to_string(),
        });
    }
    Ok(octets[debut..*position].iter().collect())
}

fn lire_chaine(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    if *position >= octets.len() || (octets[*position] != '"' && octets[*position] != '\'') {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "chaîne entre guillemets attendue".to_string(),
        });
    }
    let delimiteur = octets[*position];
    *position += 1;
    let debut = *position;
    while *position < octets.len() && octets[*position] != delimiteur {
        *position += 1;
    }
    if *position >= octets.len() {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "chaîne non terminée".to_string(),
        });
    }
    let valeur: String = octets[debut..*position].iter().collect();
    *position += 1;
    Ok(valeur)
}

fn attendre(octets: &[char], position: &mut usize, attendu: char) -> Result<(), PnmlError> {
    if *position >= octets.len() || octets[*position] != attendu {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: format!("'{attendu}' attendu"),
        });
    }
    *position += 1;
    Ok(())
}

fn sauter_jusqua(octets: &[char], position: &mut usize, cible: char) {
    while *position < octets.len() && octets[*position] != cible {
        *position += 1;
    }
    if *position < octets.len() {
        *position += 1;
    }
}

// ---------------------------------------------------------------------------
// Modèle PNML
// ---------------------------------------------------------------------------

/// Place d'un réseau de Petri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub identifiant: String,
    pub nom: Option<String>,
    pub marquage_initial: u64,
}

/// Transition d'un réseau de Petri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub identifiant: String,
    pub nom: Option<String>,
}

/// Arc orienté (poids 1, suffisant pour le sous-ensemble MCC visé).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arc {
    pub source: String,
    pub cible: String,
}

/// Réseau de Petri parsé depuis PNML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReseauPetri {
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
}

impl ReseauPetri {
    /// Parse un document PNML complet.
    pub fn depuis_pnml(document: &str) -> Result<ReseauPetri, PnmlError> {
        let racine = parser_xml(document)?;
        let XmlNoeud::Element { nom, enfants, .. } = &racine else {
            return Err(PnmlError::ElementInattendu {
                attendu: "pnml",
                trouve: "texte".to_string(),
            });
        };
        if nom != "pnml" {
            return Err(PnmlError::ElementInattendu {
                attendu: "pnml",
                trouve: nom.clone(),
            });
        }
        let net = enfants
            .iter()
            .find_map(|enfant| match enfant {
                XmlNoeud::Element { nom, enfants, .. } if nom == "net" => Some(enfants),
                _ => None,
            })
            .ok_or_else(|| PnmlError::ElementInattendu {
                attendu: "net",
                trouve: "rien".to_string(),
            })?;
        let mut places = Vec::new();
        let mut transitions = Vec::new();
        let mut arcs = Vec::new();
        for enfant in net {
            let XmlNoeud::Element {
                nom,
                attributs,
                enfants,
            } = enfant
            else {
                continue;
            };
            match nom.as_str() {
                "place" => places.push(parser_place(attributs, enfants)?),
                "transition" => transitions.push(parser_transition(attributs, enfants)?),
                "arc" => arcs.push(parser_arc(attributs)?),
                _ => {}
            }
        }
        // Vérification : tout arc pointe vers des nœuds existants.
        let ids_places: BTreeSet<&str> = places.iter().map(|p| p.identifiant.as_str()).collect();
        let ids_transitions: BTreeSet<&str> =
            transitions.iter().map(|t| t.identifiant.as_str()).collect();
        for arc in &arcs {
            let source_existe = ids_places.contains(arc.source.as_str())
                || ids_transitions.contains(arc.source.as_str());
            let cible_existe = ids_places.contains(arc.cible.as_str())
                || ids_transitions.contains(arc.cible.as_str());
            if !source_existe || !cible_existe {
                return Err(PnmlError::ArcVersInconnu {
                    source: arc.source.clone(),
                    target: arc.cible.clone(),
                });
            }
        }
        Ok(ReseauPetri {
            places,
            transitions,
            arcs,
        })
    }

    /// Nombre de places.
    pub fn nb_places(&self) -> usize {
        self.places.len()
    }

    /// Nombre de transitions.
    pub fn nb_transitions(&self) -> usize {
        self.transitions.len()
    }
}

fn attribut_obligatoire(
    attributs: &BTreeMap<String, String>,
    element: &'static str,
    nom: &'static str,
) -> Result<String, PnmlError> {
    attributs
        .get(nom)
        .cloned()
        .ok_or(PnmlError::AttributManquant {
            element,
            attribut: nom,
        })
}

fn lire_nom_optionnel(enfants: &[XmlNoeud]) -> Option<String> {
    enfants.iter().find_map(|enfant| match enfant {
        XmlNoeud::Element { nom, enfants, .. } if nom == "name" => {
            lire_texte_enfant(enfants, "text")
        }
        _ => None,
    })
}

fn lire_texte_enfant(enfants: &[XmlNoeud], element: &str) -> Option<String> {
    enfants.iter().find_map(|enfant| match enfant {
        XmlNoeud::Element { nom, enfants, .. } if nom == element => {
            // Le texte est soit un enfant direct, soit dans <text>.
            enfants.iter().find_map(|e| match e {
                XmlNoeud::Texte(texte) => Some(texte.clone()),
                XmlNoeud::Element { nom, enfants, .. } if nom == "text" => {
                    enfants.iter().find_map(|t| match t {
                        XmlNoeud::Texte(texte) => Some(texte.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
        }
        _ => None,
    })
}

fn parser_place(
    attributs: &BTreeMap<String, String>,
    enfants: &[XmlNoeud],
) -> Result<Place, PnmlError> {
    let identifiant = attribut_obligatoire(attributs, "place", "id")?;
    let nom = lire_nom_optionnel(enfants);
    let marquage_initial = match lire_texte_enfant(enfants, "initialMarking") {
        Some(texte) => texte
            .parse::<u64>()
            .map_err(|_| PnmlError::NombreInvalide {
                element: "initialMarking",
                texte,
            })?,
        None => 0,
    };
    Ok(Place {
        identifiant,
        nom,
        marquage_initial,
    })
}

fn parser_transition(
    attributs: &BTreeMap<String, String>,
    enfants: &[XmlNoeud],
) -> Result<Transition, PnmlError> {
    let identifiant = attribut_obligatoire(attributs, "transition", "id")?;
    let nom = lire_nom_optionnel(enfants);
    Ok(Transition { identifiant, nom })
}

fn parser_arc(attributs: &BTreeMap<String, String>) -> Result<Arc, PnmlError> {
    let source = attribut_obligatoire(attributs, "arc", "source")?;
    let cible = attribut_obligatoire(attributs, "arc", "target")?;
    Ok(Arc { source, cible })
}

// ---------------------------------------------------------------------------
// Conversion vers lang/ (niveau 1)
// ---------------------------------------------------------------------------

/// Marquage d'un réseau : place → nombre de jetons, encodé comme
/// `Value::Map` du niveau 1 (clé = `Value::Atom` de l'index de place,
/// valeur = `Value::Atom` du compte).
pub type Marquage = BTreeMap<Atom, Atom>;

/// Convertit le marquage initial du réseau en [`Value::Map`].
pub fn marquage_initial(reseau: &ReseauPetri) -> Value {
    let map: BTreeMap<Value, Value> = reseau
        .places
        .iter()
        .enumerate()
        .map(|(index, place)| {
            (
                Value::Atom(Atom(index as u64)),
                Value::Atom(Atom(place.marquage_initial)),
            )
        })
        .collect();
    Value::Map(map)
}

/// Extrait le [`Marquage`] d'une [`Value::Map`] produite par
/// [`marquage_initial`].
pub fn marquage_depuis_value(valeur: &Value) -> Option<Marquage> {
    match valeur {
        Value::Map(map) => map
            .iter()
            .map(|(cle, compte)| match (cle, compte) {
                (Value::Atom(c), Value::Atom(n)) => Some((*c, *n)),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// Arcs entrants d'une transition (place → transition).
fn arcs_entrants<'a>(reseau: &'a ReseauPetri, transition: &str) -> Vec<&'a str> {
    reseau
        .arcs
        .iter()
        .filter(|arc| arc.cible == transition)
        .map(|arc| arc.source.as_str())
        .collect()
}

/// Arcs sortants d'une transition (transition → place).
fn arcs_sortants<'a>(reseau: &'a ReseauPetri, transition: &str) -> Vec<&'a str> {
    reseau
        .arcs
        .iter()
        .filter(|arc| arc.source == transition)
        .map(|arc| arc.cible.as_str())
        .collect()
}

/// Une transition est sensibilisée si chaque place entrante a au moins
/// un jeton.
pub fn est_sensibilisee(reseau: &ReseauPetri, transition: &str, marquage: &Marquage) -> bool {
    arcs_entrants(reseau, transition).iter().all(|place| {
        let index = reseau
            .places
            .iter()
            .position(|p| p.identifiant == *place)
            .expect("place référencée par un arc existe");
        marquage
            .get(&Atom(index as u64))
            .map(|jetons| jetons.0 >= 1)
            .unwrap_or(false)
    })
}

/// Tirez une transition : retire un jeton de chaque place entrante,
/// ajoute un jeton à chaque place sortante.
pub fn tirer(reseau: &ReseauPetri, transition: &str, marquage: &Marquage) -> Marquage {
    let mut resultat = marquage.clone();
    for place in arcs_entrants(reseau, transition) {
        let index = reseau
            .places
            .iter()
            .position(|p| p.identifiant == place)
            .expect("place référencée par un arc existe");
        let cle = Atom(index as u64);
        if let Some(jetons) = resultat.get_mut(&cle) {
            jetons.0 = jetons.0.saturating_sub(1);
        }
    }
    for place in arcs_sortants(reseau, transition) {
        let index = reseau
            .places
            .iter()
            .position(|p| p.identifiant == place)
            .expect("place référencée par un arc existe");
        let cle = Atom(index as u64);
        resultat
            .entry(cle)
            .and_modify(|jetons| jetons.0 += 1)
            .or_insert(Atom(1));
    }
    resultat
}

/// Pas sémantique : tous les marquages successeurs de tous les
/// transitions sensibilisées, encodés en `Value::Map` (voie A).
pub fn pas_semantique(reseau: &ReseauPetri, ensemble: &BTreeSet<Value>) -> BTreeSet<Value> {
    let mut successeurs = BTreeSet::new();
    for valeur in ensemble {
        let Some(marquage) = marquage_depuis_value(valeur) else {
            continue;
        };
        for transition in &reseau.transitions {
            if est_sensibilisee(reseau, &transition.identifiant, &marquage) {
                let suivant = tirer(reseau, &transition.identifiant, &marquage);
                let map: BTreeMap<Value, Value> = suivant
                    .into_iter()
                    .map(|(cle, compte)| (Value::Atom(cle), Value::Atom(compte)))
                    .collect();
                successeurs.insert(Value::Map(map));
            }
        }
    }
    successeurs
}

/// Espace des marquages atteignables (voie A, point fixe croissant sur
/// l'interprète de référence `lang/`).
pub fn espace_atteignable(reseau: &ReseauPetri) -> BTreeSet<Value> {
    let initial = BTreeSet::from([marquage_initial(reseau)]);
    lisage_lang::interpreter::saturate(|ensemble| pas_semantique(reseau, ensemble), initial)
}
