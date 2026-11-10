//! Schémas de référence (M1-3, spec § 1 et § 2.1) : `Atom(D)` borné,
//! `Set(T)`, `Map(K, V)` bornés, compilation **balayage d'univers** (un
//! niveau de présence par clé, δ = absent, sans marqueur).
//!
//! Niveaux : `Atom(D)` = un niveau `Val = D` ; `Set(D)` = un niveau de
//! présence par élément (`Val = {absent, présent}`, δ = absent) ; `Map(K,V)`
//! = par clé `k_i` un niveau présence `2i` (δ = absent) puis un niveau valeur
//! `2i+1` (`Val = V`, δ = ⊥). Compilation : seul l'arc choisi est interné,
//! l'autre mène au défaut porteur D_δ = terminal(false) sur les présences
//! (R-collapse saute les absents, style ZDD).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_lang::value::{Atom, Value};

/// Clé d'arc de présence : absent (défaut δ) ou présent.
pub const ABSENT: u64 = 0;
/// Clé d'arc de présence : présent.
pub const PRESENT: u64 = 1;
/// Cardinalité d'un niveau de présence : {absent, présent}.
pub const CARDINALITE_PRESENCE: u64 = 2;
/// Pas entre les niveaux d'une map : présence de la clé `i` au niveau `2i`,
/// valeur au niveau `2i+1`.
pub const PAS_NIVEAU_MAP: u32 = 2;

/// Domaine d'atomes borné : énumération finie, avec défaut éventuel.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Domaine {
    /// Valeurs du domaine, dans l'ordre canonique.
    pub valeurs: Vec<Atom>,
    /// Défaut δ du domaine, si déclaré.
    pub defaut: Option<Atom>,
}

impl Domaine {
    /// Domaine sans défaut.
    pub fn nouveau(valeurs: Vec<Atom>) -> Self {
        Domaine {
            valeurs,
            defaut: None,
        }
    }

    /// Le domaine contient-il cet atome ?
    pub fn contient(&self, atome: Atom) -> bool {
        self.valeurs.contains(&atome)
    }

    /// Nombre de valeurs du domaine.
    pub fn taille(&self) -> usize {
        self.valeurs.len()
    }
}

/// Schéma de référence (algèbre de types Σ bornée, spec § 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Schema {
    /// `Atom(D)` : domaine borné d'atomes.
    Atom(Domaine),
    /// `Set(Atom(D))` : ensemble d'atomes d'un domaine borné.
    Set(Domaine),
    /// `Map(K, V)` : clés ordonnées vers un domaine de valeurs borné.
    Map(Vec<Atom>, Domaine),
    /// `Seq(D, N)` : séquence d'au plus `N` éléments d'un domaine borné `D`
    /// (M2-1, constructeur `list`). Le marqueur de fin est une clé requise.
    Seq(Domaine, u32),
    /// `Tag(K, V)` : union typée — niveau discriminant (clés `K`) puis niveau
    /// valeur `V` (M2-2). Un seul arc utile par nœud (C2).
    Tag(Vec<Atom>, Domaine),
}

impl Schema {
    /// Nombre de niveaux du balayage d'univers.
    pub fn nombre_niveaux(&self) -> u32 {
        match self {
            Schema::Atom(_) => 1,
            Schema::Set(domaine) => domaine.taille() as u32,
            Schema::Map(liste_cles, _) => PAS_NIVEAU_MAP * liste_cles.len() as u32,
            Schema::Seq(_, longueur_max) => longueur_max + 1,
            Schema::Tag(liste, _) => PAS_NIVEAU_MAP * liste.len() as u32,
        }
    }

    /// Niveau de présence de la i-ème clé d'une map.
    pub fn niveau_presence(index: usize) -> Level {
        Level(PAS_NIVEAU_MAP * index as u32)
    }

    /// Niveau valeur de la i-ème clé d'une map.
    pub fn niveau_valeur(index: usize) -> Level {
        Level(PAS_NIVEAU_MAP * index as u32 + 1)
    }
}

/// Erreur typée de compilation/décompilation d'un schéma.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SchemaError {
    /// La valeur ne correspond pas au schéma (forme de Σ différente).
    TypeMismatch {
        /// Forme attendue par le schéma.
        attendu: &'static str,
        /// Forme reçue.
        recu: &'static str,
    },
    /// Atome hors du domaine déclaré par le schéma.
    HorsDomaine(Atom),
    /// Nœud du diagramme incompatible avec le schéma (décompilation).
    NoeudInattendu {
        /// Forme attendue à cette position.
        attendu: &'static str,
        /// Nœud rencontré.
        noeud: NodeId,
    },
    /// Erreur remontée du port pool.
    Pool(PoolError),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::TypeMismatch { attendu, recu } => {
                write!(
                    formatter,
                    "type incompatible: attendu {attendu}, reçu {recu}"
                )
            }
            SchemaError::HorsDomaine(atome) => write!(formatter, "atome hors domaine: {atome:?}"),
            SchemaError::NoeudInattendu { attendu, noeud } => {
                write!(
                    formatter,
                    "nœud inattendu: attendu {attendu}, nœud {noeud:?}"
                )
            }
            SchemaError::Pool(error) => write!(formatter, "erreur du pool: {error}"),
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

/// Algèbre de clés d'un schéma : `Val(ℓ)` et δ(ℓ) par niveau du balayage.
#[derive(Clone)]
pub struct SchemaKeys {
    schema: Schema,
}

impl SchemaKeys {
    /// Algèbre de clés pour le schéma donné.
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
                    domaine.taille() as u64 + 1 // +1 pour le marqueur de fin
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
            Schema::Seq(_, _) => None, // marqueur de fin = clé requise, pas de défaut
            Schema::Tag(_, _) => None, // discriminant = clé requise
        }
    }
}

/// Contrôleur de l'instance de référence : porte le schéma, désigne D_δ =
/// terminal(false) (l'ensemble vide) pour les niveaux de présence et les
/// domaines d'atomes avec défaut ; δ = ⊥ (pas de défaut) pour les niveaux
/// valeur (spec § 2.1).
pub struct ReferenceController {
    schema: Schema,
    keys: SchemaKeys,
}

impl ReferenceController {
    /// Contrôleur pour le schéma donné.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        ReferenceController { schema, keys }
    }

    /// Schéma de cette instance.
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

/// Compile une valeur en diagramme (balayage d'univers) ; le terminal
/// d'acceptation a pour payload `true` (lecture ensembliste).
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
                            attendu: "ensemble d'atomes",
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

/// Valide les clés et valeurs d'une map contre le schéma (erreurs typées,
/// l'atome fautif est celui rapporté).
fn valider_map(
    liste_cles: &[Atom],
    domaine_valeurs: &Domaine,
    map: &BTreeMap<Value, Value>,
) -> Result<(), SchemaError> {
    for (cle, valeur) in map {
        let Value::Atom(atome_cle) = cle else {
            return Err(SchemaError::TypeMismatch {
                attendu: "clé atomique",
                recu: forme_valeur(cle),
            });
        };
        if !liste_cles.contains(atome_cle) {
            return Err(SchemaError::HorsDomaine(*atome_cle));
        }
        let Value::Atom(atome_valeur) = valeur else {
            return Err(SchemaError::TypeMismatch {
                attendu: "valeur atomique",
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
        Schema::Atom(_) => "atome",
        Schema::Set(_) => "ensemble",
        Schema::Map(_, _) => "map",
        Schema::Seq(_, _) => "séquence",
        Schema::Tag(_, _) => "union typée",
    }
}

fn forme_valeur(valeur: &Value) -> &'static str {
    match valeur {
        Value::Atom(_) => "atome",
        Value::Set(_) => "ensemble",
        Value::Map(_) => "map",
        Value::Seq(_) => "séquence",
        Value::Tag(_, _) => "union typée",
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
                attendu: "valeur atomique",
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

/// Valide une séquence contre le schéma Seq (éléments dans le domaine,
/// longueur bornée).
fn valider_seq(
    domaine: &Domaine,
    longueur_max: u32,
    elements: &[Value],
) -> Result<(), SchemaError> {
    if elements.len() > longueur_max as usize {
        return Err(SchemaError::TypeMismatch {
            attendu: "séquence de longueur bornée",
            recu: "séquence trop longue",
        });
    }
    for element in elements {
        match element {
            Value::Atom(atome) if domaine.contient(*atome) => {}
            Value::Atom(atome) => return Err(SchemaError::HorsDomaine(*atome)),
            autre => {
                return Err(SchemaError::TypeMismatch {
                    attendu: "élément atomique",
                    recu: forme_valeur(autre),
                });
            }
        }
    }
    Ok(())
}

/// Clé d'arc du marqueur de fin dans une séquence (valeur = taille du domaine).
fn cle_fin(domaine: &Domaine) -> u64 {
    domaine.taille() as u64
}

/// Compile une séquence par **positions** (balayage d'univers, M2-1) :
/// chaque position `i` a un niveau de cardinalité `|D|+1` (valeurs du
/// domaine + marqueur de fin). Le marqueur de fin mène au terminal true.
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
        // Niveau marqueur de fin : clé requise, présent = fin de séquence.
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
            _ => unreachable!("validé par valider_seq"),
        }
    } else {
        cle_fin(domaine) // marqueur de fin
    };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(position),
            arcs: BTreeMap::from([(arc, suite)]),
        },
    ))
}

/// Valide un tag contre le schéma (discriminant dans la liste, payload dans le domaine).
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
            attendu: "payload atomique",
            recu: forme_valeur(autre),
        }),
    }
}

/// Compile un tag : niveau discriminant (un arc = un variant) puis niveau
/// valeur. Un seul arc utile par nœud (C2).
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
            attendu: "payload atomique",
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

/// Décompile un diagramme en valeur (inverse de [`compiler`]) : utile pour
/// les tests de round-trip et les diagnostics.
pub fn decompiler<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    noeud: NodeId,
) -> Result<Value, SchemaError> {
    match schema {
        Schema::Atom(domaine) => {
            // Domaine à un seul élément : R-collapse réduit le nœud atomique
            // au terminal d'acceptation.
            if matches!(pool.resolve(noeud), Ok(Form::Terminal(true))) {
                return match domaine.valeurs.first() {
                    Some(atome) if domaine.taille() == 1 => Ok(Value::Atom(*atome)),
                    _ => Err(SchemaError::NoeudInattendu {
                        attendu: "nœud de niveau atomique",
                        noeud,
                    }),
                };
            }
            let Form::Node { level, arcs } = pool.resolve(noeud)? else {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "nœud de niveau atomique",
                    noeud,
                });
            };
            if *level != Level(0) {
                return Err(SchemaError::NoeudInattendu {
                    attendu: "niveau 0",
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
                attendu: "arc vers terminal true",
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
            attendu: "nœud discriminant",
            noeud,
        });
    };
    if *level != Level(0) {
        return Err(SchemaError::NoeudInattendu {
            attendu: "niveau discriminant (0)",
            noeud,
        });
    }
    let Some((&index, &noeud_valeur)) = arcs.iter().next() else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "arc discriminant",
            noeud,
        });
    };
    let discriminant = *liste
        .get(index as usize)
        .ok_or(SchemaError::NoeudInattendu {
            attendu: "index de discriminant valide",
            noeud,
        })?;
    let Form::Node {
        level: niveau_valeur,
        arcs: arcs_valeur,
    } = pool.resolve(noeud_valeur)?
    else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "nœud valeur",
            noeud: noeud_valeur,
        });
    };
    if *niveau_valeur != Level(1) {
        return Err(SchemaError::NoeudInattendu {
            attendu: "niveau valeur (1)",
            noeud: noeud_valeur,
        });
    }
    let Some((&valeur, _)) = arcs_valeur.iter().next() else {
        return Err(SchemaError::NoeudInattendu {
            attendu: "arc valeur",
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
                    attendu: "terminal true ou nœud de position",
                    noeud,
                });
            }
            Form::Node { level, arcs } => {
                if level.0 > longueur_max {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "niveau de position ou marqueur de fin",
                        noeud,
                    });
                }
                if level.0 == longueur_max {
                    // Niveau marqueur de fin.
                    if arcs.contains_key(&PRESENT) {
                        return Ok(());
                    }
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "marqueur de fin présent",
                        noeud,
                    });
                }
                let Some((&arc, &suite)) = arcs.iter().next() else {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "arc de position",
                        noeud,
                    });
                };
                if arc == cle_fin(domaine) {
                    // Marqueur de fin anticipé : la séquence s'arrête ici.
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
                    attendu: "terminal true (ensemble vide) ou nœud de présence",
                    noeud,
                });
            }
            Form::Node { level, arcs } => {
                let index = level.0 as usize;
                if index >= domaine.taille() {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "niveau de présence du domaine",
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
                        attendu: "arc de présence",
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
                    attendu: "terminal true (map vide) ou nœud de présence",
                    noeud,
                });
            }
            Form::Node { level, arcs } => {
                if !level.0.is_multiple_of(PAS_NIVEAU_MAP) {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "niveau de présence",
                        noeud,
                    });
                }
                let index = (level.0 / PAS_NIVEAU_MAP) as usize;
                if index >= liste_cles.len() {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "niveau de présence d'une clé",
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
                            attendu: "nœud de niveau valeur",
                            noeud: noeud_valeur,
                        });
                    };
                    if *niveau_valeur != Schema::niveau_valeur(index) {
                        return Err(SchemaError::NoeudInattendu {
                            attendu: "niveau valeur de la clé",
                            noeud: noeud_valeur,
                        });
                    }
                    let Some((&valeur, &suite)) = arcs_valeur.iter().next() else {
                        return Err(SchemaError::NoeudInattendu {
                            attendu: "arc de valeur",
                            noeud: noeud_valeur,
                        });
                    };
                    map.insert(Value::Atom(liste_cles[index]), Value::Atom(Atom(valeur)));
                    noeud = suite;
                } else if let Some(&suite) = arcs.get(&ABSENT) {
                    noeud = suite;
                } else {
                    return Err(SchemaError::NoeudInattendu {
                        attendu: "arc de présence",
                        noeud,
                    });
                }
            }
        }
    }
}
