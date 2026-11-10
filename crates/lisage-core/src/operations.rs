//! Opérations à canal hérité seul (M1-6, spec § 4) : `exists`, `filter`
//! (garde = image réciproque d'un prédicat) et `map` (image par fonction,
//! réétiquetage). Le canal hérité porte le **chemin** de la map en
//! construction ; le verdict est résolu au terminal (spec § 4 : « le verdict
//! est résolu au niveau de la valeur, mais appliqué au niveau de la
//! présence » — ici le canal hérité le transporte jusqu'au terminal).
//!
//! - [`Existe`] : pli sur `(Bool, ∨, ∧)` (spec § 5), `S = BoolOu`.
//! - [`Filtre`] : restriction, `S` trivial, le diagramme de sortie est
//!   l'ensemble filtré (les membres rejetés mènent à `ε(false)`).
//! - [`Image`] : réétiquetage des atomes d'un ensemble (schéma `Atom`),
//!   `S` trivial, les arcs du niveau 0 sont remplacés par leur image.
//!
//! Le canal hérité porte l'**ensemble des clés présentes** sur le chemin.
//! Les prédicats sont donc des prédicats de présence, bien définis sur le
//! diagramme réduit ; un verdict dépendant des *valeurs* sur un niveau
//! `val(k)` collapsé exige le T-EXPAND de § 6.2 (livré pour `lookup`),
//! hors périmètre ici.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::pli::{BoolOu, SemiAnneau};
use crate::schema::{PAS_NIVEAU_MAP, PRESENT, Schema};
use crate::transduction::{Transducteur, Transduction};

/// Base du namespace de fingerprints de la famille garde/image.
const FINGERPRINT_BASE_GARDE: u64 = 5000;

/// Ensemble des clés présentes sur le chemin d'une map en construction.
pub type ClePresentes = BTreeSet<u64>;

/// Prédicat de présence sur une map (exemple : `p(m) = présent(a)`).
pub type PredicatMap = Box<dyn Fn(&ClePresentes) -> bool + Send + Sync>;

/// Transduction `exists(p)` sur un ensemble de maps (schéma `Map`) :
/// pli sur `(Bool, ∨, ∧)`, le verdict est résolu au terminal sur le chemin
/// complet (spec § 5).
pub struct Existe {
    liste_cles: Vec<u64>,
    predicat: PredicatMap,
}

impl Existe {
    /// `exists(p)` pour le schéma `Map(liste_cles, _)` donné ; erreur typée
    /// si le schéma n'est pas une map.
    pub fn new(schema: &Schema, predicat: PredicatMap) -> Result<Self, CoreError> {
        let Schema::Map(liste_cles, _) = schema else {
            return Err(CoreError::SchemaInattendu {
                attendu: "Map",
                recu: forme_schema(schema),
            });
        };
        Ok(Existe {
            liste_cles: liste_cles.iter().map(|atome| atome.0).collect(),
            predicat,
        })
    }
}

/// Transduction `filter(p)` (garde) sur un ensemble de maps : restriction,
/// les membres rejetés mènent à `ε(false)` (spec § 4, `f_filter` = image
/// réciproque de `p`).
pub struct Filtre {
    liste_cles: Vec<u64>,
    predicat: PredicatMap,
}

impl Filtre {
    /// `filter(p)` pour le schéma `Map(liste_cles, _)` donné ; erreur typée
    /// si le schéma n'est pas une map.
    pub fn new(schema: &Schema, predicat: PredicatMap) -> Result<Self, CoreError> {
        let Schema::Map(liste_cles, _) = schema else {
            return Err(CoreError::SchemaInattendu {
                attendu: "Map",
                recu: forme_schema(schema),
            });
        };
        Ok(Filtre {
            liste_cles: liste_cles.iter().map(|atome| atome.0).collect(),
            predicat,
        })
    }
}

/// Nom de la forme d'un schéma (pour les erreurs typées).
fn forme_schema(schema: &Schema) -> &'static str {
    match schema {
        Schema::Atom(_) => "Atom",
        Schema::Set(_) => "Set",
        Schema::Map(_, _) => "Map",
    }
}

/// Valide que tous les niveaux du diagramme sont dans le schéma (sinon
/// `disperser_chemin` indicerait hors limites) : erreur typée `CoreError`.
fn valider_niveaux<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    noeud: NodeId,
    visite: &mut BTreeSet<NodeId>,
) -> Result<(), CoreError> {
    if !visite.insert(noeud) {
        return Ok(());
    }
    match pool.resolve(noeud)? {
        Form::Terminal(_) => Ok(()),
        Form::Node { level, arcs } => {
            if level.0 >= schema.nombre_niveaux() {
                return Err(CoreError::NiveauHorsSchema(*level));
            }
            for cible in arcs.values() {
                valider_niveaux(pool, schema, *cible, visite)?;
            }
            Ok(())
        }
    }
}

/// Transduction `map(f)` sur un ensemble d'atomes (schéma `Atom`) :
/// réétiquetage, chaque arc du niveau 0 est remplacé par son image par `f`
/// (spec § 4 : « map, filter, réétiquetages »).
pub struct Image {
    fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>,
}

impl Image {
    /// Image par `f` des atomes d'un ensemble (schéma `Atom`).
    pub fn new(fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>) -> Self {
        Image { fonction }
    }
}

/// Descendant commun : met à jour l'ensemble des clés présentes le long de
/// l'arc `v` du niveau `ℓ` (un niveau de présence avec l'arc `présent`
/// ajoute la clé ; un niveau valeur ne change rien).
fn disperser_chemin(liste_cles: &[u64], level: Level, h: &ClePresentes, arc: u64) -> ClePresentes {
    let mut h = h.clone();
    if level.0.is_multiple_of(PAS_NIVEAU_MAP) && arc == PRESENT {
        let index = (level.0 / PAS_NIVEAU_MAP) as usize;
        // Garanti par `valider_niveaux` avant la transduction.
        if let Some(&cle) = liste_cles.get(index) {
            h.insert(cle);
        }
    }
    h
}

impl Transduction for Existe {
    type H = ClePresentes;
    type S = BoolOu;
    type Out = bool;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 1
    }

    fn h0(&self) -> ClePresentes {
        ClePresentes::new()
    }

    fn phi_disp(&self, level: Level, h: &ClePresentes, arc: u64) -> ClePresentes {
        disperser_chemin(&self.liste_cles, level, h, arc)
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &ClePresentes,
        arcs: &BTreeMap<u64, (Option<Level>, BoolOu)>,
    ) -> BoolOu {
        // (Bool, ∨, ∧) : le multiplicateur A4 vaut ⊤ (|Val| ≥ 1), le pli
        // est le ∨ des sous-totaux ; l'arc par défaut implicite mène à
        // D_δ = ε(false), de poids faux.
        arcs.values().fold(BoolOu(false), |acc, (_, s)| acc.plus(s))
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        _h: &ClePresentes,
        arcs: BTreeMap<u64, (NodeId, BoolOu)>,
    ) -> NodeId {
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        pool.intern(anchor, Form::Node { level, arcs })
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &ClePresentes,
    ) -> (NodeId, BoolOu) {
        let verdict = payload && (self.predicat)(h);
        (
            pool.intern(anchor, Form::Terminal(payload)),
            BoolOu(verdict),
        )
    }
}

impl Transduction for Filtre {
    type H = ClePresentes;
    type S = ();
    type Out = bool;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 2
    }

    fn h0(&self) -> ClePresentes {
        ClePresentes::new()
    }

    fn phi_disp(&self, level: Level, h: &ClePresentes, arc: u64) -> ClePresentes {
        disperser_chemin(&self.liste_cles, level, h, arc)
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &ClePresentes,
        _arcs: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        _h: &ClePresentes,
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> NodeId {
        // (A2) : si tous les fils mènent à ε(false), le nœud est ε(false)
        // (les nœuds produits par le runner sont toujours résolubles).
        let tout_rejete = !arcs.is_empty()
            && arcs
                .values()
                .all(|(n, _)| matches!(pool.resolve(*n), Ok(Form::Terminal(false))));
        if tout_rejete {
            return pool.intern(anchor, Form::Terminal(false));
        }
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        pool.intern(anchor, Form::Node { level, arcs })
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &ClePresentes,
    ) -> (NodeId, ()) {
        // Garde : le membre est rejeté (ε(false)) si le prédicat échoue.
        let garde = payload && (self.predicat)(h);
        (pool.intern(anchor, Form::Terminal(garde)), ())
    }
}

impl Transduction for Image {
    type H = ();
    type S = ();
    type Out = bool;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_GARDE + 3
    }

    fn h0(&self) {}

    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }

    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        _h: &(),
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> NodeId {
        // Réétiquetage : l'arc v devient f(v).
        let arcs = arcs
            .into_iter()
            .map(|(v, (n, _))| ((self.fonction)(v), n))
            .collect();
        pool.intern(anchor, Form::Node { level, arcs })
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

/// `exists(p)` sur l'ensemble de maps dénoté par `noeud` (schéma `Map`).
pub fn existe<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    noeud: NodeId,
    predicat: PredicatMap,
) -> Result<bool, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Existe::new(schema, predicat)?;
    let mut visite = BTreeSet::new();
    valider_niveaux(pool_entree, schema, noeud, &mut visite)?;
    let mut runner = Transducteur::new(transduction);
    let (_, verdict) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(verdict.0)
}

/// `filter(p)` sur l'ensemble de maps dénoté par `noeud` : renvoie le
/// diagramme de l'ensemble filtré (canonique, A3).
pub fn filtrer<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    noeud: NodeId,
    predicat: PredicatMap,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let transduction = Filtre::new(schema, predicat)?;
    let mut visite = BTreeSet::new();
    valider_niveaux(pool_entree, schema, noeud, &mut visite)?;
    let mut runner = Transducteur::new(transduction);
    let (resultat, _) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(resultat)
}

/// `map(f)` sur l'ensemble d'atomes dénoté par `noeud` (schéma `Atom`).
pub fn image<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    noeud: NodeId,
    fonction: Box<dyn Fn(u64) -> u64 + Send + Sync>,
) -> Result<NodeId, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducteur::new(Image::new(fonction));
    let (resultat, _) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(resultat)
}
