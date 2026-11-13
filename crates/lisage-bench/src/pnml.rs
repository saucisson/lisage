//! PNML parser and conversion to `lang/` (M5-2, statements C3/C4).
//!
//! This is an **external format** parser (PNML, the Petri net exchange
//! standard of the Model Checking Contest), not the Lisage language: it
//! produces level-1 [`Value`]s, consumable by the reference interpreter
//! (lane A of the triptych). No external dependency: the XML is parsed by
//! hand, read-only.
//!
//! Supported PNML subset (sufficient for the usual MCC "place/transition"
//! models): `<pnml><net><place id><initialMarking>
//! <text>n</text>`, `<transition id>`, `<edge id source target>` with
//! `source`/`target` each referencing a place and a transition (in either
//! direction). The name annotations (`<name><text>…</text></name>`) are
//! read when present. An edge carrying a `weight` attribute other than `1`
//! is rejected. XML entities (`&amp;` etc.) are not decoded — acceptable
//! for MCC PT-net models.

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
const ELEMENT_EDGE: &str = "edge";
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
/// Accepted edge weight (weights > 1 are outside the subset).
const POIDS_ACCEPTE: &str = "1";

// ---------------------------------------------------------------------------
// Distinct types for node identifiers (ADR-0006)
// ---------------------------------------------------------------------------

/// Identifier of a place, distinct type at the boundary.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PlaceId(pub String);

/// Identifier of a transition, distinct type at the boundary.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TransitionId(pub String);

/// End of an edge: always a node of the network.
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
    XmlBadForm { position: usize, detail: String },
    /// Unexpected PNML element at this point.
    ElementUnexpected {
        expected: &'static str,
        trouve: String,
    },
    /// Mandatory attribute missing.
    AttributeMissing {
        element: &'static str,
        attribute: &'static str,
    },
    /// Invalid numeric value.
    NumberInvalid { element: &'static str, text: String },
    /// Arc whose end does not exist in the network.
    EdgeTowardUnknown { source: String, target: String },
    /// Badly oriented edge: both ends are places, or both are transitions
    /// (a PNML edge links a place and a transition).
    EdgeBadOriented { source: String, target: String },
    /// Arc weight outside the subset (only weight 1 is supported).
    WeightNotSupporte { weight: String },
}

impl std::fmt::Display for PnmlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PnmlError::XmlBadForm { position, detail } => {
                write!(formatter, "malformed XML at position {position}: {detail}")
            }
            PnmlError::ElementUnexpected { expected, trouve } => {
                write!(formatter, "expected element {expected}, found <{trouve}>")
            }
            PnmlError::AttributeMissing { element, attribute } => {
                write!(formatter, "attribute {attribute} missing on <{element}>")
            }
            PnmlError::NumberInvalid { element, text } => {
                write!(formatter, "invalid number in <{element}>: {text:?}")
            }
            PnmlError::EdgeTowardUnknown { source, target } => {
                write!(formatter, "edge from {source:?} to unknown {target:?}")
            }
            PnmlError::EdgeBadOriented { source, target } => {
                write!(
                    formatter,
                    "badly oriented edge from {source:?} to {target:?} (place↔transition expected)"
                )
            }
            PnmlError::WeightNotSupporte { weight } => {
                write!(
                    formatter,
                    "edge weight {weight:?} not supported (weight 1 only)"
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
    let octets: Vec<char> = document.chars().collect();
    let mut position = 0;
    let root = parser_element(&octets, &mut position)?;
    skip_spaces(&octets, &mut position);
    if position != octets.len() {
        return Err(PnmlError::XmlBadForm {
            position,
            detail: "content after the root element".to_string(),
        });
    }
    Ok(root)
}

fn skip_spaces(octets: &[char], position: &mut usize) {
    while *position < octets.len() && octets[*position].is_whitespace() {
        *position += 1;
    }
}

/// Reads an XML name (alphanumeric + `_ - : .`).
fn read_name(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let start = *position;
    while *position < octets.len()
        && (octets[*position].is_alphanumeric()
            || matches!(octets[*position], '_' | '-' | ':' | '.'))
    {
        *position += 1;
    }
    if *position == start {
        return Err(PnmlError::XmlBadForm {
            position: *position,
            detail: "name expected".to_string(),
        });
    }
    Ok(octets[start..*position].iter().collect())
}

/// Reads a string between single or double quotes.
fn read_string(octets: &[char], position: &mut usize) -> Result<String, PnmlError> {
    let Some(&delimiteur) = octets.get(*position) else {
        return Err(PnmlError::XmlBadForm {
            position: *position,
            detail: "quoted string expected".to_string(),
        });
    };
    if delimiteur != '"' && delimiteur != '\'' {
        return Err(PnmlError::XmlBadForm {
            position: *position,
            detail: "quoted string expected".to_string(),
        });
    }
    *position += 1;
    let start = *position;
    while *position < octets.len() && octets[*position] != delimiteur {
        *position += 1;
    }
    if *position >= octets.len() {
        return Err(PnmlError::XmlBadForm {
            position: *position,
            detail: "unterminated string".to_string(),
        });
    }
    let value: String = octets[start..*position].iter().collect();
    *position += 1;
    Ok(value)
}

fn expect_char(octets: &[char], position: &mut usize, expected: char) -> Result<(), PnmlError> {
    match octets.get(*position) {
        Some(&c) if c == expected => {
            *position += 1;
            Ok(())
        }
        _ => Err(PnmlError::XmlBadForm {
            position: *position,
            detail: format!("'{expected}' expected"),
        }),
    }
}

/// Skips to the next `>` (prologue `<?…?>`, comments, truncated CDATA —
/// entities are not decoded, see the module doc).
fn skip_until_a(octets: &[char], position: &mut usize, target: char) {
    while *position < octets.len() && octets[*position] != target {
        *position += 1;
    }
    if *position < octets.len() {
        *position += 1;
    }
}

/// Reads the opening tag `<name attribute="v" …>` or `<name …/>`.
/// Returns `(name, attributes, is_auto_closing)`.
fn read_balise_opening(
    octets: &[char],
    position: &mut usize,
) -> Result<(String, BTreeMap<String, String>, bool), PnmlError> {
    *position += 1; // '<'
    let name = read_name(octets, position)?;
    let mut attributes = BTreeMap::new();
    loop {
        skip_spaces(octets, position);
        let Some(&c) = octets.get(*position) else {
            return Err(PnmlError::XmlBadForm {
                position: *position,
                detail: format!("unclosed tag <{name}>"),
            });
        };
        match c {
            '/' => {
                *position += 1;
                expect_char(octets, position, '>')?;
                return Ok((name, attributes, true));
            }
            '>' => {
                *position += 1;
                return Ok((name, attributes, false));
            }
            _ => {
                let key = read_name(octets, position)?;
                skip_spaces(octets, position);
                expect_char(octets, position, '=')?;
                skip_spaces(octets, position);
                let value = read_string(octets, position)?;
                attributes.insert(key, value);
            }
        }
    }
}

/// Reads a text chunk up to the next '<'.
fn read_text(octets: &[char], position: &mut usize) -> String {
    let start = *position;
    while *position < octets.len() && octets[*position] != '<' {
        *position += 1;
    }
    octets[start..*position]
        .iter()
        .collect::<String>()
        .trim()
        .to_string()
}

/// Reads the closing tag `</name>` and checks the match.
fn read_balise_closing(octets: &[char], position: &mut usize, name: &str) -> Result<(), PnmlError> {
    *position += 2; // '</'
    let name_closure = read_name(octets, position)?;
    if name_closure != name {
        return Err(PnmlError::XmlBadForm {
            position: *position,
            detail: format!("closing </{name_closure}> for <{name}>"),
        });
    }
    expect_char(octets, position, '>')
}

/// Parses a complete element (recursive).
fn parser_element(octets: &[char], position: &mut usize) -> Result<XmlNode, PnmlError> {
    skip_spaces(octets, position);
    if octets.get(*position) != Some(&'<') {
        return Err(PnmlError::XmlBadForm {
            position: *position,
            detail: "element expected".to_string(),
        });
    }
    // Prologue or comment: skip it, retry.
    if matches!(octets.get(*position + 1), Some('?') | Some('!')) {
        skip_until_a(octets, position, '>');
        return parser_element(octets, position);
    }
    let (name, attributes, auto_closing) = read_balise_opening(octets, position)?;
    if auto_closing {
        return Ok(XmlNode::Element {
            name,
            attributes,
            children: Vec::new(),
        });
    }
    let children = parser_children(octets, position, &name)?;
    Ok(XmlNode::Element {
        name,
        attributes,
        children,
    })
}

/// Parses the children of an element up to its closing tag.
fn parser_children(
    octets: &[char],
    position: &mut usize,
    name: &str,
) -> Result<Vec<XmlNode>, PnmlError> {
    let mut children = Vec::new();
    loop {
        skip_spaces(octets, position);
        let Some(&c) = octets.get(*position) else {
            return Err(PnmlError::XmlBadForm {
                position: *position,
                detail: format!("end of document inside <{name}>"),
            });
        };
        if c == '<' {
            if octets.get(*position + 1) == Some(&'/') {
                read_balise_closing(octets, position, name)?;
                return Ok(children);
            }
            children.push(parser_element(octets, position)?);
        } else {
            let text = read_text(octets, position);
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

/// Oriented edge place↔transition (weight 1).
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
    pub edges: Vec<Arc>,
}

impl NetworkPetri {
    /// Parses a complete PNML document.
    pub fn since_pnml(document: &PnmlDocument) -> Result<NetworkPetri, PnmlError> {
        let root = parser_xml(&document.0)?;
        let XmlNode::Element { name, children, .. } = &root else {
            return Err(PnmlError::ElementUnexpected {
                expected: ELEMENT_PNML,
                trouve: "text".to_string(),
            });
        };
        if name != ELEMENT_PNML {
            return Err(PnmlError::ElementUnexpected {
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
            .ok_or_else(|| PnmlError::ElementUnexpected {
                expected: ELEMENT_NET,
                trouve: "nothing".to_string(),
            })?;
        let mut places = Vec::new();
        let mut transitions = Vec::new();
        let mut edges_raw = Vec::new();
        for child in net {
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
                ELEMENT_EDGE => edges_raw.push(parser_edge(attributes)?),
                _ => {}
            }
        }
        let edges = solve_edges(&places, &transitions, &edges_raw)?;
        Ok(NetworkPetri {
            places,
            transitions,
            edges,
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

/// Raw edge as parsed (identifiers not yet resolved).
struct EdgeRaw {
    source: String,
    target: String,
}

/// Resolves edge identifiers into [`NodeId`] and checks that each edge
/// links a place and a transition whose both ends exist.
fn solve_edges(
    places: &[Place],
    transitions: &[Transition],
    edges_raw: &[EdgeRaw],
) -> Result<Vec<Arc>, PnmlError> {
    edges_raw
        .iter()
        .map(|raw| solve_edge(places, transitions, raw))
        .collect()
}

fn solve_edge(
    places: &[Place],
    transitions: &[Transition],
    raw: &EdgeRaw,
) -> Result<Arc, PnmlError> {
    let source = solve_node(places, transitions, &raw.source);
    let target = solve_node(places, transitions, &raw.target);
    let (source, target) = match (source, target) {
        (Some(source), Some(target)) => (source, target),
        _ => {
            return Err(PnmlError::EdgeTowardUnknown {
                source: raw.source.clone(),
                target: raw.target.clone(),
            });
        }
    };
    let bad_oriented = matches!(
        (&source, &target),
        (NodeId::Place(_), NodeId::Place(_)) | (NodeId::Transition(_), NodeId::Transition(_))
    );
    if bad_oriented {
        return Err(PnmlError::EdgeBadOriented {
            source: raw.source.clone(),
            target: raw.target.clone(),
        });
    }
    Ok(Arc { source, target })
}

fn solve_node(places: &[Place], transitions: &[Transition], identifier: &str) -> Option<NodeId> {
    if places.iter().any(|p| p.identifier.0 == identifier) {
        return Some(NodeId::Place(PlaceId(identifier.to_string())));
    }
    if transitions.iter().any(|t| t.identifier.0 == identifier) {
        return Some(NodeId::Transition(TransitionId(identifier.to_string())));
    }
    None
}

fn attribute_required(
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
/// either a direct child or inside a `<text>` sub-element.
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

fn read_name_optional(children: &[XmlNode]) -> Option<String> {
    read_text_child(children, ELEMENT_NAME)
}

fn parser_place(
    attributes: &BTreeMap<String, String>,
    children: &[XmlNode],
) -> Result<Place, PnmlError> {
    let identifier = PlaceId(attribute_required(attributes, ELEMENT_PLACE, ATTRIBUTE_ID)?);
    let name = read_name_optional(children);
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
    let identifier = TransitionId(attribute_required(
        attributes,
        ELEMENT_TRANSITION,
        ATTRIBUTE_ID,
    )?);
    let name = read_name_optional(children);
    Ok(Transition { identifier, name })
}

fn parser_edge(attributes: &BTreeMap<String, String>) -> Result<EdgeRaw, PnmlError> {
    if let Some(weight) = attributes.get(ATTRIBUTE_WEIGHT)
        && weight != POIDS_ACCEPTE
    {
        return Err(PnmlError::WeightNotSupporte {
            weight: weight.clone(),
        });
    }
    let source = attribute_required(attributes, ELEMENT_EDGE, ATTRIBUTE_SOURCE)?;
    let target = attribute_required(attributes, ELEMENT_EDGE, ATTRIBUTE_TARGET)?;
    Ok(EdgeRaw { source, target })
}

// ---------------------------------------------------------------------------
// Conversion to lang/ (level 1)
// ---------------------------------------------------------------------------

/// Marking of a network: place → token count, encoded as a level-1
/// `Value::Map` (key = `Value::Atom` of the place index, value =
/// `Value::Atom` of the count).
pub type Marking = BTreeMap<Atom, Atom>;

/// Converts the initial marking of the network into a [`Value::Map`].
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
pub fn marking_since_value(value: &Value) -> Option<Marking> {
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

/// Incoming places of a transition (edges place → transition).
fn places_incoming<'a>(network: &'a NetworkPetri, transition: &TransitionId) -> Vec<&'a PlaceId> {
    network
        .edges
        .iter()
        .filter_map(|edge| match (&edge.source, &edge.target) {
            (NodeId::Place(p), NodeId::Transition(t)) if t == transition => Some(p),
            _ => None,
        })
        .collect()
}

/// Outgoing places of a transition (edges transition → place).
fn places_outgoing<'a>(network: &'a NetworkPetri, transition: &TransitionId) -> Vec<&'a PlaceId> {
    network
        .edges
        .iter()
        .filter_map(|edge| match (&edge.source, &edge.target) {
            (NodeId::Transition(t), NodeId::Place(p)) if t == transition => Some(p),
            _ => None,
        })
        .collect()
}

/// A transition is enabled if each incoming place has at least one token.
pub fn is_sensitive(network: &NetworkPetri, transition: &TransitionId, marking: &Marking) -> bool {
    places_incoming(network, transition).iter().all(|place| {
        let Some(index) = network.index_place(place) else {
            return false;
        };
        marking
            .get(&Atom(index as u64))
            .map(|tokens| tokens.0 >= 1)
            .unwrap_or(false)
    })
}

/// Fires a transition: removes one token from each incoming place, adds
/// one token to each outgoing place.
pub fn fire(network: &NetworkPetri, transition: &TransitionId, marking: &Marking) -> Marking {
    let mut result = marking.clone();
    for place in places_incoming(network, transition) {
        if let Some(index) = network.index_place(place) {
            let key = Atom(index as u64);
            if let Some(tokens) = result.get_mut(&key) {
                tokens.0 = tokens.0.saturating_sub(1);
            }
        }
    }
    for place in places_outgoing(network, transition) {
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

/// Step semantics: all successor markings of all enabled transitions,
/// encoded as `Value::Map` (lane A).
pub fn not_semantic(network: &NetworkPetri, set: &BTreeSet<Value>) -> BTreeSet<Value> {
    let mut successors = BTreeSet::new();
    for value in set {
        let Some(marking) = marking_since_value(value) else {
            continue;
        };
        for transition in &network.transitions {
            if is_sensitive(network, &transition.identifier, &marking) {
                let next = fire(network, &transition.identifier, &marking);
                let map: BTreeMap<Value, Value> = next
                    .into_iter()
                    .map(|(key, count)| (Value::Atom(key), Value::Atom(count)))
                    .collect();
                successors.insert(Value::Map(map));
            }
        }
    }
    successors
}

/// Space of reachable markings (lane A, increasing fixpoint on the
/// reference `lang/` interpreter).
pub fn space_reachable(network: &NetworkPetri) -> BTreeSet<Value> {
    let initial = BTreeSet::from([marking_initial(network)]);
    lisage_lang::interpreter::saturate(|set| not_semantic(network, set), initial)
}
