//! **CDD** instance-test (*Clock Decision Diagrams*, UPPAAL school, Larsen and
//! al.; backlog § 9 of the roadmap, open question 9 of the spec,
//! thread #83 after the interval, DBM fragments and the
//! DDD-difference instance-test): **not-convex unions of DBM zones** as
//! clock region-keys — the "δ = clock differences, not-convex sets" case
//! of the state of the art.
//!
//! Spec Q9 (i): "not-convex unions with no known normal form" — the
//! not-canonicity is **assumed** (DDD style): the normal form of the union
//! is the sort + dedup of the closed zones (syntactic equality of the
//! normalized writings), not set equality. Two writings of the same
//! not-convex set (for example a zone included in another) can
//! remain distinct keys: C1 holds relative to this normal
//! form, not to the denoted set. Point membership, for its part, is
//! semantically decidable (membership in one of the zones).
//!
//! The key contract (normal form, coverage, intersection) remains declared
//! on the controller side (`Cdd`, `CddKeys`); the pool does not change. The
//! universe sweep only compiles if the universe is bounded to enumerable
//! regions (spec Q9): diagram construction goes through explicit regions,
//! never through point enumeration — free DDD kingdom assumed on the
//! continuum.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

use crate::dbm::Dbm;

/// Canonical empty union: zero zones — the continuous "absent" δ(ℓ).
pub const EMPTY: Cdd = Cdd { zones: Vec::new() };

/// Non-convex region-key: finite union of closed DBM zones over
/// `number_variables` clocks. Every construction goes through [`Cdd::new`],
/// which normalizes: empty zones removed, sort, dedup — the *syntactic*
/// normal form of the union (assumed not-canonicity, spec Q9 (i)).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Cdd {
    zones: Vec<Dbm>,
}

impl Cdd {
    /// Union of the given `zones`, normalized (empties removed, sort, dedup).
    pub fn new(zones: &[Dbm]) -> Self {
        let mut normalisees: Vec<Dbm> = zones
            .iter()
            .filter(|zone| !zone.is_empty())
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
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Does the point `point[i] = x_i` belong to the union (semantics,
    /// decidable: membership in one of the zones)?
    pub fn contains(&self, point: &[u64]) -> bool {
        self.zones.iter().any(|zone| zone.contains(point))
    }

    /// Intersection (meet) of two unions: distributivity — the union of the
    /// pointwise meets of the zones, then normalization (empty meets
    /// are removed). The meet of two zone unions is a union of
    /// zones: the lattice is closed.
    pub fn intersection(&self, other: &Cdd) -> Cdd {
        let mut zones: Vec<Dbm> = Vec::new();
        for a in &self.zones {
            for b in &other.zones {
                let meet = a.intersection(b);
                if !meet.is_empty() {
                    zones.push(meet);
                }
            }
        }
        Cdd::new(&zones)
    }
}

/// CDD region-key algebra over a per-variable bounded domain:
/// `Val(ℓ) = [0, size_domain)`, a key = a finite union of DBM zones of
/// the box, `δ(ℓ)` = the empty union [`EMPTY`].
#[derive(Clone)]
pub struct CddKeys {
    size_domain: u64,
}

impl CddKeys {
    /// Algebra for variables with bounded domain `[0, size_domain)`.
    pub fn new(size_domain: u64) -> Self {
        CddKeys { size_domain }
    }

    /// Size of a variable's domain, `Val(ℓ) = [0, size)`.
    pub fn size_domain(&self) -> u64 {
        self.size_domain
    }

    /// Coverage: does the family of unions cover the whole bounded box
    /// `[0, size_domain)^n` (n = dimension of the zones) ?
    ///
    /// Key contract declared on the controller side (roadmap § 9:
    /// "normal form, coverage, intersection"). Decidable by
    /// universe sweep **only if the universe is bounded to enumerable
    /// regions** (spec Q9): on the continuous domain, the sweep does
    /// not compiled and the free DDD kingdom is assumed — coverage is
    /// then declared by the explicit edges of the diagram.
    pub fn coverage(&self, regions: &[Cdd]) -> bool {
        // The empty union covers nothing; the dimension is that of the zones
        // of a not-empty union (a family of empty unions covers nothing).
        let Some(n) = regions
            .iter()
            .find(|region| !region.is_empty())
            .and_then(|region| region.zones().first())
            .map(Dbm::number_variables)
        else {
            return false;
        };
        for region in regions {
            for zone in region.zones() {
                assert_eq!(
                    zone.number_variables(),
                    n,
                    "coverage of a heterogeneous family (internal invariant)"
                );
            }
        }
        let size = self.size_domain;
        // Universe sweep of the bounded box (odometer over n variables).
        let mut point = vec![0u64; n];
        loop {
            if !regions.iter().any(|region| region.contains(&point)) {
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
                if point[i] < size {
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
        Cdd::new(key.zones())
    }

    fn cardinality(&self, _level: Level) -> u64 {
        self.size_domain
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(EMPTY)
    }
}

/// CDD region-key controller: Boolean payload, `D_δ` = empty union →
/// `false` (mirror of `Set` presence: the absent edge leads to
/// `terminal(false)` and R-default removes it).
pub struct CddController {
    keys: CddKeys,
}

impl CddController {
    /// Controller for the given CDD region-key algebra.
    pub fn new(keys: CddKeys) -> Self {
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

/// Evaluates a CDD on a point of the domain: at each node, follows the edge whose
/// the union contains the point. `Ok(None)` if no edge covers the point:
/// the absent edge is δ(ℓ), the continuous "absent".
pub fn evaluer<P: Pool<Key = Cdd, Payload = bool>>(
    pool: &P,
    root: NodeId,
    point: &[u64],
) -> Result<Option<bool>, PoolError> {
    let mut node = root;
    loop {
        match pool.resolve(node)? {
            Form::Terminal(payload) => return Ok(Some(*payload)),
            Form::Node { edges, .. } => {
                match edges
                    .iter()
                    .find(|(region, _)| region.contains(point))
                    .map(|(_, target)| *target)
                {
                    Some(target) => node = target,
                    None => return Ok(None),
                }
            }
        }
    }
}

/// Counts the distinct nodes reachable from `root`, terminals
/// included. Propagates the pool error: a `NodeId` not interned is a
/// error, not a silent leaf.
pub fn count_nodes<P: Pool<Key = Cdd, Payload = bool>>(
    pool: &P,
    root: NodeId,
) -> Result<u64, PoolError> {
    let mut visit = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !visit.insert(node) {
            continue;
        }
        if let Form::Node { edges, .. } = pool.resolve(node)? {
            stack.extend(edges.values().copied());
        }
    }
    Ok(visit.len() as u64)
}

/// Instance-test: the not-convex region `{x − y ≤ 0} ∪ {x − y ≥ 2}` over two
/// clocks — what no single DBM expresses — compiled in a single level
/// by explicit regions (no universe sweep at construction):
///
/// - the not-convex union edge leads to `true`;
/// - the complementary edge `{x − y = 1}` (the open band `0 < x − y < 2`
///   over the whole bounded domain) leads to `false`; the two edges cover
///   the box.
pub fn compile_union_clocks<P: Pool<Key = Cdd, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
) -> NodeId {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let false_node = pool.intern(anchor, Form::Terminal(false));
    let zone_a = Dbm::new(2, &[(0, 1, 0)]);
    let zone_b = Dbm::new(2, &[(1, 0, -2)]);
    let union = Cdd::new(&[zone_a, zone_b]);
    let complementary = Cdd::new(&[Dbm::new(2, &[(1, 0, -1), (0, 1, 1)])]);
    pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(union, true_node), (complementary, false_node)]),
        },
    )
}
