//! Region-key algebra, **DBM** fragment (backlog § 9 of the roadmap,
//! open question 9 of the spec, thread #83 after the previous
//! interval fragment #85): the edge key is a **difference bound matrix**
//! (DBM), a convex region of the bounded box `[0, N)^n`.
//!
//! Canonical normal form = **shortest-path closure**
//! (Floyd-Warshall): two writings of the same region have the same closure,
//! equality is decidable, C1 holds relative to the region algebra.
//! An inconsistent region (negative diagonal after closure) is normalized
//! to the canonical [`INCONSISTENT`], the empty region — the continuous "absent" δ(ℓ).
//!
//! The bounds are stored as `i128` internally: the closure sum of the
//! `i64` bounds cannot overflow. The 2D Lebesgue measure (area of the
//! zone ∩ box, [`DbmKeys::measure_zone`]) and the fold integral over DBM
//! keys (`crate::integral::integral_dbm_constant`) complete this fragment
//! on the measure side; the semiring with polynomial densities stays out of scope.

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level};

/// Number of variables of a null DBM (canonical empty region without variable).
pub const INCONSISTENT: Dbm = Dbm {
    number_variables: 0,
    matrix: Vec::new(),
};

/// Convex region key: DBM over `number_variables` variables, `matrix[i][j]`
/// = bound `Some(c)` of the constraint `x_i − x_j ≤ c` (`None` = no
/// constraint). Every construction goes through [`Dbm::new`], which sets the
/// diagonal to `x_i − x_i ≤ 0` and normalizes: canonical closure, and
/// inconsistent region → [`INCONSISTENT`]. The derived equality is then the
/// equality of the regions.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Dbm {
    number_variables: usize,
    matrix: Vec<Vec<Option<i128>>>,
}

impl Dbm {
    /// DBM over `number_variables` variables from constraints
    /// `(i, j, c)` meaning `x_i − x_j ≤ c`, then normalization (canonical
    /// closure; inconsistency → [`INCONSISTENT`]).
    pub fn new(number_variables: usize, constraints: &[(usize, usize, i64)]) -> Self {
        let n = number_variables;
        let mut matrix = vec![vec![None; n]; n];
        for (i, line) in matrix.iter_mut().enumerate() {
            line[i] = Some(0);
        }
        for &(i, j, c) in constraints {
            assert!(
                i < n && j < n,
                "constraint outside domain (internal invariant)"
            );
            let bound = i128::from(c);
            matrix[i][j] = Some(match matrix[i][j] {
                Some(previousne) => previousne.min(bound),
                None => bound,
            });
        }
        Self::close(n, matrix)
    }

    /// Normal form of the region: closure of the current matrix (fixpoint
    /// if already normalized), inconsistency → [`INCONSISTENT`].
    fn normaliser(&self) -> Self {
        if self.is_empty() {
            return INCONSISTENT;
        }
        Self::close(self.number_variables, self.matrix.clone())
    }

    /// Shortest-path closure (Floyd-Warshall); if a diagonal
    /// becomes negative, the region is empty → [`INCONSISTENT`].
    fn close(number_variables: usize, mut matrix: Vec<Vec<Option<i128>>>) -> Self {
        let n = number_variables;
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if let (Some(a), Some(b)) = (matrix[i][k], matrix[k][j]) {
                        let sum = a + b;
                        matrix[i][j] = Some(match matrix[i][j] {
                            Some(previousne) => previousne.min(sum),
                            None => sum,
                        });
                    }
                }
            }
        }
        if (0..n).any(|i| matches!(matrix[i][i], Some(d) if d < 0)) {
            return INCONSISTENT;
        }
        Dbm {
            number_variables: n,
            matrix,
        }
    }

    /// Number of variables of the region.
    pub fn number_variables(&self) -> usize {
        self.number_variables
    }

    /// Matrix of bounds `matrix[i][j]` = `Some(c)` for `x_i − x_j ≤ c`.
    pub fn matrix(&self) -> &[Vec<Option<i128>>] {
        &self.matrix
    }

    /// Is the region empty (inconsistent)?
    pub fn is_empty(&self) -> bool {
        *self == INCONSISTENT
    }

    /// Intersection (meet) of two regions: bound `x_i − x_j ≤ min(c, c')`
    /// point by point, then canonical closure. The intersection of two
    /// convex regions is convex: the meet of two closed DBMs is
    /// already closed, the closure is idempotent. Any empty region
    /// donne [`INCONSISTENT`].
    pub fn intersection(&self, other: &Dbm) -> Dbm {
        if self.is_empty() || other.is_empty() {
            return INCONSISTENT;
        }
        assert_eq!(
            self.number_variables, other.number_variables,
            "intersection of regions of different dimensions (internal invariant)"
        );
        let n = self.number_variables;
        let matrix: Vec<Vec<Option<i128>>> = self
            .matrix
            .iter()
            .zip(&other.matrix)
            .map(|(line_a, line_b)| {
                line_a
                    .iter()
                    .zip(line_b)
                    .map(|(a, b)| match (a, b) {
                        (Some(a), Some(b)) => Some((*a).min(*b)),
                        (Some(a), None) => Some(*a),
                        (None, Some(b)) => Some(*b),
                        (None, None) => None,
                    })
                    .collect()
            })
            .collect();
        Self::close(n, matrix)
    }

    /// Does the point `point[i] = x_i` belong to the region?
    pub fn contains(&self, point: &[u64]) -> bool {
        if self.is_empty() {
            return false;
        }
        assert_eq!(
            point.len(),
            self.number_variables,
            "point of wrong dimension (internal invariant)"
        );
        (0..self.number_variables).all(|i| {
            (0..self.number_variables).all(|j| match self.matrix[i][j] {
                Some(c) => i128::from(point[i]) - i128::from(point[j]) <= c,
                None => true,
            })
        })
    }
}

/// DBM region-key algebra over a per-variable bounded domain:
/// `Val(ℓ) = [0, size_domain)`, a key = a convex region of the box,
/// `δ(ℓ)` = the empty region [`INCONSISTENT`].
#[derive(Clone)]
pub struct DbmKeys {
    size_domain: u64,
}

impl DbmKeys {
    /// Algebra for variables with bounded domain `[0, size_domain)`.
    pub fn new(size_domain: u64) -> Self {
        DbmKeys { size_domain }
    }

    /// Size of a variable's domain, `Val(ℓ) = [0, size)`.
    pub fn size_domain(&self) -> u64 {
        self.size_domain
    }

    /// Coverage: does the family of regions cover the whole bounded
    /// box `[0, size_domain)^n` (n = dimension of the regions)?
    ///
    /// Key contract declared on the controller side (roadmap § 9:
    /// "normal form, coverage, intersection"). Decidable by
    /// universe sweep **only if the universe is bounded to enumerable
    /// regions** (spec Q9): on the continuous domain, the sweep does
    /// not compiled and the free DDD kingdom is assumed — coverage is
    /// then declared by the explicit edges of the diagram, it is not
    /// computed.
    /// **Lebesgue measure** of the zone intersected with the box
    /// `[0, size_domain)²`, returned **doubled** (area × 2, exact in
    /// `i128`: the area can be half-integer). Reserved for **2D** zones:
    /// a 2-variable DBM is a band `−b ≤ x − y ≤ a` (the only
    /// off-diagonal constraints are `matrix[0][1]` and `matrix[1][0]`),
    /// and the area of the band ∩ square `[0, N]²` is `f(−b) − f(a)` with
    /// `f(t)` = area of `{x − y > t}`: 0 if `t ≥ N`, `N²` if `t ≤ −N`,
    /// `(N − t)²/2` if `0 ≤ t < N`, `N² − (N + t)²/2` if `−N < t < 0`.
    /// Empty zone → 0. (Spec Q9 (ii):
    /// "polyhedral regions" — the exact 2D case; n > 2 out of scope.)
    pub fn measure_zone(&self, zone: &Dbm) -> i128 {
        if zone.is_empty() {
            return 0;
        }
        assert_eq!(
            zone.number_variables(),
            2,
            "exact measure reserved for 2D zones (internal invariant)"
        );
        let n = i128::from(self.size_domain);
        // Doubled area of {x − y > t} in the square [0, N]²: triangle with
        // sides N − t if t ≥ 0, square minus triangle with sides N + t if
        // t < 0.
        let above = |t: i128| -> i128 {
            if t >= n {
                0
            } else if t <= -n {
                2 * n * n
            } else if t >= 0 {
                (n - t) * (n - t)
            } else {
                2 * n * n - (n + t) * (n + t)
            }
        };
        let a = zone.matrix()[0][1];
        let b = zone.matrix()[1][0];
        // Area of the band −b ≤ x−y ≤ a = {x−y > −b} minus {x−y > a}.
        let area_doubled = match b {
            None => 2 * n * n,
            Some(b) => above(-b),
        } - match a {
            None => 0,
            Some(a) => above(a),
        };
        debug_assert!(
            area_doubled >= 0,
            "negative area: zone not closed (internal invariant)"
        );
        area_doubled.max(0)
    }

    pub fn coverage(&self, regions: &[Dbm]) -> bool {
        // The empty region covers nothing; the dimension is that of the
        // not-empty regions (a family of empty regions covers nothing).
        let Some(n) = regions
            .iter()
            .find(|region| !region.is_empty())
            .map(Dbm::number_variables)
        else {
            return false;
        };
        for region in regions {
            if !region.is_empty() {
                assert_eq!(
                    region.number_variables(),
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

impl KeyAlgebra for DbmKeys {
    type Key = Dbm;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        key.normaliser()
    }

    fn cardinality(&self, _level: Level) -> u64 {
        self.size_domain
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(INCONSISTENT)
    }
}

/// DBM region-key controller: Boolean payload, `D_δ` = empty region →
/// `false` (mirror of `Set` presence: the absent edge leads to
/// `terminal(false)` and R-default removes it).
pub struct DbmController {
    keys: DbmKeys,
}

impl DbmController {
    /// Controller for the given DBM region-key algebra.
    pub fn new(keys: DbmKeys) -> Self {
        DbmController { keys }
    }
}

impl Controller for DbmController {
    type Key = Dbm;
    type Payload = bool;
    type Keys = DbmKeys;

    const FINGERPRINT: ControllerId = ControllerId(5);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}
