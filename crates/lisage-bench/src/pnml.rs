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

/// PNML `page` element (MCC models nest the net content in pages).
const ELEMENT_PAGE: &str = "page";
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
const ELEMENT_MARKING: &str = "initialMarking";
/// Accepted arc weight (weights > 1 are outside the subset).
const WEIGHT_ACCEPTED: &str = "1";

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
    AttributeMissing {
        element: &'static str,
        attribute: &'static str,
    },
    /// Invalid numeric value.
    NumberInvalid { element: &'static str, text: String },
    /// Arc whose one end does not exist in the net.
    ArcToUnknown { source: String, target: String },
    /// Badly oriented arc: both ends are places, or two
    /// transitions (a PNML arc links a place and a transition).
    MisorientedArc { source: String, target: String },
    /// Arc weight outside the subset (only weight 1 is supported).
    WeightNonSupported { weight: String },
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
            PnmlError::AttributeMissing { element, attribute } => {
                write!(formatter, "attribute {attribute} missing on <{element}>")
            }
            PnmlError::NumberInvalid { element, text } => {
                write!(formatter, "invalid number in <{element}>: {text:?}")
            }
            PnmlError::ArcToUnknown { source, target } => {
                write!(formatter, "arc from {source:?} to {target:?} unknown")
            }
            PnmlError::MisorientedArc { source, target } => {
                write!(
                    formatter,
                    "badly oriented arc from {source:?} to {target:?} (place↔transition expected)"
                )
            }
            PnmlError::WeightNonSupported { weight } => {
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
        name: String,
        attributes: BTreeMap<String, String>,
        children: Vec<XmlNode>,
    },
    Text(String),
}

/// Parses an XML document into a [`XmlNode`] tree.
fn parser_xml(document: &str) -> Result<XmlNode, PnmlError> {
    let bytes: Vec<char> = document.chars().collect();
    let mut position = 0;
    let root = parser_element(&bytes, &mut position)?;
    sauter_spaces(&bytes, &mut position);
    if position != bytes.len() {
        return Err(PnmlError::XmlMalformed {
            position,
            detail: "content after the root element".to_string(),
        });
    }
    Ok(root)
}

fn sauter_spaces(bytes: &[char], position: &mut usize) {
    while *position < bytes.len() && bytes[*position].is_whitespace() {
        *position += 1;
    }
}

/// Reads an XML name (alphanumeric + `_ - : .`).
fn read_name(bytes: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let start = *position;
    while *position < bytes.len()
        && (bytes[*position].is_alphanumeric() || matches!(bytes[*position], '_' | '-' | ':' | '.'))
    {
        *position += 1;
    }
    if *position == start {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "name expected".to_string(),
        });
    }
    Ok(bytes[start..*position].iter().collect())
}

/// Reads a string between single or double quotes.
fn read_string(bytes: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let Some(&delimiteur) = bytes.get(*position) else {
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
    let start = *position;
    while *position < bytes.len() && bytes[*position] != delimiteur {
        *position += 1;
    }
    if *position >= bytes.len() {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "unterminated string".to_string(),
        });
    }
    let value: String = bytes[start..*position].iter().collect();
    *position += 1;
    Ok(value)
}

fn attendre(bytes: &[char], position: &mut usize, expected: char) -> Result<(), PnmlError> {
    match bytes.get(*position) {
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
fn sauter_jusqu_a(bytes: &[char], position: &mut usize, target: char) {
    while *position < bytes.len() && bytes[*position] != target {
        *position += 1;
    }
    if *position < bytes.len() {
        *position += 1;
    }
}

/// Reads the opening tag `<name attr="v" …>` or `<name …/>`.
/// Returns `(name, attributes, is_self_closing)`.
fn read_opening_tag(
    bytes: &[char],
    position: &mut usize,
) -> Result<(String, BTreeMap<String, String>, bool), PnmlError> {
    *position += 1; // '<'
    let name = read_name(bytes, position)?;
    let mut attributes = BTreeMap::new();
    loop {
        sauter_spaces(bytes, position);
        let Some(&c) = bytes.get(*position) else {
            return Err(PnmlError::XmlMalformed {
                position: *position,
                detail: format!("unclosed tag <{name}>"),
            });
        };
        match c {
            '/' => {
                *position += 1;
                attendre(bytes, position, '>')?;
                return Ok((name, attributes, true));
            }
            '>' => {
                *position += 1;
                return Ok((name, attributes, false));
            }
            _ => {
                let key = read_name(bytes, position)?;
                sauter_spaces(bytes, position);
                attendre(bytes, position, '=')?;
                sauter_spaces(bytes, position);
                let value = read_string(bytes, position)?;
                attributes.insert(key, value);
            }
        }
    }
}

/// Reads a text chunk up to the next '<'.
fn read_text(bytes: &[char], position: &mut usize) -> String {
    let start = *position;
    while *position < bytes.len() && bytes[*position] != '<' {
        *position += 1;
    }
    bytes[start..*position]
        .iter()
        .collect::<String>()
        .trim()
        .to_string()
}

/// Reads the closing tag `</name>` and checks the match.
fn read_closing_tag(bytes: &[char], position: &mut usize, name: &str) -> Result<(), PnmlError> {
    *position += 2; // '</'
    let name_closing = read_name(bytes, position)?;
    if name_closing != name {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: format!("closing </{name_closing}> for <{name}>"),
        });
    }
    attendre(bytes, position, '>')
}

/// Parses a complete element (recursive).
fn parser_element(bytes: &[char], position: &mut usize) -> Result<XmlNode, PnmlError> {
    sauter_spaces(bytes, position);
    if bytes.get(*position) != Some(&'<') {
        return Err(PnmlError::XmlMalformed {
            position: *position,
            detail: "element expected".to_string(),
        });
    }
    // Prologue or comment: skip, retry.
    if matches!(bytes.get(*position + 1), Some('?') | Some('!')) {
        sauter_jusqu_a(bytes, position, '>');
        return parser_element(bytes, position);
    }
    let (name, attributes, self_closing) = read_opening_tag(bytes, position)?;
    if self_closing {
        return Ok(XmlNode::Element {
            name,
            attributes,
            children: Vec::new(),
        });
    }
    let children = parser_children(bytes, position, &name)?;
    Ok(XmlNode::Element {
        name,
        attributes,
        children,
    })
}

/// Parses the children of an element up to its closing tag.
fn parser_children(
    bytes: &[char],
    position: &mut usize,
    name: &str,
) -> Result<Vec<XmlNode>, PnmlError> {
    let mut children = Vec::new();
    loop {
        sauter_spaces(bytes, position);
        let Some(&c) = bytes.get(*position) else {
            return Err(PnmlError::XmlMalformed {
                position: *position,
                detail: format!("end of document inside <{name}>"),
            });
        };
        if c == '<' {
            if bytes.get(*position + 1) == Some(&'/') {
                read_closing_tag(bytes, position, name)?;
                return Ok(children);
            }
            children.push(parser_element(bytes, position)?);
        } else {
            let text = read_text(bytes, position);
            if !text.is_empty() {
                children.push(XmlNode::Text(text));
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
    pub identifier: PlaceId,
    pub name: Option<String>,
    pub marking_initial: u64,
}

/// Transition of a Petri net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub identifier: TransitionId,
    pub name: Option<String>,
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
    pub fn from_pnml(document: &PnmlDocument) -> Result<NetworkPetri, PnmlError> {
        let root = parser_xml(&document.0)?;
        let XmlNode::Element { name, children, .. } = &root else {
            return Err(PnmlError::ElementInexpected {
                expected: ELEMENT_PNML,
                trouve: "text".to_string(),
            });
        };
        if name != ELEMENT_PNML {
            return Err(PnmlError::ElementInexpected {
                expected: ELEMENT_PNML,
                trouve: name.clone(),
            });
        }
        let net = children
            .iter()
            .find_map(|child| match child {
                XmlNode::Element { name, children, .. } if name == ELEMENT_NET => Some(children),
                _ => None,
            })
            .ok_or_else(|| PnmlError::ElementInexpected {
                expected: ELEMENT_NET,
                trouve: "nothing".to_string(),
            })?;
        let mut places = Vec::new();
        let mut transitions = Vec::new();
        let mut arcs_bruts = Vec::new();
        // MCC models nest the places/transitions/arcs inside `<page>`
        // elements: the pages are flattened (single-page nets are the
        // common case).
        let mut elements: Vec<&XmlNode> = Vec::new();
        for child in net {
            if let XmlNode::Element { name, children, .. } = child
                && name == ELEMENT_PAGE
            {
                elements.extend(children.iter());
                continue;
            }
            elements.push(child);
        }
        for child in elements {
            let XmlNode::Element {
                name,
                attributes,
                children,
            } = child
            else {
                continue;
            };
            match name.as_str() {
                ELEMENT_PLACE => places.push(parser_place(attributes, children)?),
                ELEMENT_TRANSITION => transitions.push(parser_transition(attributes, children)?),
                ELEMENT_ARC => arcs_bruts.push(parser_arc(attributes)?),
                _ => {}
            }
        }
        let arcs = resolve_arcs(&places, &transitions, &arcs_bruts)?;
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
    pub fn index_place(&self, identifier: &PlaceId) -> Option<usize> {
        self.places.iter().position(|p| &p.identifier == identifier)
    }
}

/// Raw arc as parsed (identifiers not yet resolved).
struct ArcRaw {
    source: String,
    target: String,
}

/// Resolves arc identifiers into [`NodeId`] and checks that each
/// arc links a place and a transition whose both ends
/// exist.
fn resolve_arcs(
    places: &[Place],
    transitions: &[Transition],
    arcs_bruts: &[ArcRaw],
) -> Result<Vec<Arc>, PnmlError> {
    arcs_bruts
        .iter()
        .map(|raw| resolve_arc(places, transitions, raw))
        .collect()
}

fn resolve_arc(
    places: &[Place],
    transitions: &[Transition],
    raw: &ArcRaw,
) -> Result<Arc, PnmlError> {
    let source = resolve_node(places, transitions, &raw.source);
    let target = resolve_node(places, transitions, &raw.target);
    let (source, target) = match (source, target) {
        (Some(source), Some(target)) => (source, target),
        _ => {
            return Err(PnmlError::ArcToUnknown {
                source: raw.source.clone(),
                target: raw.target.clone(),
            });
        }
    };
    let badly_oriented = matches!(
        (&source, &target),
        (NodeId::Place(_), NodeId::Place(_)) | (NodeId::Transition(_), NodeId::Transition(_))
    );
    if badly_oriented {
        return Err(PnmlError::MisorientedArc {
            source: raw.source.clone(),
            target: raw.target.clone(),
        });
    }
    Ok(Arc { source, target })
}

fn resolve_node(places: &[Place], transitions: &[Transition], identifier: &str) -> Option<NodeId> {
    if places.iter().any(|p| p.identifier.0 == identifier) {
        return Some(NodeId::Place(PlaceId(identifier.to_string())));
    }
    if transitions.iter().any(|t| t.identifier.0 == identifier) {
        return Some(NodeId::Transition(TransitionId(identifier.to_string())));
    }
    None
}

fn attribute_obligatoire(
    attributes: &BTreeMap<String, String>,
    element: &'static str,
    name: &'static str,
) -> Result<String, PnmlError> {
    attributes
        .get(name)
        .cloned()
        .ok_or(PnmlError::AttributeMissing {
            element,
            attribute: name,
        })
}

/// Text of a `<name><text>…</text></name>` or
/// `<initialMarking><text>…</text></initialMarking>` element: the text is
/// either a direct child, or inside a `<text>` sub-element.
fn read_text_child(children: &[XmlNode], element: &str) -> Option<String> {
    for child in children {
        let XmlNode::Element { name, children, .. } = child else {
            continue;
        };
        if name != element {
            continue;
        }
        return children.iter().find_map(text_direct_or_in_text);
    }
    None
}

fn text_direct_or_in_text(node: &XmlNode) -> Option<String> {
    match node {
        XmlNode::Text(text) => Some(text.clone()),
        XmlNode::Element { name, children, .. } if name == ELEMENT_TEXT => {
            children.iter().find_map(|e| match e {
                XmlNode::Text(text) => Some(text.clone()),
                _ => None,
            })
        }
        _ => None,
    }
}

fn read_name_optionnel(children: &[XmlNode]) -> Option<String> {
    read_text_child(children, ELEMENT_NAME)
}

fn parser_place(
    attributes: &BTreeMap<String, String>,
    children: &[XmlNode],
) -> Result<Place, PnmlError> {
    let identifier = PlaceId(attribute_obligatoire(
        attributes,
        ELEMENT_PLACE,
        ATTRIBUTE_ID,
    )?);
    let name = read_name_optionnel(children);
    let marking_initial = match read_text_child(children, ELEMENT_MARKING) {
        Some(text) => text.parse::<u64>().map_err(|_| PnmlError::NumberInvalid {
            element: ELEMENT_MARKING,
            text,
        })?,
        None => 0,
    };
    Ok(Place {
        identifier,
        name,
        marking_initial,
    })
}

fn parser_transition(
    attributes: &BTreeMap<String, String>,
    children: &[XmlNode],
) -> Result<Transition, PnmlError> {
    let identifier = TransitionId(attribute_obligatoire(
        attributes,
        ELEMENT_TRANSITION,
        ATTRIBUTE_ID,
    )?);
    let name = read_name_optionnel(children);
    Ok(Transition { identifier, name })
}

fn parser_arc(attributes: &BTreeMap<String, String>) -> Result<ArcRaw, PnmlError> {
    if let Some(weight) = attributes.get(ATTRIBUTE_WEIGHT)
        && weight != WEIGHT_ACCEPTED
    {
        return Err(PnmlError::WeightNonSupported {
            weight: weight.clone(),
        });
    }
    let source = attribute_obligatoire(attributes, ELEMENT_ARC, ATTRIBUTE_SOURCE)?;
    let target = attribute_obligatoire(attributes, ELEMENT_ARC, ATTRIBUTE_TARGET)?;
    Ok(ArcRaw { source, target })
}

// ---------------------------------------------------------------------------
// Conversion to lang/ (level 1)
// ---------------------------------------------------------------------------

/// Marking of a net: place → token count, encoded as a level-1
/// `Value::Map` (key = `Value::Atom` of the place index,
/// value = `Value::Atom` of the count).
pub type Marking = BTreeMap<Atom, Atom>;

/// Converts the initial marking of the net into a [`Value::Map`].
pub fn marking_initial(network: &NetworkPetri) -> Value {
    let map: BTreeMap<Value, Value> = network
        .places
        .iter()
        .enumerate()
        .map(|(index, place)| {
            (
                Value::Atom(Atom(index as u64)),
                Value::Atom(Atom(place.marking_initial)),
            )
        })
        .collect();
    Value::Map(map)
}

/// Extracts the [`Marking`] from a [`Value::Map`] produced by
/// [`marking_initial`].
pub fn marking_from_value(value: &Value) -> Option<Marking> {
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
pub fn is_sensitized(network: &NetworkPetri, transition: &TransitionId, marking: &Marking) -> bool {
    places_entrantes(network, transition).iter().all(|place| {
        let Some(index) = network.index_place(place) else {
            return false;
        };
        marking
            .get(&Atom(index as u64))
            .map(|tokens| tokens.0 >= 1)
            .unwrap_or(false)
    })
}

/// Fires a transition: removes one token from each input place,
/// adds one token to each output place.
pub fn fire(network: &NetworkPetri, transition: &TransitionId, marking: &Marking) -> Marking {
    let mut result = marking.clone();
    for place in places_entrantes(network, transition) {
        if let Some(index) = network.index_place(place) {
            let key = Atom(index as u64);
            if let Some(tokens) = result.get_mut(&key) {
                tokens.0 = tokens.0.saturating_sub(1);
            }
        }
    }
    for place in places_sortantes(network, transition) {
        if let Some(index) = network.index_place(place) {
            let key = Atom(index as u64);
            result
                .entry(key)
                .and_modify(|tokens| tokens.0 += 1)
                .or_insert(Atom(1));
        }
    }
    result
}

/// Semantic step: all successor markings of all enabled
/// transitions, encoded as `Value::Map` (path A).
pub fn step_semantic(network: &NetworkPetri, set: &BTreeSet<Value>) -> BTreeSet<Value> {
    let mut successeurs = BTreeSet::new();
    for value in set {
        let Some(marking) = marking_from_value(value) else {
            continue;
        };
        for transition in &network.transitions {
            if is_sensitized(network, &transition.identifier, &marking) {
                let next = fire(network, &transition.identifier, &marking);
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
pub fn space_reachable(network: &NetworkPetri) -> BTreeSet<Value> {
    let initial = BTreeSet::from([marking_initial(network)]);
    lisage_lang::interpreter::saturate(|set| step_semantic(network, set), initial)
}
