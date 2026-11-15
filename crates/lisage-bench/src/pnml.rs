//! PNML parser and conversion to `lang/` (M5-2, statements C3/C4).
//!
//! This is a parser of an **external format** (PNML, the Petri-net exchange
//! standard of the Model Checking Contest), not of the
//! Lisage language: it produces level-1 [`Value`]s, consumable by
//! the reference interpreter (path A of the triptych). No external
//! dependency: the XML is parsed by hand, read-only.
//!
//! Supported PNML subset (sufficient for the usual MCC
//! "place/transition" models): `<pnml><net><place id><initialMarking>
//! <text>n</text>`, `<transition id>`, `<arc id source target>` with
//! `source`/`target` each referencing a place and a transition
//! (in either direction). Name annotations
//! (`<name><text>…</text></name>`) are read when present. An arc
//! carrying a `weight` attribute other than `1` is rejected. XML
//! entities (`&amp;` etc.) are not decoded — acceptable for
//! MCC PT-net models.

use std::collections::{BTreeMap, BTreeSet};

use lisage_lang::value::{Atom, Value};

// ---------------------------------------------------------------------------
// Format constants
// ---------------------------------------------------------------------------

/// PNML root element.
const ELEMENT_PNML: &str = "pnml";
/// Network element.
const ELEMENT_NET: &str = "net";
/// Place element.
const ELEMENT_PLACE: &str = "place";
/// Transition element.
const ELEMENT_TRANSITION: &str = "transition";
/// Arc element.
const ELEMENT_ARC: &str = "arc";
/// Identifier attribute.
const ATTRIBUTE_ID: &str = "id";
/// Arc source attribute.
const ATTRIBUTE_SOURCE: &str = "source";
/// Arc target attribute.
const ATTRIBUTE_TARGET: &str = "target";
/// Arc weight attribute (subset: weight 1 only).
const ATTRIBUTE_WEIGHT: &str = "weight";
/// Name annotation.
const ELEMENT_NAME: &str = "name";
/// PNML text container.
const ELEMENT_TEXT: &str = "text";
/// Initial marking.
const ELEMENT_MARQUAGE: &str = "initialMarking";
/// Accepted arc weight (weights > 1 are outside the subset).
const WEIGHT_ACCEPTE: &str = "1";

// ---------------------------------------------------------------------------
// Distinct types for node identifiers (ADR-0006)
// ---------------------------------------------------------------------------

/// Identifier of a place, distinct type at the boundary.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PlaceId(pub String);

/// Identifier of a transition, distinct type at the boundary.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TransitionId(pub String);

/// End of an arc: always a node of the net.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NodeId {
    Place(PlaceId),
    Transition(TransitionId),
}

// ---------------------------------------------------------------------------
// Typed errors
// ---------------------------------------------------------------------------

/// Typed error of the PNML parser.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PnmlError {
    /// Malformed XML (unclosed tag, attribute without value, etc.).
    XmlMalformed { position: usize, detail: String },
    /// Unexpected PNML element at this point.
    ElementInexpected {
        expected: &'static str,
        trouve: String,
    },
    /// Mandatory attribute absent.
    AttributeManquant {
        element: &'static str,
        attribute: &'static str,
    },
    /// Invalid numeric value.
    NumberInvalide {
        element: &'static str,
        texte: String,
    },
    /// Arc whose one end does not exist in the net.
    ArcVersInconnu { source: String, target: String },
    /// Badly oriented arc: both ends are places, or two
    /// transitions (a PNML arc links a place and a transition).
    ArcMalOriente { source: String, target: String },
    /// Arc weight outside the subset (only weight 1 is supported).
    WeightNonSupporte { weight: String },
}

impl std::fmt::Display for PnmlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PnmlError::XmlMalformed { position, detail } => {
                write!(formatter, "malformed XML at position {position}: {detail}")
            }
            PnmlError::ElementInexpected { expected, trouve } => {
                write!(formatter, "expected element {expected}, found <{trouve}>")
            }
            PnmlError::AttributeManquant { element, attribute } => {
                write!(formatter, "attribute {attribute} missing on <{element}>")
            }
            PnmlError::NumberInvalide { element, texte } => {
                write!(formatter, "invalid number in <{element}>: {texte:?}")
            }
            PnmlError::ArcVersInconnu { source, target } => {
                write!(formatter, "arc from {source:?} to {target:?} unknown")
            }
            PnmlError::ArcMalOriente { source, target } => {
                write!(
                    formatter,
                    "badly oriented arc from {source:?} to {target:?} (place↔transition expected)"
                )
            }
            PnmlError::WeightNonSupporte { weight } => {
                write!(
                    formatter,
                    "arc weight {weight:?} not supported (weight 1 only)"
                )
            }
        }
    }
}

impl std::error::Error for PnmlError {}

// ---------------------------------------------------------------------------
// Minimal XML (read-only) — bounded-size functions
// ---------------------------------------------------------------------------

/// XML node: element with attributes and children, or text.
#[derive(Clone, Debug, PartialEq, Eq)]
enum XmlNode {
    Element {
        nom: String,
        attributes: BTreeMap<String, String>,
        children: Vec<XmlNode>,
    },
    Texte(String),
}

/// Parses an XML document into a [`XmlNode`] tree.
fn parser_xml(document: &str) -> Result<XmlNode, PnmlError> {
    let octets: Vec<char> = document.chars().collect();
    let mut position = 0;
    let root = parser_element(&octets, &mut position)?;
    sauter_spaces(&octets, &mut position);
    if position != octets.len() {
        return Err(PnmlError::XmlMalformed {
            position,
            detail: "content after the root element".to_string(),
        });
    }
    Ok(root)
}

fn sauter_spaces(octets: &[char], position: &mut usize) {
    while *position < octets.len() && octets[*position].is_whitespace() {
        *position += 1;
    }
}

/// Reads an XML name (alphanumeric + `_ - : .`).
fn lire_nom(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let debut = *position;
    while *position < octets.len()
        && (octets[*position].is_alphanumeric()
            || matches!(octets[*position], '_' | '-' | ':' | '.'))
    {
        *position += 1;
    }
    if *position == debut {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "name expected".to_string(),
        });
    }
    Ok(octets[debut..*position].iter().collect())
}

/// Reads a string between single or double quotes.
fn lire_chaine(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let Some(&delimiteur) = octets.get(*position) else {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "quoted string expected".to_string(),
        });
    };
    if delimiteur != '"' && delimiteur != '\'' {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "quoted string expected".to_string(),
        });
    }
    *position += 1;
    let debut = *position;
    while *position < octets.len() && octets[*position] != delimiteur {
        *position += 1;
    }
    if *position >= octets.len() {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "unterminated string".to_string(),
        });
    }
    let value: String = octets[debut..*position].iter().collect();
    *position += 1;
    Ok(value)
}

fn attendre(octets: &[char], position: &mut usize, expected: char) -> Result<(), PnmlError> {
    match octets.get(*position) {
        Some(&c) if c == expected => {
            *position += 1;
            Ok(())
        }
        _ => Err(PnmlError::XmlMalformed {
            position: *position,
            detail: format!("'{expected}' expected"),
        }),
    }
}

/// Skips to the next `>` (prologue `<?…?>`, comments, truncated
/// CDATA — entities are not decoded, see the module doc).
fn sauter_jusqu_a(octets: &[char], position: &mut usize, target: char) {
    while *position < octets.len() && octets[*position] != target {
        *position += 1;
    }
    if *position < octets.len() {
        *position += 1;
    }
}

/// Reads the opening tag `<nom attr="v" …>` or `<nom …/>`.
/// Returns `(nom, attributes, est_auto_fermante)`.
fn lire_balise_ouvrante(
    octets: &[char],
    position: &mut usize,
) -> Result<(String, BTreeMap<String, String>, bool), PnmlError> {
    *position += 1; // '<'
    let nom = lire_nom(octets, position)?;
    let mut attributes = BTreeMap::new();
    loop {
        sauter_spaces(octets, position);
        let Some(&c) = octets.get(*position) else {
            return Err(PnmlError::XmlMalformed {
                position: *position,
                detail: format!("unclosed tag <{nom}>"),
            });
        };
        match c {
            '/' => {
                *position += 1;
                attendre(octets, position, '>')?;
                return Ok((nom, attributes, true));
            }
            '>' => {
                *position += 1;
                return Ok((nom, attributes, false));
            }
            _ => {
                let key = lire_nom(octets, position)?;
                sauter_spaces(octets, position);
                attendre(octets, position, '=')?;
                sauter_spaces(octets, position);
                let value = lire_chaine(octets, position)?;
                attributes.insert(key, value);
            }
        }
    }
}

/// Reads a text chunk up to the next '<'.
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

/// Reads the closing tag `</nom>` and checks the match.
fn lire_balise_fermante(octets: &[char], position: &mut usize, nom: &str) -> Result<(), PnmlError> {
    *position += 2; // '</'
    let nom_fermeture = lire_nom(octets, position)?;
    if nom_fermeture != nom {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: format!("closing </{nom_fermeture}> for <{nom}>"),
        });
    }
    attendre(octets, position, '>')
}

/// Parses a complete element (recursive).
fn parser_element(octets: &[char], position: &mut usize) -> Result<XmlNode, PnmlError> {
    sauter_spaces(octets, position);
    if octets.get(*position) != Some(&'<') {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "element expected".to_string(),
        });
    }
    // Prologue or comment: skip, retry.
    if matches!(octets.get(*position + 1), Some('?') | Some('!')) {
        sauter_jusqu_a(octets, position, '>');
        return parser_element(octets, position);
    }
    let (nom, attributes, auto_fermante) = lire_balise_ouvrante(octets, position)?;
    if auto_fermante {
        return Ok(XmlNode::Element {
            nom,
            attributes,
            children: Vec::new(),
        });
    }
    let children = parser_children(octets, position, &nom)?;
    Ok(XmlNode::Element {
        nom,
        attributes,
        children,
    })
}

/// Parses the children of an element up to its closing tag.
fn parser_children(
    octets: &[char],
    position: &mut usize,
    nom: &str,
) -> Result<Vec<XmlNode>, PnmlError> {
    let mut children = Vec::new();
    loop {
        sauter_spaces(octets, position);
        let Some(&c) = octets.get(*position) else {
            return Err(PnmlError::XmlMalformed {
                position: *position,
                detail: format!("end of document inside <{nom}>"),
            });
        };
        if c == '<' {
            if octets.get(*position + 1) == Some(&'/') {
                lire_balise_fermante(octets, position, nom)?;
                return Ok(children);
            }
            children.push(parser_element(octets, position)?);
        } else {
            let texte = lire_texte(octets, position);
            if !texte.is_empty() {
                children.push(XmlNode::Texte(texte));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PNML model
// ---------------------------------------------------------------------------

/// Place of a Petri net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub identifiant: PlaceId,
    pub nom: Option<String>,
    pub marquage_initial: u64,
}

/// Transition of a Petri net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub identifiant: TransitionId,
    pub nom: Option<String>,
}

/// Oriented arc place↔transition (weight 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arc {
    pub source: NodeId,
    pub target: NodeId,
}

/// Raw PNML document, distinct type at the boundary (ADR-0006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PnmlDocument(pub String);

/// Petri net parsed from PNML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkPetri {
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
}

impl NetworkPetri {
    /// Parses a complete PNML document.
    pub fn depuis_pnml(document: &PnmlDocument) -> Result<NetworkPetri, PnmlError> {
        let root = parser_xml(&document.0)?;
        let XmlNode::Element { nom, children, .. } = &root else {
            return Err(PnmlError::ElementInexpected {
                expected: ELEMENT_PNML,
                trouve: "text".to_string(),
            });
        };
        if nom != ELEMENT_PNML {
            return Err(PnmlError::ElementInexpected {
                expected: ELEMENT_PNML,
                trouve: nom.clone(),
            });
        }
        let net = children
            .iter()
            .find_map(|child| match child {
                XmlNode::Element { nom, children, .. } if nom == ELEMENT_NET => Some(children),
                _ => None,
            })
            .ok_or_else(|| PnmlError::ElementInexpected {
                expected: ELEMENT_NET,
                trouve: "nothing".to_string(),
            })?;
        let mut places = Vec::new();
        let mut transitions = Vec::new();
        let mut arcs_bruts = Vec::new();
        for child in net {
            let XmlNode::Element {
                nom,
                attributes,
                children,
            } = child
            else {
                continue;
            };
            match nom.as_str() {
                ELEMENT_PLACE => places.push(parser_place(attributes, children)?),
                ELEMENT_TRANSITION => transitions.push(parser_transition(attributes, children)?),
                ELEMENT_ARC => arcs_bruts.push(parser_arc(attributes)?),
                _ => {}
            }
        }
        let arcs = resoudre_arcs(&places, &transitions, &arcs_bruts)?;
        Ok(NetworkPetri {
            places,
            transitions,
            arcs,
        })
    }

    /// Number of places.
    pub fn number_places(&self) -> usize {
        self.places.len()
    }

    /// Number of transitions.
    pub fn number_transitions(&self) -> usize {
        self.transitions.len()
    }

    /// Index of a place from its identifier.
    pub fn index_place(&self, identifiant: &PlaceId) -> Option<usize> {
        self.places
            .iter()
            .position(|p| &p.identifiant == identifiant)
    }
}

/// Raw arc as parsed (identifiers not yet resolved).
struct ArcBrut {
    source: String,
    target: String,
}

/// Resolves arc identifiers into [`NodeId`] and checks that each
/// arc links a place and a transition whose both ends
/// exist.
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
    let source = resoudre_node(places, transitions, &brut.source);
    let target = resoudre_node(places, transitions, &brut.target);
    let (source, target) = match (source, target) {
        (Some(source), Some(target)) => (source, target),
        _ => {
            return Err(PnmlError::ArcVersInconnu {
                source: brut.source.clone(),
                target: brut.target.clone(),
            });
        }
    };
    let badly_oriented = matches!(
        (&source, &target),
        (NodeId::Place(_), NodeId::Place(_)) | (NodeId::Transition(_), NodeId::Transition(_))
    );
    if badly_oriented {
        return Err(PnmlError::ArcMalOriente {
            source: brut.source.clone(),
            target: brut.target.clone(),
        });
    }
    Ok(Arc { source, target })
}

fn resoudre_node(
    places: &[Place],
    transitions: &[Transition],
    identifiant: &str,
) -> Option<NodeId> {
    if places.iter().any(|p| p.identifiant.0 == identifiant) {
        return Some(NodeId::Place(PlaceId(identifiant.to_string())));
    }
    if transitions.iter().any(|t| t.identifiant.0 == identifiant) {
        return Some(NodeId::Transition(TransitionId(identifiant.to_string())));
    }
    None
}

fn attribute_obligatoire(
    attributes: &BTreeMap<String, String>,
    element: &'static str,
    nom: &'static str,
) -> Result<String, PnmlError> {
    attributes
        .get(nom)
        .cloned()
        .ok_or(PnmlError::AttributeManquant {
            element,
            attribute: nom,
        })
}

/// Text of a `<name><text>…</text></name>` or
/// `<initialMarking><text>…</text></initialMarking>` element: the text is
/// either a direct child, or inside a `<text>` sub-element.
fn lire_texte_child(children: &[XmlNode], element: &str) -> Option<String> {
    for child in children {
        let XmlNode::Element { nom, children, .. } = child else {
            continue;
        };
        if nom != element {
            continue;
        }
        return children.iter().find_map(texte_direct_ou_dans_text);
    }
    None
}

fn texte_direct_ou_dans_text(node: &XmlNode) -> Option<String> {
    match node {
        XmlNode::Texte(texte) => Some(texte.clone()),
        XmlNode::Element { nom, children, .. } if nom == ELEMENT_TEXT => {
            children.iter().find_map(|e| match e {
                XmlNode::Texte(texte) => Some(texte.clone()),
                _ => None,
            })
        }
        _ => None,
    }
}

fn lire_nom_optionnel(children: &[XmlNode]) -> Option<String> {
    lire_texte_child(children, ELEMENT_NAME)
}

fn parser_place(
    attributes: &BTreeMap<String, String>,
    children: &[XmlNode],
) -> Result<Place, PnmlError> {
    let identifiant = PlaceId(attribute_obligatoire(
        attributes,
        ELEMENT_PLACE,
        ATTRIBUTE_ID,
    )?);
    let nom = lire_nom_optionnel(children);
    let marquage_initial = match lire_texte_child(children, ELEMENT_MARQUAGE) {
        Some(texte) => texte
            .parse::<u64>()
            .map_err(|_| PnmlError::NumberInvalide {
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
    attributes: &BTreeMap<String, String>,
    children: &[XmlNode],
) -> Result<Transition, PnmlError> {
    let identifiant = TransitionId(attribute_obligatoire(
        attributes,
        ELEMENT_TRANSITION,
        ATTRIBUTE_ID,
    )?);
    let nom = lire_nom_optionnel(children);
    Ok(Transition { identifiant, nom })
}

fn parser_arc(attributes: &BTreeMap<String, String>) -> Result<ArcBrut, PnmlError> {
    if let Some(weight) = attributes.get(ATTRIBUTE_WEIGHT)
        && weight != WEIGHT_ACCEPTE
    {
        return Err(PnmlError::WeightNonSupporte {
            weight: weight.clone(),
        });
    }
    let source = attribute_obligatoire(attributes, ELEMENT_ARC, ATTRIBUTE_SOURCE)?;
    let target = attribute_obligatoire(attributes, ELEMENT_ARC, ATTRIBUTE_TARGET)?;
    Ok(ArcBrut { source, target })
}

// ---------------------------------------------------------------------------
// Conversion to lang/ (level 1)
// ---------------------------------------------------------------------------

/// Marking of a net: place → token count, encoded as a level-1
/// `Value::Map` (key = `Value::Atom` of the place index,
/// value = `Value::Atom` of the count).
pub type Marquage = BTreeMap<Atom, Atom>;

/// Converts the initial marking of the net into a [`Value::Map`].
pub fn marquage_initial(network: &NetworkPetri) -> Value {
    let map: BTreeMap<Value, Value> = network
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

/// Extracts the [`Marquage`] from a [`Value::Map`] produced by
/// [`marquage_initial`].
pub fn marquage_depuis_value(value: &Value) -> Option<Marquage> {
    match value {
        Value::Map(map) => map
            .iter()
            .map(|(key, count)| match (key, count) {
                (Value::Atom(c), Value::Atom(n)) => Some((*c, *n)),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// Input places of a transition (arcs place → transition).
fn places_entrantes<'a>(network: &'a NetworkPetri, transition: &TransitionId) -> Vec<&'a PlaceId> {
    network
        .arcs
        .iter()
        .filter_map(|arc| match (&arc.source, &arc.target) {
            (NodeId::Place(p), NodeId::Transition(t)) if t == transition => Some(p),
            _ => None,
        })
        .collect()
}

/// Output places of a transition (arcs transition → place).
fn places_sortantes<'a>(network: &'a NetworkPetri, transition: &TransitionId) -> Vec<&'a PlaceId> {
    network
        .arcs
        .iter()
        .filter_map(|arc| match (&arc.source, &arc.target) {
            (NodeId::Transition(t), NodeId::Place(p)) if t == transition => Some(p),
            _ => None,
        })
        .collect()
}

/// A transition is enabled if each input place has at least
/// one token.
pub fn est_sensibilisee(
    network: &NetworkPetri,
    transition: &TransitionId,
    marquage: &Marquage,
) -> bool {
    places_entrantes(network, transition).iter().all(|place| {
        let Some(index) = network.index_place(place) else {
            return false;
        };
        marquage
            .get(&Atom(index as u64))
            .map(|jetons| jetons.0 >= 1)
            .unwrap_or(false)
    })
}

/// Fires a transition: removes one token from each input place,
/// adds one token to each output place.
pub fn tirer(network: &NetworkPetri, transition: &TransitionId, marquage: &Marquage) -> Marquage {
    let mut result = marquage.clone();
    for place in places_entrantes(network, transition) {
        if let Some(index) = network.index_place(place) {
            let key = Atom(index as u64);
            if let Some(jetons) = result.get_mut(&key) {
                jetons.0 = jetons.0.saturating_sub(1);
            }
        }
    }
    for place in places_sortantes(network, transition) {
        if let Some(index) = network.index_place(place) {
            let key = Atom(index as u64);
            result
                .entry(key)
                .and_modify(|jetons| jetons.0 += 1)
                .or_insert(Atom(1));
        }
    }
    result
}

/// Semantic step: all successor markings of all enabled
/// transitions, encoded as `Value::Map` (path A).
pub fn pas_semantique(network: &NetworkPetri, set: &BTreeSet<Value>) -> BTreeSet<Value> {
    let mut successeurs = BTreeSet::new();
    for value in set {
        let Some(marquage) = marquage_depuis_value(value) else {
            continue;
        };
        for transition in &network.transitions {
            if est_sensibilisee(network, &transition.identifiant, &marquage) {
                let next = tirer(network, &transition.identifiant, &marquage);
                let map: BTreeMap<Value, Value> = next
                    .into_iter()
                    .map(|(key, count)| (Value::Atom(key), Value::Atom(count)))
                    .collect();
                successeurs.insert(Value::Map(map));
            }
        }
    }
    successeurs
}

/// Reachable marking space (path A, increasing fixpoint over
/// the reference `lang/` interpreter).
pub fn space_atteignable(network: &NetworkPetri) -> BTreeSet<Value> {
    let initial = BTreeSet::from([marquage_initial(network)]);
    lisage_lang::interpreter::saturate(|set| pas_semantique(network, set), initial)
}
