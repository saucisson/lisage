//! Instance-test **CDD** (*Clock Decision Diagrams*, école UPPAAL, Larsen et
//! al. ; backlog § 9 de la feuille de route, question ouverte 9 de la spec,
//! veine #83 après les fragments intervalles, DBM et l'instance-test
//! DDD-différence) : les **unions non convexes de zones DBM** comme
//! clés-régions d'horloges — le cas « δ = différences d'horloges, ensembles
//! non convexes » de l'état de l'art.
//!
//! Spec Q9 (i) : « unions non convexes sans forme normale connue » — la
//! non-canonicité est **assumée** (style DDD) : la forme normale de l'union
//! est le tri + dédup des zones clôturées (égalité syntaxique des écritures
//! normalisées), pas l'égalité ensembliste. Deux écritures d'un même
//! ensemble non convexe (par exemple une zone incluse dans une autre) peu-
//! vent rester des clés distinctes : C1 tient relativement à cette forme
//! normale, pas à l'ensemble dénoté. L'appartenance d'un point, elle, est
//! décidable sémantiquement (appartenance à l'une des zones).
//!
//! Le contrat de clés (forme normale, couverture, intersection) reste déclaré
//! côté contrôleur (`Cdd`, `CddKeys`) ; le pool ne change pas. Le balayage
//! d'univers ne compile que si l'univers est borné à régions énumérables
//! (spec Q9) : la construction du diagramme passe par les régions explicites,
//! jamais par l'énumération des points — royaume libre DDD assumé sur le
//! continu.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

use crate::dbm::Dbm;

/// Union vide canonique : zéro zone — l'« absent » continu δ(ℓ).
pub const VIDE: Cdd = Cdd { zones: Vec::new() };

/// Clé-région non convexe : union finie de zones DBM clôturées sur
/// `nombre_variables` horloges. Toute construction passe par [`Cdd::nouveau`],
/// qui normalise : zones vides retirées, tri, dédup — la forme normale
/// *syntaxique* de l'union (non-canonicité assumée, spec Q9 (i)).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Cdd {
    zones: Vec<Dbm>,
}

impl Cdd {
    /// Union des `zones` données, normalisée (vides retirées, tri, dédup).
    pub fn nouveau(zones: &[Dbm]) -> Self {
        let mut normalisees: Vec<Dbm> = zones
            .iter()
            .filter(|zone| !zone.est_vide())
            .cloned()
            .collect();
        normalisees.sort();
        normalisees.dedup();
        Cdd { zones: normalisees }
    }

    /// Les zones de l'union, triées et dédupliquées.
    pub fn zones(&self) -> &[Dbm] {
        &self.zones
    }

    /// L'union est-elle vide (zéro zone) ?
    pub fn est_vide(&self) -> bool {
        self.zones.is_empty()
    }

    /// Le point `point[i] = x_i` appartient-il à l'union (sémantique,
    /// décidable : appartenance à l'une des zones) ?
    pub fn contient(&self, point: &[u64]) -> bool {
        self.zones.iter().any(|zone| zone.contient(point))
    }

    /// Intersection (meet) de deux unions : distributivité — l'union des
    /// meets point à point des zones, puis normalisation (les meets vides
    /// sont retirés). Le meet de deux unions de zones est une union de
    /// zones : le treillis est clos.
    pub fn intersection(&self, autre: &Cdd) -> Cdd {
        let mut zones: Vec<Dbm> = Vec::new();
        for a in &self.zones {
            for b in &autre.zones {
                let meet = a.intersection(b);
                if !meet.est_vide() {
                    zones.push(meet);
                }
            }
        }
        Cdd::nouveau(&zones)
    }
}

/// Algèbre de clés-régions CDD sur domaine borné par variable :
/// `Val(ℓ) = [0, taille_domaine)`, une clé = une union finie de zones DBM de
/// la boîte, `δ(ℓ)` = l'union vide [`VIDE`].
#[derive(Clone)]
pub struct CddKeys {
    taille_domaine: u64,
}

impl CddKeys {
    /// Algèbre pour des variables à domaine borné `[0, taille_domaine)`.
    pub fn nouvelle(taille_domaine: u64) -> Self {
        CddKeys { taille_domaine }
    }

    /// Taille du domaine d'une variable, `Val(ℓ) = [0, taille)`.
    pub fn taille_domaine(&self) -> u64 {
        self.taille_domaine
    }

    /// Couverture : la famille d'unions couvre-t-elle toute la boîte bornée
    /// `[0, taille_domaine)^n` (n = dimension des zones) ?
    ///
    /// Contrat de clés déclaré côté contrôleur (feuille de route § 9 :
    /// « forme normale, couverture, intersection »). Décidable par
    /// balayage d'univers **seulement si l'univers est borné à régions
    /// énumérables** (spec Q9) : sur le domaine continu, le balayage ne
    /// compile pas et le royaume libre DDD est assumé — la couverture se
    /// déclare alors par les arcs explicites du diagramme.
    pub fn couverture(&self, regions: &[Cdd]) -> bool {
        // L'union vide ne couvre rien ; la dimension est celle des zones
        // d'une union non vide (une famille d'unions vides ne couvre pas).
        let Some(n) = regions
            .iter()
            .find(|region| !region.est_vide())
            .and_then(|region| region.zones().first())
            .map(Dbm::nombre_variables)
        else {
            return false;
        };
        for region in regions {
            for zone in region.zones() {
                assert_eq!(
                    zone.nombre_variables(),
                    n,
                    "couverture d'une famille hétérogène (invariant interne)"
                );
            }
        }
        let taille = self.taille_domaine;
        // Balayage d'univers de la boîte bornée (odomètre sur n variables).
        let mut point = vec![0u64; n];
        loop {
            if !regions.iter().any(|region| region.contient(&point)) {
                return false;
            }
            // Incrémentation odométrique.
            let mut i = n;
            loop {
                if i == 0 {
                    return true;
                }
                i -= 1;
                point[i] += 1;
                if point[i] < taille {
                    break;
                }
                point[i] = 0;
            }
        }
    }
}

impl KeyAlgebra for CddKeys {
    type Key = Cdd;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        Cdd::nouveau(key.zones())
    }

    fn cardinality(&self, _level: Level) -> u64 {
        self.taille_domaine
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(VIDE)
    }
}

/// Contrôleur à clés-régions CDD : payload booléen, `D_δ` = union vide →
/// `false` (miroir de la présence `Set` : l'arc absent mène à
/// `terminal(false)` et R-default le supprime).
pub struct CddController {
    keys: CddKeys,
}

impl CddController {
    /// Contrôleur pour l'algèbre de clés-régions CDD donnée.
    pub fn nouveau(keys: CddKeys) -> Self {
        CddController { keys }
    }
}

impl Controller for CddController {
    type Key = Cdd;
    type Payload = bool;
    type Keys = CddKeys;

    const FINGERPRINT: ControllerId = ControllerId(6);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

/// Évalue un CDD sur un point du domaine : à chaque nœud, suit l'arc dont
/// l'union contient le point. `Ok(None)` si aucun arc ne couvre le point :
/// l'arc absent vaut δ(ℓ), l'« absent » continu.
pub fn evaluer<P: Pool<Key = Cdd, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    point: &[u64],
) -> Result<Option<bool>, PoolError> {
    let mut noeud = racine;
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(payload) => return Ok(Some(*payload)),
            Form::Node { arcs, .. } => {
                match arcs
                    .iter()
                    .find(|(region, _)| region.contient(point))
                    .map(|(_, cible)| *cible)
                {
                    Some(cible) => noeud = cible,
                    None => return Ok(None),
                }
            }
        }
    }
}

/// Compte les nœuds distincts atteignables depuis `racine`, terminaux compris.
pub fn compter_noeuds<P: Pool<Key = Cdd, Payload = bool>>(pool: &P, racine: NodeId) -> u64 {
    let mut visite = BTreeSet::new();
    let mut pile = vec![racine];
    while let Some(noeud) = pile.pop() {
        if !visite.insert(noeud) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(noeud) {
            pile.extend(arcs.values().copied());
        }
    }
    visite.len() as u64
}

/// Instance-test : la région non convexe `{x − y ≤ 0} ∪ {x − y ≥ 2}` sur deux
/// horloges — ce qu'aucune DBM seule n'exprime — compilée en un seul niveau
/// par régions explicites (aucun balayage d'univers à la construction) :
///
/// - l'arc d'union non convexe mène à `true` ;
/// - l'arc complémentaire `{x − y = 1}` (la bande ouverte `0 < x − y < 2`
///   sur le domaine entier borné) mène à `false` ; les deux arcs couvrent la
///   boîte.
pub fn compiler_union_horloges<P: Pool<Key = Cdd, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let faux = pool.intern(anchor, Form::Terminal(false));
    let zone_a = Dbm::nouveau(2, &[(0, 1, 0)]);
    let zone_b = Dbm::nouveau(2, &[(1, 0, -2)]);
    let union = Cdd::nouveau(&[zone_a, zone_b]);
    let complementaire = Cdd::nouveau(&[Dbm::nouveau(2, &[(1, 0, -1), (0, 1, 1)])]);
    pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(union, vrai), (complementaire, faux)]),
        },
    )
}
