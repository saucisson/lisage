//! Reference schemas (M1-3, spec § 1 and § 2.1): bounded `Atom(D)`,
//! `Set(T)`, `Map(K, V)`, **universe sweep** compilation (one presence level
//! per key, δ = absent, no marker).
//!
//! Levels: `Atom(D)` = one level `Val = D`; `Set(D)` = one presence level
//! per element (`Val = {absent, present}`, δ = absent); `Map(K,V)`
//! = per key `k_i` a presence level `2i` (δ = absent) then a value level
//! `2i+1` (`Val = V`, δ = ⊥). Compilation: only the chosen arc is interned,
//! the other leads to the carrier default D_δ = terminal(false) on presences
//! (R-collapse skips absents, ZDD style).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_lang::value::{Atom, Value};

/// Presence arc key: absent (default δ) or present.
pub const ABSENT: u64 = 0;
/// Presence arc key: present.
pub const PRESENT: u64 = 1;
/// Cardinality of a presence level: {absent, present}.
pub const CARDINALITE_PRESENCE: u64 = 2;
/// Stride between the levels of a map: presence of key `i` at level `2i`,
/// value at level `2i+1`.
pub const PAS_NIVEAU_MAP: u32 = 2;

/// Bounded atom domain: finite enumeration, with optional default.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Domaine {
    /// Values of the domain, in canonical order.
    pub valeurs: Vec<Atom>,
    /// Default δ of the domain, if declared.
    pub defaut: Option<Atom>,
}

impl Domaine {
    /// Domain without default.
    pub fn nouveau(valeurs: Vec<Atom>) -> Self {
        Domaine {
            valeurs,
            defaut: None,
        }
    }

    /// Does the domain contain this atom?
    pub fn contient(&self, atome: Atom) -> bool {
        self.valeurs.contains(&atome)
    }

    /// Number of values in the domain.
    pub fn taille(&self) -> usize {
        self.valeurs.len()
    }
}

/// Reference schema (bounded type algebra Σ, spec § 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Schema {
    /// `Atom(D)`: bounded domain of atoms.
    Atom(Domaine),
    /// `Set(Atom(D))`: set of atoms from a bounded domain.
    Set(Domaine),
    /// `Map(K, V)`: ordered keys to a bounded value domain.
    Map(Vec<Atom>, Domaine),
    /// `Seq(D, N)`: sequence of at most `N` elements of a bounded domain `D`
    /// (M2-1, `list` constructor). The end marker is a required key.
    Seq(Domaine, u32),
    /// `Tag(K, V)`: typed union — discriminant level (keys `K`) then value
    /// level `V` (M2-2). A single useful arc per node (C2).
    Tag(Vec<Atom>, Domaine),
}

impl Schema {
    /// Number of levels of the universe sweep.
    pub fn nombre_niveaux(&self) -> u32 {
        match self {
            Schema::Atom(_) => 1,
            Schema::Set(domaine) => domaine.taille() as u32,
            Schema::Map(liste_cles, _) => PAS_NIVEAU_MAP * liste_cles.len() as u32,
            Schema::Seq(_, longueur_max) => longueur_max + 1,
            Schema::Tag(liste, _) => PAS_NIVEAU_MAP * liste.len() as u32,
        }
    }

    /// Presence level of the i-th key of a map.
    pub fn niveau_presence(index: usize) -> Level {
        Level(PAS_NIVEAU_MAP * index as u32)
    }

    /// Value level of the i-th key of a map.
    pub fn niveau_valeur(index: usize) -> Level {
        Level(PAS_NIVEAU_MAP * index as u32 + 1)
    }
}

/// Typed error of schema compilation/decompilation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SchemaError {
    /// The value does not match the schema (different Σ shape).
    TypeMismatch {
        /// Shape expected by the schema.
        attendu: &'static str,
        /// Shape received.
        recu: &'static str,
    },
    /// Atom outside the domain declared by the schema.
    HorsDomaine(Atom),
    /// Diagram node incompatible with the schema (decompilation).
    NoeudInattendu {
        /// Shape expected at this position.
        attendu: &'static str,
        /// Node encountered.
        noeud: NodeId,
    },
    /// Error propagated from the pool port.
    Pool(PoolError),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::TypeMismatch { attendu, recu } => {
                write!(formatter, "type mismatch: expected {attendu}, got {recu}")
            }
            SchemaError::HorsDomaine(atome) => write!(formatter, "atom outside domain: {atome:?}"),
            SchemaError::NoeudInattendu { attendu, noeud } => {
                write!(
                    formatter,
                    "unexpected node: expected {attendu}, node {noeud:?}"
                )
            }
            SchemaError::Pool(error) => write!(formatter, "pool error: {error}"),
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SchemaError::Pool(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PoolError> for SchemaError {
    fn from(error: PoolError) -> Self {
        SchemaError::Pool(error)
    }
}

/// Key algebra of a schema: `Val(ℓ)` and δ(ℓ) per level of the sweep.
#[derive(Clone)]
pub struct SchemaKeys {
    schema: Schema,
}

impl SchemaKeys {
    /// Key algebra for the given schema.
    pub fn new(schema: Schema) -> Self {
        SchemaKeys { schema }
    }
}

impl KeyAlgebra for SchemaKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, level: Level) -> u64 {
        match &self.schema {
            Schema::Atom(domaine) => domaine.taille() as u64,
            Schema::Set(_) => CARDINALITE_PRESENCE,
            Schema::Map(_, domaine_valeurs) => {
                if level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    CARDINALITE_PRESENCE
                } else {
                    domaine_valeurs.taille() as u64
                }
            }
            Schema::Seq(domaine, longueur_max) => {
                if level.0 < *longueur_max {
                    domaine.taille() as u64 + 1 // +1 for the end marker
                } else {
                    CARDINALITE_PRESENCE
                }
            }
            Schema::Tag(liste, domaine_valeurs) => {
                if level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    liste.len() as u64
                } else {
                    domaine_valeurs.taille() as u64
                }
            }
        }
    }

    fn default(&self, level: Level) -> Option<Self::Key> {
        match &self.schema {
            Schema::Atom(domaine) => domaine.defaut.map(|atome| atome.0),
            Schema::Set(_) => Some(ABSENT),
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    Some(ABSENT)
                } else {
                    None
                }
            }
            Schema::Seq(_, _) => None, // end marker = required key, no default
            Schema::Tag(_, _) => None, // discriminant = required key
        }
    }
}

/// Controller of the reference instance: carries the schema, designates D_δ =
/// terminal(false) (the empty set) for presence levels and atom domains
/// with default; δ = ⊥ (no default) for value levels (spec § 2.1).
pub struct ReferenceController {
    schema: Schema,
    keys: SchemaKeys,
}

impl ReferenceController {
    /// Controller for the given schema.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        ReferenceController { schema, keys }
    }

    /// Schema of this instance.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl Controller for ReferenceController {
    type Key = u64;
    type Payload = bool;
    type Keys = SchemaKeys;

    const FINGERPRINT: ControllerId = ControllerId(1);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        match &self.schema {
            Schema::Atom(domaine) => domaine.defaut.map(|_| false),
            Schema::Set(_) => Some(false),
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    Some(false)
                } else {
                    None
                }
            }
            Schema::Seq(_, _) => None,
            Schema::Tag(_, _) => None,
        }
    }
}

/// Compiles a value into a diagram (universe sweep); the acceptance terminal
/// has payload `true` (set-theoretic reading).
pub fn compiler<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    schema: &Schema,
    valeur: &Value,
) -> Result<NodeId, SchemaError> {
    match (schema, valeur) {
        (Schema::Atom(domaine), Value::Atom(atome)) => {
            if !domaine.contient(*atome) {
                return Err(SchemaError::HorsDomaine(*atome));
            }
            let terminal = pool.intern(anchor, Form::Terminal(true));
            Ok(pool.intern(
                anchor,
                Form::Node {
                    level: Level(0),
                    arcs: BTreeMap::from([(atome.0, terminal)]),
                },
            ))
        }
        (Schema::Set(domaine), Value::Set(elements)) => {
            for element in elements {
                match element {
                    Value::Atom(atome) if domaine.contient(*atome) => {}
                    Value::Atom(atome) => return Err(SchemaError::HorsDomaine(*atome)),
                    autre => {
                        return Err(SchemaError::TypeMismatch {
                            attendu: "set of atoms",
                            recu: forme_valeur(autre),
                        });
                    }
                }
            }
            compiler_ensemble(pool, anchor, domaine, elements, 0)
        }
        (Schema::Map(liste_cles, domaine_valeurs), Value::Map(map)) => {
            valider_map(liste_cles, domaine_valeurs, map)?;
            compiler_map(pool, anchor, liste_cles, map, 0)
        }
        (Schema::Seq(domaine, longueur_max), Value::Seq(elements)) => {
            valider_seq(domaine, *longueur_max, elements)?;
            compiler_seq(pool, anchor, domaine, *longueur_max, elements)
        }
        (Schema::Tag(liste, domaine_valeurs), Value::Tag(discriminant, payload)) => {
            valider_tag(liste, domaine_valeurs, *discriminant, payload)?;
            compiler_tag(pool, anchor, liste, domaine_valeurs, *discriminant, payload)
        }
        (schema, valeur) => Err(SchemaError::TypeMismatch {
            attendu: forme_schema(schema),
            recu: forme_valeur(valeur),
        }),
    }
}

/// Validates the keys and values of a map against the schema (typed errors,
/// the faulty atom is the one reported).
fn valider_map(
    liste_cles: &[Atom],
    domaine_valeurs: &Domaine,
    map: &BTreeMap<Value, Value>,
) -> Result<(), SchemaError> {
    for (cle, valeur) in map {
        let Value::Atom(atome_cle) = cle else {
            return Err(SchemaError::TypeMismatch {
                attendu: "atomic key",
                recu: forme_valeur(cle),
            });
        };
        if !liste_cles.contains(atome_cle) {
            return Err(SchemaError::HorsDomaine(*atome_cle));
        }
        let Value::Atom(atome_valeur) = valeur else {
            return Err(SchemaError::TypeMismatch {
                attendu: "atomic value",
                recu: forme_valeur(valeur),
            });
        };
        if !domaine_valeurs.contient(*atome_valeur) {
            return Err(SchemaError::HorsDomaine(*atome_valeur));
        }
    }
    Ok(())
}

fn forme_schema(schema: &Schema) -> &'static str {
    match schema {
        Schema::Atom(_) => "atom",
        Schema::Set(_) => "set",
        Schema::Map(_, _) => "map",
        Schema::Seq(_, _) => "sequence",
        Schema::Tag(_, _) => "typed union",
    }
}

fn forme_valeur(valeur: &Value) -> &'static str {
    match valeur {
        Value::Atom(_) => "atom",
        Value::Set(_) => "set",
        Value::Map(_) => "map",
        Value::Seq(_) => "sequence",
        Value::Tag(_, _) => "typed union",
    }
}

fn compiler_ensemble<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    domaine: &Domaine,
    elements: &BTreeSet<Value>,
    index: usize,
) -> Result<NodeId, SchemaError> {
    if index == domaine.taille() {
        return Ok(pool.intern(anchor, Form::Terminal(true)));
    }
    let suite = compiler_ensemble(pool, anchor, domaine, elements, index + 1)?;
    let present = elements.contains(&Value::Atom(domaine.valeurs[index]));
    let arc = if present { PRESENT } else { ABSENT };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(index as u32),
            arcs: BTreeMap::from([(arc, suite)]),
        },
    ))
}

fn compiler_map<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    liste_cles: &[Atom],
    map: &BTreeMap<Value, Value>,
    index: usize,
) -> Result<NodeId, SchemaError> {
    if index == liste_cles.len() {
        return Ok(pool.intern(anchor, Form::Terminal(true)));
    }
    let suite = compiler_map(pool, anchor, liste_cles, map, index + 1)?;
    let cle = Value::Atom(liste_cles[index]);
    let arcs = match map.get(&cle) {
        Some(Value::Atom(valeur)) => {
            let noeud_valeur = pool.intern(
                anchor,
                Form::Node {
                    level: Schema::niveau_valeur(index),
                    arcs: BTreeMap::from([(valeur.0, suite)]),
                },
            );
            BTreeMap::from([(PRESENT, noeud_valeur)])
        }
        Some(autre) => {
            return Err(SchemaError::TypeMismatch {
                attendu: "atomic value",
                recu: forme_valeur(autre),
            });
        }
        None => BTreeMap::from([(ABSENT, suite)]),
    };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Schema::niveau_presence(index),
            arcs,
        },
    ))
}

/// Validates a sequence against the Seq schema (elements in the domain,
/// bounded length).
fn valider_seq(
    domaine: &Domaine,
    longueur_max: u32,
    elements: &[Value],
) -> Result<(), SchemaError> {
    if elements.len() > longueur_max as usize {
        return Err(SchemaError::TypeMismatch {
            attendu: "bounded-length sequence",
            recu: "sequence too long",
        });
    }
    for element in elements {
        match element {
            Value::Atom(atome) if domaine.contient(*atome) => {}
            Value::Atom(atome) => return Err(SchemaError::HorsDomaine(*atome)),
            autre => {
                return Err(SchemaError::TypeMismatch {
                    attendu: "atomic element",
                    recu: forme_valeur(autre),
                });
            }
        }
    }
    Ok(())
}

/// Arc key of the end marker in a sequence (value = domain size).
fn cle_fin(domaine: &Domaine) -> u64 {
    domaine.taille() as u64
}

/// Compiles a sequence by **positions** (universe sweep, M2-1):
/// each position `i` has a level of cardinality `|D|+1` (domain
/// values + end marker). The end marker leads to terminal true.
fn compiler_seq<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    domaine: &Domaine,
    longueur_max: u32,
    elements: &[Value],
) -> Result<NodeId, SchemaError> {
    compiler_seq_position(pool, anchor, domaine, longueur_max, elements, 0)
}

fn compiler_seq_position<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    domaine: &Domaine,
    longueur_max: u32,
    elements: &[Value],
    position: u32,
) -> Result<NodeId, SchemaError> {
    if position >= longueur_max {
        // End-marker level: required key, present = end of sequence.
        let terminal = pool.intern(anchor, Form::Terminal(true));
        return Ok(pool.intern(
            anchor,
            Form::Node {
                level: Level(longueur_max),
                arcs: BTreeMap::from([(PRESENT, terminal)]),
            },
        ));
    }
    let suite = compiler_seq_position(pool, anchor, domaine, longueur_max, elements, position + 1)?;
    let arc = if (position as usize) < elements.len() {
        match &elements[position as usize] {
            Value::Atom(atome) => atome.0,
            _ => unreachable!("validated by valider_seq"),
        }
    } else {
        cle_fin(domaine) // end marker
    };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(position),
            arcs: BTreeMap::from([(arc, suite)]),
        },
    ))
}

/// Validates a tag against the schema (discriminant in the list, payload in the domain).
fn valider_tag(
    liste: &[Atom],
    domaine_valeurs: &Domaine,
    discriminant: Atom,
    payload: &Value,
) -> Result<(), SchemaError> {
    if !liste.contains(&discriminant) {
        return Err(SchemaError::HorsDomaine(discriminant));
    }
    match payload {
        Value::Atom(atome) if domaine_valeurs.contient(*atome) => Ok(()),
        Value::Atom(atome) => Err(SchemaError::HorsDomaine(*atome)),
        autre => Err(SchemaError::TypeMismatch {
            attendu: "atomic payload",
            recu: forme_valeur(autre),
        }),
    }
}

/// Compiles a tag: discriminant level (one arc = one variant) then value
/// level. A single useful arc per node (C2).
fn compiler_tag<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    liste: &[Atom],
    domaine_valeurs: &Domaine,
    discriminant: Atom,
    payload: &Value,
) -> Result<NodeId, SchemaError> {
    let terminal = pool.intern(anchor, Form::Terminal(true));
    let Value::Atom(atome_valeur) = payload else {
        return Err(SchemaError::TypeMismatch {
            attendu: "atomic payload",
            recu: forme_valeur(payload),
        });
    };
    let noeud_valeur = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(atome_valeur.0, terminal)]),
        },
    );
    let index = liste.iter().position(|a| *a == discriminant).unwrap();
    let _ = domaine_valeurs;
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(index as u64, noeud_valeur)]),
        },
    ))
}

/// Decompiles a diagram into a value (inverse of [`compiler`]): useful for
/// round-trip tests and diagnostics.
pub fn decompiler<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    noeud: NodeId,
) -> Result<Value, SchemaError> {
    match schema {
        Schema::Atom(domaine) => {
            // Single-element domain: R-collapse reduces the atomic node
            // to the acceptance terminal.
            if matches!(pool.resolve(noeud), Ok(Form::Terminal(true))) {
                return match domaine.valeurs.first() {
                    Some(atome) if domaine.taille() == 1 => Ok(Value::Atom(*atome)),
                    _ => Err(SchemaError::NoeudInattendu {
                        attendu: "atomic-level node",
                        noeud,
                    }),
                };
            }
            let Form::Node { level, arcs } = pool.resolve(noeud)? else {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "atomic-level node",
                    noeud,
                });
            };
            if *level != Level(0) {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "level 0",
                    noeud,
                });
            }
            for (cle, cible) in arcs {
                if matches!(pool.resolve(*cible), Ok(Form::Terminal(true))) {
                    let atome = Atom(*cle);
                    if domaine.contient(atome) {
                        return Ok(Value::Atom(atome));
                    }
                    return Err(SchemaError::HorsDomaine(atome));
                }
            }
            Err(SchemaError::NoeudInattendu {
                attendu: "arc to terminal true",
                noeud,
            })
        }
        Schema::Set(domaine) => {
            let mut elements = BTreeSet::new();
            decompiler_ensemble(pool, domaine, noeud, &mut elements)?;
            Ok(Value::Set(elements))
        }
        Schema::Map(liste_cles, _) => {
            let mut map = BTreeMap::new();
            decompiler_map(pool, liste_cles, noeud, &mut map)?;
            Ok(Value::Map(map))
        }
        Schema::Seq(domaine, longueur_max) => {
            let mut elements = Vec::new();
            decompiler_seq(pool, domaine, *longueur_max, noeud, &mut elements)?;
            Ok(Value::Seq(elements))
        }
        Schema::Tag(liste, domaine_valeurs) => decompiler_tag(pool, liste, domaine_valeurs, noeud),
    }
}

fn decompiler_tag<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    liste: &[Atom],
    domaine_valeurs: &Domaine,
    noeud: NodeId,
) -> Result<Value, SchemaError> {
    let Form::Node { level, arcs } = pool.resolve(noeud)? else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "discriminant node",
            noeud,
        });
    };
    if *level != Level(0) {
        return Err(SchemaError::NoeudInattendu {
            attendu: "discriminant level (0)",
            noeud,
        });
    }
    let Some((&index, &noeud_valeur)) = arcs.iter().next() else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "discriminant arc",
            noeud,
        });
    };
    let discriminant = *liste
        .get(index as usize)
        .ok_or(SchemaError::NoeudInattendu {
            attendu: "valid discriminant index",
            noeud,
        })?;
    let Form::Node {
        level: niveau_valeur,
        arcs: arcs_valeur,
    } = pool.resolve(noeud_valeur)?
    else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "value node",
            noeud: noeud_valeur,
        });
    };
    if *niveau_valeur != Level(1) {
        return Err(SchemaError::NoeudInattendu {
            attendu: "value level (1)",
            noeud: noeud_valeur,
        });
    }
    let Some((&valeur, _)) = arcs_valeur.iter().next() else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "value arc",
            noeud: noeud_valeur,
        });
    };
    let atome = Atom(valeur);
    if !domaine_valeurs.contient(atome) {
        return Err(SchemaError::HorsDomaine(atome));
    }
    Ok(Value::Tag(discriminant, Box::new(Value::Atom(atome))))
}

fn decompiler_seq<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    domaine: &Domaine,
    longueur_max: u32,
    mut noeud: NodeId,
    elements: &mut Vec<Value>,
) -> Result<(), SchemaError> {
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(true) => return Ok(()),
            Form::Terminal(false) => {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "terminal true or position node",
                    noeud,
                });
            }
            Form::Node { level, arcs } => {
                if level.0 > longueur_max {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "position level or end marker",
                        noeud,
                    });
                }
                if level.0 == longueur_max {
                    // End-marker level.
                    if arcs.contains_key(&PRESENT) {
                        return Ok(());
                    }
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "end marker present",
                        noeud,
                    });
                }
                let Some((&arc, &suite)) = arcs.iter().next() else {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "position arc",
                        noeud,
                    });
                };
                if arc == cle_fin(domaine) {
                    // Early end marker: the sequence stops here.
                    noeud = suite;
                    continue;
                }
                let atome = Atom(arc);
                if !domaine.contient(atome) {
                    return Err(SchemaError::HorsDomaine(atome));
                }
                elements.push(Value::Atom(atome));
                noeud = suite;
            }
        }
    }
}

fn decompiler_ensemble<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    domaine: &Domaine,
    mut noeud: NodeId,
    elements: &mut BTreeSet<Value>,
) -> Result<(), SchemaError> {
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(true) => return Ok(()),
            Form::Terminal(false) => {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "terminal true (empty set) or presence node",
                    noeud,
                });
            }
            Form::Node { level, arcs } => {
                let index = level.0 as usize;
                if index >= domaine.taille() {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "domain presence level",
                        noeud,
                    });
                }
                if let Some(&suite) = arcs.get(&PRESENT) {
                    elements.insert(Value::Atom(domaine.valeurs[index]));
                    noeud = suite;
                } else if let Some(&suite) = arcs.get(&ABSENT) {
                    noeud = suite;
                } else {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "presence arc",
                        noeud,
                    });
                }
            }
        }
    }
}

fn decompiler_map<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    liste_cles: &[Atom],
    mut noeud: NodeId,
    map: &mut BTreeMap<Value, Value>,
) -> Result<(), SchemaError> {
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(true) => return Ok(()),
            Form::Terminal(false) => {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "terminal true (empty map) or presence node",
                    noeud,
                });
            }
            Form::Node { level, arcs } => {
                if !level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "presence level",
                        noeud,
                    });
                }
                let index = (level.0 / PAS_NIVEAU_MAP) as usize;
                if index >= liste_cles.len() {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "presence level of a key",
                        noeud,
                    });
                }
                if let Some(&noeud_valeur) = arcs.get(&PRESENT) {
                    let Form::Node {
                        level: niveau_valeur,
                        arcs: arcs_valeur,
                    } = pool.resolve(noeud_valeur)?
                    else {
                        return Err(SchemaError::NoeudInattendu {
                            attendu: "value-level node",
                            noeud: noeud_valeur,
                        });
                    };
                    if *niveau_valeur != Schema::niveau_valeur(index) {
                        return Err(SchemaError::NoeudInattendu {
                            attendu: "value level of the key",
                            noeud: noeud_valeur,
                        });
                    }
                    let Some((&valeur, &suite)) = arcs_valeur.iter().next() else {
                        return Err(SchemaError::NoeudInattendu {
                            attendu: "value arc",
                            noeud: noeud_valeur,
                        });
                    };
                    map.insert(Value::Atom(liste_cles[index]), Value::Atom(Atom(valeur)));
                    noeud = suite;
                } else if let Some(&suite) = arcs.get(&ABSENT) {
                    noeud = suite;
                } else {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "presence arc",
                        noeud,
                    });
                }
            }
        }
    }
}
