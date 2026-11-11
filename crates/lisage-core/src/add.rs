//! ADD/MTBDD (M5-4, énoncé C7, question ouverte n° 8 de la spec § 10) :
//! **construction levée** — merge non idempotent à poids sur les terminaux,
//! table unique indexée par les poids (le handle inclut le poids, donc deux
//! terminaux de poids différents sont des handles distincts), ⊕ pointwise
//! à la ADD. Garde-fou de compatibilité (A5) : un pli pondéré n'est exact
//! que sur cette construction levée ; sur une construction Bool
//! (`union` idempotent), seul le cardinal est exact — la multiplicité est
//! détruite à l'écriture.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};

use crate::error::CoreError;
use crate::schema::{ABSENT, Domaine, PRESENT, Schema, SchemaError, SchemaKeys};
use lisage_lang::value::{Atom, Value};

/// Empreinte du contrôleur ADD (distincte de `ReferenceController`).
const FINGERPRINT_ADD: ControllerId = ControllerId(2);

/// Contrôleur de la construction levée ADD : payload = poids `u64` sur les
/// terminaux, `D_δ = terminal(0)` (poids nul) aux niveaux de présence.
pub struct AddController {
    schema: Schema,
    keys: SchemaKeys,
}

impl AddController {
    /// Contrôleur ADD pour le schéma donné.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        AddController { schema, keys }
    }

    /// Schéma de cette instance.
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
                if level.0.is_multiple_of(crate::schema::PAS_NIVEAU_MAP) {
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Compile une `Value::Map` en diagramme pondéré de poids donné (balayage
/// d'univers, miroir de `schema::compiler` pour payload `u64`).
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
        if !liste_cles.contains(atome_cle) || !domaine.contient(*atome_valeur) {
            return Err(SchemaError::HorsDomaine(*atome_cle));
        }
    }
    Ok(compiler_map_pondere(
        pool, anchor, liste_cles, map, 0, poids,
    ))
}

/// Compilation récursive d'une map en chaîne présence/valeur pondérée.
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

/// Merge de construction ADD : ⊕ pointwise sur les poids des terminaux,
/// **non idempotent** (`merge_add(a, a)` double les poids — la table unique
/// est indexée par les poids, le collapse des doublons est perdu).
pub fn merge_add<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    a: NodeId,
    b: NodeId,
) -> Result<NodeId, CoreError> {
    let (forme_a, forme_b) = (pool.resolve(a)?.clone(), pool.resolve(b)?.clone());
    match (forme_a, forme_b) {
        (Form::Terminal(pa), Form::Terminal(pb)) => {
            Ok(pool.intern(anchor, Form::Terminal(pa + pb)))
        }
        (Form::Node { level, arcs }, Form::Terminal(pb)) => {
            if pb == 0 {
                Ok(a)
            } else {
                merge_avec_autre(pool, keys, anchor, level, arcs, b)
            }
        }
        (Form::Terminal(pa), Form::Node { level, arcs }) => {
            if pa == 0 {
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

/// Merge d'un nœud de niveau `level` avec un autre diagramme `autre`
/// (terminal de poids > 0, ou nœud plus profond) : le chemin de `autre`
/// passe par l'arc par défaut `δ(level)` si le niveau en a un, sinon par
/// toutes les valeurs du niveau.
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

/// Pli pondéré : somme des poids des terminaux sur tous les chemins
/// acceptants (expansion-comptage A4 comme `Pli`). Exact **uniquement** sur
/// une construction levée ADD — c'est le garde-fou de compatibilité (Q8).
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

/// Somme pondérée récursive avec mémo sur les nœuds.
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
            // Arc par défaut implicite : D_δ = terminal(0), de poids nul.
            total
        }
    };
    memo.insert(noeud, total);
    Ok(total)
}

/// (A4) multiplicateur de l'arc `arc` du niveau `level` vers un fils de
/// niveau `niveau_cible` (`None` = terminal) : produit des cardinalités des
/// niveaux valeur sautés sur le chemin (miroir de `Pli::multiplicateur`).
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
        Some(defaut) if arc == defaut => level.0 + 2,
        _ => level.0 + 1,
    };
    let mut produit = 1u64;
    while n < borne {
        if keys.default(Level(n)).is_some() {
            n += 2;
        } else {
            produit *= keys.cardinality(Level(n));
            n += 1;
        }
    }
    produit
}

/// Domaine `0..=max` pour un schéma de test.
pub fn domaine_jetons(max: u64) -> Domaine {
    Domaine::nouveau((0..=max).map(Atom).collect())
}
