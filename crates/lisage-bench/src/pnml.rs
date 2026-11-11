//! Parseur PNML et conversion vers `lang/` (M5-2, énoncés C3/C4).
//!
//! C'est un parseur de **format externe** (PNML, le standard d'échange
//! des réseaux de Petri de la Model Checking Contest), pas du langage
//! Lisage : il produit des [`Value`] du niveau 1, consommables par
//! l'interprète de référence (voie A du triptyque). Aucune dépendance
//! externe : le XML est parsé à la main, en lecture seule.
//!
//! Sous-ensemble PNML supporté (suffisant pour les modèles MCC
//! "place/transition" usuels) : `<pnml><net><place id><initialMarking>
//! <text>n</text>`, `<transition id>`, `<arc id source target>` avec
//! `source`/`target` référençant chacun une place et une transition
//! (dans un sens ou dans l'autre). Les annotations de nom
//! (`<name><text>…</text></name>`) sont lues quand présentes. Un arc
//! portant un attribut `weight` autre que `1` est rejeté. Les entités
//! XML (`&amp;` etc.) ne sont pas décodées — acceptable pour les
//! modèles MCC PT-net.

use std::collections::{BTreeMap, BTreeSet};

use lisage_lang::value::{Atom, Value};

// ---------------------------------------------------------------------------
// Constantes du format
// ---------------------------------------------------------------------------

/// Élément racine PNML.
const ELEMENT_PNML: &str = "pnml";
/// Élément réseau.
const ELEMENT_NET: &str = "net";
/// Élément place.
const ELEMENT_PLACE: &str = "place";
/// Élément transition.
const ELEMENT_TRANSITION: &str = "transition";
/// Élément arc.
const ELEMENT_ARC: &str = "arc";
/// Attribut identifiant.
const ATTRIBUT_ID: &str = "id";
/// Attribut source d'arc.
const ATTRIBUT_SOURCE: &str = "source";
/// Attribut cible d'arc.
const ATTRIBUT_TARGET: &str = "target";
/// Attribut poids d'arc (sous-ensemble : poids 1 uniquement).
const ATTRIBUT_WEIGHT: &str = "weight";
/// Annotation de nom.
const ELEMENT_NAME: &str = "name";
/// Conteneur de texte PNML.
const ELEMENT_TEXT: &str = "text";
/// Marquage initial.
const ELEMENT_MARQUAGE: &str = "initialMarking";
/// Poids d'arc accepté (les poids > 1 sont hors sous-ensemble).
const POIDS_ACCEPTE: &str = "1";

// ---------------------------------------------------------------------------
// Types distincts pour les identifiants de nœuds (ADR-0006)
// ---------------------------------------------------------------------------

/// Identifiant d'une place, type distinct à la frontière.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PlaceId(pub String);

/// Identifiant d'une transition, type distinct à la frontière.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TransitionId(pub String);

/// Extrémité d'un arc : toujours un nœud du réseau.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NoeudId {
    Place(PlaceId),
    Transition(TransitionId),
}

// ---------------------------------------------------------------------------
// Erreurs typées
// ---------------------------------------------------------------------------

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
    /// Arc dont une extrémité n'existe pas dans le réseau.
    ArcVersInconnu { source: String, cible: String },
    /// Arc mal orienté : les deux extrémités sont des places, ou deux
    /// transitions (un arc PNML relie une place et une transition).
    ArcMalOriente { source: String, cible: String },
    /// Poids d'arc hors sous-ensemble (seul le poids 1 est supporté).
    PoidsNonSupporte { poids: String },
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
            PnmlError::ArcVersInconnu { source, cible } => {
                write!(formatter, "arc de {source:?} vers {cible:?} inconnu")
            }
            PnmlError::ArcMalOriente { source, cible } => {
                write!(
                    formatter,
                    "arc mal orienté de {source:?} vers {cible:?} (place↔transition attendu)"
                )
            }
            PnmlError::PoidsNonSupporte { poids } => {
                write!(
                    formatter,
                    "poids d'arc {poids:?} non supporté (poids 1 uniquement)"
                )
            }
        }
    }
}

impl std::error::Error for PnmlError {}

// ---------------------------------------------------------------------------
// XML minimal (lecture seule) — fonctions de taille bornée
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

/// Lit un nom XML (alphanumérique + `_ - : .`).
fn lire_nom(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let debut = *position;
    while *position < octets.len()
        && (octets[*position].is_alphanumeric()
            || matches!(octets[*position], '_' | '-' | ':' | '.'))
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

/// Lit une chaîne entre guillemets simples ou doubles.
fn lire_chaine(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let Some(&delimiteur) = octets.get(*position) else {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "chaîne entre guillemets attendue".to_string(),
        });
    };
    if delimiteur != '"' && delimiteur != '\'' {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "chaîne entre guillemets attendue".to_string(),
        });
    }
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
    match octets.get(*position) {
        Some(&c) if c == attendu => {
            *position += 1;
            Ok(())
        }
        _ => Err(PnmlError::XmlMalForme {
            position: *position,
            detail: format!("'{attendu}' attendu"),
        }),
    }
}

/// Saute jusqu'au prochain `>` (prologue `<?…?>`, commentaires, CDATA
/// tronqué — les entités ne sont pas décodées, voir la doc du module).
fn sauter_jusqua(octets: &[char], position: &mut usize, cible: char) {
    while *position < octets.len() && octets[*position] != cible {
        *position += 1;
    }
    if *position < octets.len() {
        *position += 1;
    }
}

/// Lit la balise ouvrante `<nom attr="v" …>` ou `<nom …/>`.
/// Retourne `(nom, attributs, est_auto_fermante)`.
fn lire_balise_ouvrante(
    octets: &[char],
    position: &mut usize,
) -> Result<(String, BTreeMap<String, String>, bool), PnmlError> {
    *position += 1; // '<'
    let nom = lire_nom(octets, position)?;
    let mut attributs = BTreeMap::new();
    loop {
        sauter_espaces(octets, position);
        let Some(&c) = octets.get(*position) else {
            return Err(PnmlError::XmlMalForme {
                position: *position,
                detail: format!("balise <{nom}> non fermée"),
            });
        };
        match c {
            '/' => {
                *position += 1;
                attendre(octets, position, '>')?;
                return Ok((nom, attributs, true));
            }
            '>' => {
                *position += 1;
                return Ok((nom, attributs, false));
            }
            _ => {
                let cle = lire_nom(octets, position)?;
                sauter_espaces(octets, position);
                attendre(octets, position, '=')?;
                sauter_espaces(octets, position);
                let valeur = lire_chaine(octets, position)?;
                attributs.insert(cle, valeur);
            }
        }
    }
}

/// Lit un tronçon de texte jusqu'au prochain '<'.
fn lire_texte(octets: &[char], position: &mut usize) -> String {
    let debut = *position;
    while *position < octets.len() && octets[*position] != '<' {
        *position += 1;
    }
    octets[debut..*position]
        .iter()
        .collect::<String>()
        .trim()
        .to_string()
}

/// Lit la balise fermante `</nom>` et vérifie la correspondance.
fn lire_balise_fermante(octets: &[char], position: &mut usize, nom: &str) -> Result<(), PnmlError> {
    *position += 2; // '</'
    let nom_fermeture = lire_nom(octets, position)?;
    if nom_fermeture != nom {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: format!("fermeture </{nom_fermeture}> pour <{nom}>"),
        });
    }
    attendre(octets, position, '>')
}

/// Parse un élément complet (récursif).
fn parser_element(octets: &[char], position: &mut usize) -> Result<XmlNoeud, PnmlError> {
    sauter_espaces(octets, position);
    if octets.get(*position) != Some(&'<') {
        return Err(PnmlError::XmlMalForme {
            position: *position,
            detail: "élément attendu".to_string(),
        });
    }
    // Prologue ou commentaire : on saute, on relance.
    if matches!(octets.get(*position + 1), Some('?') | Some('!')) {
        sauter_jusqua(octets, position, '>');
        return parser_element(octets, position);
    }
    let (nom, attributs, auto_fermante) = lire_balise_ouvrante(octets, position)?;
    if auto_fermante {
        return Ok(XmlNoeud::Element {
            nom,
            attributs,
            enfants: Vec::new(),
        });
    }
    let enfants = parser_enfants(octets, position, &nom)?;
    Ok(XmlNoeud::Element {
        nom,
        attributs,
        enfants,
    })
}

/// Parse les enfants d'un élément jusqu'à sa balise fermante.
fn parser_enfants(
    octets: &[char],
    position: &mut usize,
    nom: &str,
) -> Result<Vec<XmlNoeud>, PnmlError> {
    let mut enfants = Vec::new();
    loop {
        sauter_espaces(octets, position);
        let Some(&c) = octets.get(*position) else {
            return Err(PnmlError::XmlMalForme {
                position: *position,
                detail: format!("fin de document dans <{nom}>"),
            });
        };
        if c == '<' {
            if octets.get(*position + 1) == Some(&'/') {
                lire_balise_fermante(octets, position, nom)?;
                return Ok(enfants);
            }
            enfants.push(parser_element(octets, position)?);
        } else {
            let texte = lire_texte(octets, position);
            if !texte.is_empty() {
                enfants.push(XmlNoeud::Texte(texte));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Modèle PNML
// ---------------------------------------------------------------------------

/// Place d'un réseau de Petri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub identifiant: PlaceId,
    pub nom: Option<String>,
    pub marquage_initial: u64,
}

/// Transition d'un réseau de Petri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub identifiant: TransitionId,
    pub nom: Option<String>,
}

/// Arc orienté place↔transition (poids 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arc {
    pub source: NoeudId,
    pub cible: NoeudId,
}

/// Document PNML brut, type distinct à la frontière (ADR-0006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PnmlDocument(pub String);

/// Réseau de Petri parsé depuis PNML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReseauPetri {
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
}

impl ReseauPetri {
    /// Parse un document PNML complet.
    pub fn depuis_pnml(document: &PnmlDocument) -> Result<ReseauPetri, PnmlError> {
        let racine = parser_xml(&document.0)?;
        let XmlNoeud::Element { nom, enfants, .. } = &racine else {
            return Err(PnmlError::ElementInattendu {
                attendu: ELEMENT_PNML,
                trouve: "texte".to_string(),
            });
        };
        if nom != ELEMENT_PNML {
            return Err(PnmlError::ElementInattendu {
                attendu: ELEMENT_PNML,
                trouve: nom.clone(),
            });
        }
        let net = enfants
            .iter()
            .find_map(|enfant| match enfant {
                XmlNoeud::Element { nom, enfants, .. } if nom == ELEMENT_NET => Some(enfants),
                _ => None,
            })
            .ok_or_else(|| PnmlError::ElementInattendu {
                attendu: ELEMENT_NET,
                trouve: "rien".to_string(),
            })?;
        let mut places = Vec::new();
        let mut transitions = Vec::new();
        let mut arcs_bruts = Vec::new();
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
                ELEMENT_PLACE => places.push(parser_place(attributs, enfants)?),
                ELEMENT_TRANSITION => transitions.push(parser_transition(attributs, enfants)?),
                ELEMENT_ARC => arcs_bruts.push(parser_arc(attributs)?),
                _ => {}
            }
        }
        let arcs = resoudre_arcs(&places, &transitions, &arcs_bruts)?;
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

    /// Index d'une place d'après son identifiant.
    pub fn index_place(&self, identifiant: &PlaceId) -> Option<usize> {
        self.places
            .iter()
            .position(|p| &p.identifiant == identifiant)
    }
}

/// Arc brut tel que parsé (identifiants encore non résolus).
struct ArcBrut {
    source: String,
    cible: String,
}

/// Résout les identifiants d'arcs en [`NoeudId`] et vérifie que chaque
/// arc relie une place et une transition dont les deux extrémités
/// existent.
fn resoudre_arcs(
    places: &[Place],
    transitions: &[Transition],
    arcs_bruts: &[ArcBrut],
) -> Result<Vec<Arc>, PnmlError> {
    arcs_bruts
        .iter()
        .map(|brut| resoudre_arc(places, transitions, brut))
        .collect()
}

fn resoudre_arc(
    places: &[Place],
    transitions: &[Transition],
    brut: &ArcBrut,
) -> Result<Arc, PnmlError> {
    let source = resoudre_noeud(places, transitions, &brut.source);
    let cible = resoudre_noeud(places, transitions, &brut.cible);
    let (source, cible) = match (source, cible) {
        (Some(source), Some(cible)) => (source, cible),
        _ => {
            return Err(PnmlError::ArcVersInconnu {
                source: brut.source.clone(),
                cible: brut.cible.clone(),
            });
        }
    };
    let mal_oriente = matches!(
        (&source, &cible),
        (NoeudId::Place(_), NoeudId::Place(_)) | (NoeudId::Transition(_), NoeudId::Transition(_))
    );
    if mal_oriente {
        return Err(PnmlError::ArcMalOriente {
            source: brut.source.clone(),
            cible: brut.cible.clone(),
        });
    }
    Ok(Arc { source, cible })
}

fn resoudre_noeud(
    places: &[Place],
    transitions: &[Transition],
    identifiant: &str,
) -> Option<NoeudId> {
    if places.iter().any(|p| p.identifiant.0 == identifiant) {
        return Some(NoeudId::Place(PlaceId(identifiant.to_string())));
    }
    if transitions.iter().any(|t| t.identifiant.0 == identifiant) {
        return Some(NoeudId::Transition(TransitionId(identifiant.to_string())));
    }
    None
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

/// Texte d'un élément `<name><text>…</text></name>` ou
/// `<initialMarking><text>…</text></initialMarking>` : le texte est
/// soit un enfant direct, soit dans un sous-élément `<text>`.
fn lire_texte_enfant(enfants: &[XmlNoeud], element: &str) -> Option<String> {
    for enfant in enfants {
        let XmlNoeud::Element { nom, enfants, .. } = enfant else {
            continue;
        };
        if nom != element {
            continue;
        }
        return enfants.iter().find_map(texte_direct_ou_dans_text);
    }
    None
}

fn texte_direct_ou_dans_text(noeud: &XmlNoeud) -> Option<String> {
    match noeud {
        XmlNoeud::Texte(texte) => Some(texte.clone()),
        XmlNoeud::Element { nom, enfants, .. } if nom == ELEMENT_TEXT => {
            enfants.iter().find_map(|e| match e {
                XmlNoeud::Texte(texte) => Some(texte.clone()),
                _ => None,
            })
        }
        _ => None,
    }
}

fn lire_nom_optionnel(enfants: &[XmlNoeud]) -> Option<String> {
    lire_texte_enfant(enfants, ELEMENT_NAME)
}

fn parser_place(
    attributs: &BTreeMap<String, String>,
    enfants: &[XmlNoeud],
) -> Result<Place, PnmlError> {
    let identifiant = PlaceId(attribut_obligatoire(attributs, ELEMENT_PLACE, ATTRIBUT_ID)?);
    let nom = lire_nom_optionnel(enfants);
    let marquage_initial = match lire_texte_enfant(enfants, ELEMENT_MARQUAGE) {
        Some(texte) => texte
            .parse::<u64>()
            .map_err(|_| PnmlError::NombreInvalide {
                element: ELEMENT_MARQUAGE,
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
    let identifiant = TransitionId(attribut_obligatoire(
        attributs,
        ELEMENT_TRANSITION,
        ATTRIBUT_ID,
    )?);
    let nom = lire_nom_optionnel(enfants);
    Ok(Transition { identifiant, nom })
}

fn parser_arc(attributs: &BTreeMap<String, String>) -> Result<ArcBrut, PnmlError> {
    if let Some(poids) = attributs.get(ATTRIBUT_WEIGHT)
        && poids != POIDS_ACCEPTE
    {
        return Err(PnmlError::PoidsNonSupporte {
            poids: poids.clone(),
        });
    }
    let source = attribut_obligatoire(attributs, ELEMENT_ARC, ATTRIBUT_SOURCE)?;
    let cible = attribut_obligatoire(attributs, ELEMENT_ARC, ATTRIBUT_TARGET)?;
    Ok(ArcBrut { source, cible })
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

/// Places entrantes d'une transition (arcs place → transition).
fn places_entrantes<'a>(reseau: &'a ReseauPetri, transition: &TransitionId) -> Vec<&'a PlaceId> {
    reseau
        .arcs
        .iter()
        .filter_map(|arc| match (&arc.source, &arc.cible) {
            (NoeudId::Place(p), NoeudId::Transition(t)) if t == transition => Some(p),
            _ => None,
        })
        .collect()
}

/// Places sortantes d'une transition (arcs transition → place).
fn places_sortantes<'a>(reseau: &'a ReseauPetri, transition: &TransitionId) -> Vec<&'a PlaceId> {
    reseau
        .arcs
        .iter()
        .filter_map(|arc| match (&arc.source, &arc.cible) {
            (NoeudId::Transition(t), NoeudId::Place(p)) if t == transition => Some(p),
            _ => None,
        })
        .collect()
}

/// Une transition est sensibilisée si chaque place entrante a au moins
/// un jeton.
pub fn est_sensibilisee(
    reseau: &ReseauPetri,
    transition: &TransitionId,
    marquage: &Marquage,
) -> bool {
    places_entrantes(reseau, transition).iter().all(|place| {
        let Some(index) = reseau.index_place(place) else {
            return false;
        };
        marquage
            .get(&Atom(index as u64))
            .map(|jetons| jetons.0 >= 1)
            .unwrap_or(false)
    })
}

/// Tirez une transition : retire un jeton de chaque place entrante,
/// ajoute un jeton à chaque place sortante.
pub fn tirer(reseau: &ReseauPetri, transition: &TransitionId, marquage: &Marquage) -> Marquage {
    let mut resultat = marquage.clone();
    for place in places_entrantes(reseau, transition) {
        if let Some(index) = reseau.index_place(place) {
            let cle = Atom(index as u64);
            if let Some(jetons) = resultat.get_mut(&cle) {
                jetons.0 = jetons.0.saturating_sub(1);
            }
        }
    }
    for place in places_sortantes(reseau, transition) {
        if let Some(index) = reseau.index_place(place) {
            let cle = Atom(index as u64);
            resultat
                .entry(cle)
                .and_modify(|jetons| jetons.0 += 1)
                .or_insert(Atom(1));
        }
    }
    resultat
}

/// Pas sémantique : tous les marquages successeurs de toutes les
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
