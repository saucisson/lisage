//! **CDD** (*Clock Decision Diagrams*) test instance (UPPAAL school, Larsen et
//! al.; backlog § 9 of the roadmap, open question 9 of the spec,
//! vein #83 after the intervals, DBM fragments and the
//! DDD-difference test instance): the **non-convex unions of DBM zones** as
//! clock region-keys — the "δ = clock differences, non-convex sets" case of
//! the state of the art.
//!
//! Spec Q9 (i): "non-convex unions with no known normal form" — the
//! non-canonicity is **assumed** (DDD style): the normal form of the union
//! is the sort + dedup of the closed zones (syntactic equality of the
//! normalized writings), not set-theoretic equality. Two writings of the same
//! non-convex set (for example a zone included in another) can
//! remain distinct keys: C1 holds relative to this
//! normal form, not to the denoted set. Point membership, for its part, is
//! semantically decidable (membership in one of the zones).
//!
//! The key contract (normal form, coverage, intersection) remains declared
//! controller-side (`Cdd`, `CddKeys`); the pool does not change. The
//! universe sweep only compiles if the universe is bounded to enumerable
//! regions (spec Q9): the diagram construction goes through explicit regions,
//! never through point enumeration — free DDD realm assumed on the
//! continuum.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

use crate::dbm::Dbm;

/// Canonical empty union: zero zones — the continuous "absent" δ(ℓ).
pub const VIDE: Cdd = Cdd { zones: Vec::new() };

/// Non-convex region-key: finite union of closed DBM zones over
/// `nombre_variables` clocks. Every construction goes through [`Cdd::nouveau`],
/// which normalizes: empty zones removed, sort, dedup — the
/// *syntactic* normal form of the union (assumed non-canonicity, spec Q9 (i)).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Cdd {
    zones: Vec<Dbm>,
}

impl Cdd {
    /// Union of the given `zones`, normalized (empties removed, sort, dedup).
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

    /// The zones of the union, sorted and deduplicated.
    pub fn zones(&self) -> &[Dbm] {
        &self.zones
    }

    /// Is the union empty (zero zones)?
    pub fn est_vide(&self) -> bool {
        self.zones.is_empty()
    }

    /// Does the point `point[i] = x_i` belong to the union (semantics,
    /// decidable: membership in one of the zones)?
    pub fn contient(&self, point: &[u64]) -> bool {
        self.zones.iter().any(|zone| zone.contient(point))
    }

    /// Intersection (meet) of two unions: distributivity — the union of the
    /// point-by-point meets of the zones, then normalization (empty meets
    /// are removed). The meet of two unions of zones is a union of
    /// zones: the lattice is closed.
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

/// CDD region-key algebra over a per-variable bounded domain:
/// `Val(ℓ) = [0, taille_domaine)`, a key = a finite union of DBM zones of
/// the box, `δ(ℓ)` = the empty union [`VIDE`].
#[derive(Clone)]
pub struct CddKeys {
    taille_domaine: u64,
}

impl CddKeys {
    /// Algebra for variables with bounded domain `[0, taille_domaine)`.
    pub fn nouvelle(taille_domaine: u64) -> Self {
        CddKeys { taille_domaine }
    }

    /// Domain size of a variable, `Val(ℓ) = [0, taille)`.
    pub fn taille_domaine(&self) -> u64 {
        self.taille_domaine
    }

    /// Coverage: does the family of unions cover the whole bounded box
    /// `[0, taille_domaine)^n` (n = dimension of the zones)?
    ///
    /// Key contract declared controller-side (roadmap § 9:
    /// "normal form, coverage, intersection"). Decidable by
    /// universe sweep **only if the universe is bounded to
    /// enumerable regions** (spec Q9): on the continuous domain, the sweep
    /// does not compile and the free DDD realm is assumed — coverage is
    /// then declared by the explicit arcs of the diagram.
    pub fn couverture(&self, regions: &[Cdd]) -> bool {
        // The empty union covers nothing; the dimension is that of the zones
        // of a non-empty union (a family of empty unions covers nothing).
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
                    "coverage of a heterogeneous family (internal invariant)"
                );
            }
        }
        let taille = self.taille_domaine;
        // Universe sweep of the bounded box (odometer over n variables).
        let mut point = vec![0u64; n];
        loop {
            if !regions.iter().any(|region| region.contient(&point)) {
                return false;
            }
            // Odometer increment.
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

/// CDD region-key controller: boolean payload, `D_δ` = empty union →
/// `false` (mirror of `Set` presence: the absent arc leads to
/// `terminal(false)` and R-default removes it).
pub struct CddController {
    keys: CddKeys,
}

impl CddController {
    /// Controller for the given CDD region-key algebra.
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

/// Evaluates a CDD on a domain point: at each node, follows the arc whose
/// union contains the point. `Ok(None)` if no arc covers the point:
/// the absent arc is δ(ℓ), the continuous "absent".
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

/// Counts the distinct nodes reachable from `racine`, terminals
/// included. Propagates the pool error: a non-interned `NodeId` is an
/// error, not a silent leaf.
pub fn compter_noeuds<P: Pool<Key = Cdd, Payload = bool>>(
    pool: &P,
    racine: NodeId,
) -> Result<u64, PoolError> {
    let mut visite = BTreeSet::new();
    let mut pile = vec![racine];
    while let Some(noeud) = pile.pop() {
        if !visite.insert(noeud) {
            continue;
        }
        if let Form::Node { arcs, .. } = pool.resolve(noeud)? {
            pile.extend(arcs.values().copied());
        }
    }
    Ok(visite.len() as u64)
}

/// Test instance: the non-convex region `{x − y ≤ 0} ∪ {x − y ≥ 2}` over two
/// clocks — what no single DBM expresses — compiled in a single level
/// by explicit regions (no universe sweep at construction):
///
/// - the non-convex union arc leads to `true`;
/// - the complementary arc `{x − y = 1}` (the open band `0 < x − y < 2`
///   over the bounded integer domain) leads to `false`; the two arcs cover
///   the box.
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
