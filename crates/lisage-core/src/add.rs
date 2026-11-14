//! ADD/MTBDD (M5-4, statement C7, open question n° 8 of spec § 10):
//! **lifted construction** — non-idempotent weighted merge on terminals,
//! unique table indexed by weights (the handle includes the weight, so two
//! terminals with different weights are distinct handles), ⊕ pointwise as in
//! ADD. Compatibility guard (A5): a weighted fold is exact only on this
//! lifted construction; on a Bool construction (`union` idempotent), only the
//! cardinal is exact — multiplicity is destroyed at write time.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};

use crate::error::CoreError;
use crate::schema::{ABSENT, PAS_NIVEAU_MAP, PRESENT, Schema, SchemaError, SchemaKeys};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the ADD controller (distinct from `ReferenceController`).
const FINGERPRINT_ADD: ControllerId = ControllerId(2);

/// Controller of the lifted ADD construction: payload = weight `u64` on
/// terminals, `D_δ = terminal(0)` (zero weight) at presence levels.
pub struct AddController {
    schema: Schema,
    keys: SchemaKeys,
}

impl AddController {
    /// ADD controller for the given schema.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        AddController { schema, keys }
    }

    /// Schema of this instance.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl Controller for AddController {
    type Key = u64;
    type Payload = u64;
    type Keys = SchemaKeys;

    const FINGERPRINT: ControllerId = FINGERPRINT_ADD;

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        match &self.schema {
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Compiles a `Value::Map` into a weighted diagram of the given weight
/// (universe sweep, mirror of `schema::compiler` for payload `u64`).
pub fn compiler_pondere<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    anchor: Anchor,
    schema: &Schema,
    valeur: &Value,
    poids: u64,
) -> Result<NodeId, SchemaError> {
    let Schema::Map(liste_cles, domaine) = schema else {
        return Err(SchemaError::TypeMismatch {
            attendu: "map",
            recu: "autre schéma",
        });
    };
    let Value::Map(map) = valeur else {
        return Err(SchemaError::TypeMismatch {
            attendu: "map",
            recu: "autre valeur",
        });
    };
    for (cle, valeur) in map {
        let (Value::Atom(atome_cle), Value::Atom(atome_valeur)) = (cle, valeur) else {
            return Err(SchemaError::TypeMismatch {
                attendu: "clé et valeur atomiques",
                recu: "non atomique",
            });
        };
        if !liste_cles.contains(atome_cle) {
            return Err(SchemaError::HorsDomaine(*atome_cle));
        }
        if !domaine.contient(*atome_valeur) {
            return Err(SchemaError::HorsDomaine(*atome_valeur));
        }
    }
    Ok(compiler_map_pondere(
        pool, anchor, liste_cles, map, 0, poids,
    ))
}

/// Recursive compilation of a map into a weighted presence/value chain.
fn compiler_map_pondere<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    anchor: Anchor,
    liste_cles: &[Atom],
    map: &BTreeMap<Value, Value>,
    index: usize,
    poids: u64,
) -> NodeId {
    if index == liste_cles.len() {
        return pool.intern(anchor, Form::Terminal(poids));
    }
    let suite = compiler_map_pondere(pool, anchor, liste_cles, map, index + 1, poids);
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
        _ => BTreeMap::from([(ABSENT, suite)]),
    };
    pool.intern(
        anchor,
        Form::Node {
            level: Schema::niveau_presence(index),
            arcs,
        },
    )
}

/// ADD construction merge: ⊕ pointwise on terminal weights, **non-idempotent**
/// (`merge_add(a, a)` doubles the weights — the unique table is indexed by
/// weights, the collapse of duplicates is lost).
pub fn merge_add<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    a: NodeId,
    b: NodeId,
) -> Result<NodeId, CoreError> {
    let (forme_a, forme_b) = (pool.resolve(a)?.clone(), pool.resolve(b)?.clone());
    match (forme_a, forme_b) {
        (Form::Terminal(poids_a), Form::Terminal(poids_b)) => {
            Ok(pool.intern(anchor, Form::Terminal(poids_a + poids_b)))
        }
        (Form::Node { level, arcs }, Form::Terminal(poids_b)) => {
            if poids_b == 0 {
                Ok(a)
            } else {
                merge_avec_autre(pool, keys, anchor, level, arcs, b)
            }
        }
        (Form::Terminal(poids_a), Form::Node { level, arcs }) => {
            if poids_a == 0 {
                Ok(b)
            } else {
                merge_avec_autre(pool, keys, anchor, level, arcs, a)
            }
        }
        (
            Form::Node {
                level: level_a,
                arcs: arcs_a,
            },
            Form::Node {
                level: level_b,
                arcs: arcs_b,
            },
        ) => {
            if level_a == level_b {
                let mut arcs = arcs_a;
                for (arc, cible_b) in arcs_b {
                    match arcs.remove(&arc) {
                        Some(cible_a) => {
                            let fusion = merge_add(pool, keys, anchor, cible_a, cible_b)?;
                            arcs.insert(arc, fusion);
                        }
                        None => {
                            arcs.insert(arc, cible_b);
                        }
                    }
                }
                Ok(pool.intern(
                    anchor,
                    Form::Node {
                        level: level_a,
                        arcs,
                    },
                ))
            } else if level_a < level_b {
                merge_avec_autre(pool, keys, anchor, level_a, arcs_a, b)
            } else {
                merge_avec_autre(pool, keys, anchor, level_b, arcs_b, a)
            }
        }
    }
}

/// Merge of a node of level `level` with another diagram `autre` (terminal of
/// weight > 0, or deeper node): the path of `autre` goes through the default
/// arc `δ(level)` if the level has one, otherwise through all values of the
/// level.
fn merge_avec_autre<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    level: Level,
    mut arcs: BTreeMap<u64, NodeId>,
    autre: NodeId,
) -> Result<NodeId, CoreError> {
    match keys.default(level) {
        Some(defaut) => {
            let cible = match arcs.remove(&defaut) {
                Some(cible) => merge_add(pool, keys, anchor, cible, autre)?,
                None => autre,
            };
            arcs.insert(defaut, cible);
        }
        None => {
            let cardinalite = keys.cardinality(level);
            for v in 0..cardinalite {
                let cible = match arcs.remove(&v) {
                    Some(cible) => merge_add(pool, keys, anchor, cible, autre)?,
                    None => autre,
                };
                arcs.insert(v, cible);
            }
        }
    }
    Ok(pool.intern(anchor, Form::Node { level, arcs }))
}

/// Weighted fold: sum of terminal weights over all accepting paths
/// (expansion-counting A4 as in `Pli`). Exact **only** on a lifted ADD
/// construction — this is the compatibility guard (Q8).
pub fn somme_poids<P: Pool<Key = u64, Payload = u64>>(
    pool: &P,
    schema: &Schema,
    noeud: NodeId,
) -> Result<u64, CoreError> {
    let keys = SchemaKeys::new(schema.clone());
    let nombre_niveaux = schema.nombre_niveaux();
    let mut memo = HashMap::new();
    somme(pool, &keys, nombre_niveaux, noeud, &mut memo)
}

/// Recursive weighted sum with memo on nodes.
fn somme<P: Pool<Key = u64, Payload = u64>>(
    pool: &P,
    keys: &SchemaKeys,
    nombre_niveaux: u32,
    noeud: NodeId,
    memo: &mut HashMap<NodeId, u64>,
) -> Result<u64, CoreError> {
    if let Some(total) = memo.get(&noeud) {
        return Ok(*total);
    }
    let forme = pool.resolve(noeud)?.clone();
    let total = match forme {
        Form::Terminal(poids) => poids,
        Form::Node { level, arcs } => {
            let mut total = 0u64;
            for (arc, cible) in &arcs {
                let niveau_cible = match pool.resolve(*cible)?.clone() {
                    Form::Node { level: niveau, .. } => Some(niveau),
                    Form::Terminal(_) => None,
                };
                let multiplicateur =
                    multiplicateur(keys, nombre_niveaux, level, *arc, niveau_cible);
                total += multiplicateur * somme(pool, keys, nombre_niveaux, *cible, memo)?;
            }
            // Implicit default arc: D_δ = terminal(0), of zero weight.
            total
        }
    };
    memo.insert(noeud, total);
    Ok(total)
}

/// (A4) multiplier of arc `arc` of level `level` towards a child of level
/// `niveau_cible` (`None` = terminal): product of the cardinalities of the
/// skipped value levels on the path (mirror of `Pli::multiplicateur`).
fn multiplicateur(
    keys: &SchemaKeys,
    nombre_niveaux: u32,
    level: Level,
    arc: u64,
    niveau_cible: Option<Level>,
) -> u64 {
    let borne = match niveau_cible {
        Some(niveau) => niveau.0,
        None => nombre_niveaux,
    };
    let mut n = match keys.default(level) {
        Some(defaut) if arc == defaut => level.0 + PAS_NIVEAU_MAP,
        _ => level.0 + 1,
    };
    let mut produit = 1u64;
    while n < borne {
        if keys.default(Level(n)).is_some() {
            n += PAS_NIVEAU_MAP;
        } else {
            produit *= keys.cardinality(Level(n));
            n += 1;
        }
    }
    produit
}
