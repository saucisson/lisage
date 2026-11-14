//! Region-key algebra, **DBM** fragment (backlog § 9 of the roadmap, open
//! question 9 of the spec, vein #83 after the intervals fragment
//! #85-previous): the arc key is a **difference bound matrix** (DBM), a
//! convex region of the bounded box `[0, N)^n`.
//!
//! Canonical normal form = **shortest-path closure**
//! (Floyd-Warshall): two writings of the same region have the same closure,
//! equality is decidable, C1 holds relative to the region algebra.
//! An inconsistent region (negative diagonal after closure) is normalized
//! to the canonical [`INCOHERENT`], the empty region — the continuous
//! "absent" δ(ℓ).
//!
//! Bounds are stored as `i128` internally: the closure sum of `i64`
//! bounds cannot overflow. The 2D Lebesgue measure (area of the
//! zone ∩ box, [`DbmKeys::mesure_zone`]) and the fold integral over DBM
//! keys (`crate::integrale::integrale_dbm_constante`) complete this fragment
//! on the measure side; the semiring with polynomial densities remains
//! outside the fragment.

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level};

/// Number of variables of a null DBM (canonical empty region without variable).
pub const INCOHERENT: Dbm = Dbm {
    nombre_variables: 0,
    matrice: Vec::new(),
};

/// Convex region-key: DBM over `nombre_variables` variables, `matrice[i][j]`
/// = bound `Some(c)` of the constraint `x_i − x_j ≤ c` (`None` = no
/// constraint). Every construction goes through [`Dbm::nouveau`], which sets
/// the diagonal to `x_i − x_i ≤ 0` and normalizes: canonical closure, and
/// inconsistent region → [`INCOHERENT`]. The derived equality is then the
/// equality of regions.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Dbm {
    nombre_variables: usize,
    matrice: Vec<Vec<Option<i128>>>,
}

impl Dbm {
    /// DBM over `nombre_variables` variables from constraints
    /// `(i, j, c)` meaning `x_i − x_j ≤ c`, then normalization (canonical
    /// closure; inconsistency → [`INCOHERENT`]).
    pub fn nouveau(nombre_variables: usize, contraintes: &[(usize, usize, i64)]) -> Self {
        let n = nombre_variables;
        let mut matrice = vec![vec![None; n]; n];
        for (i, ligne) in matrice.iter_mut().enumerate() {
            ligne[i] = Some(0);
        }
        for &(i, j, c) in contraintes {
            assert!(
                i < n && j < n,
                "constraint outside domain (internal invariant)"
            );
            let borne = i128::from(c);
            matrice[i][j] = Some(match matrice[i][j] {
                Some(ancienne) => ancienne.min(borne),
                None => borne,
            });
        }
        Self::fermer(n, matrice)
    }

    /// Normal form of the region: closure of the current matrix (fixed
    /// point if already normalized), inconsistency → [`INCOHERENT`].
    fn normaliser(&self) -> Self {
        if self.est_vide() {
            return INCOHERENT;
        }
        Self::fermer(self.nombre_variables, self.matrice.clone())
    }

    /// Shortest-path closure (Floyd-Warshall); if a diagonal
    /// becomes negative, the region is empty → [`INCOHERENT`].
    fn fermer(nombre_variables: usize, mut matrice: Vec<Vec<Option<i128>>>) -> Self {
        let n = nombre_variables;
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if let (Some(a), Some(b)) = (matrice[i][k], matrice[k][j]) {
                        let somme = a + b;
                        matrice[i][j] = Some(match matrice[i][j] {
                            Some(ancienne) => ancienne.min(somme),
                            None => somme,
                        });
                    }
                }
            }
        }
        if (0..n).any(|i| matches!(matrice[i][i], Some(d) if d < 0)) {
            return INCOHERENT;
        }
        Dbm {
            nombre_variables: n,
            matrice,
        }
    }

    /// Number of variables of the region.
    pub fn nombre_variables(&self) -> usize {
        self.nombre_variables
    }

    /// Bound matrix `matrice[i][j]` = `Some(c)` for `x_i − x_j ≤ c`.
    pub fn matrice(&self) -> &[Vec<Option<i128>>] {
        &self.matrice
    }

    /// Is the region empty (inconsistent)?
    pub fn est_vide(&self) -> bool {
        *self == INCOHERENT
    }

    /// Intersection (meet) of two regions: bound `x_i − x_j ≤ min(c, c')`
    /// point by point, then canonical closure. The intersection of two
    /// convex regions is convex: the meet of two closed DBMs is
    /// already closed, the closure is idempotent. Any empty region
    /// gives [`INCOHERENT`].
    pub fn intersection(&self, autre: &Dbm) -> Dbm {
        if self.est_vide() || autre.est_vide() {
            return INCOHERENT;
        }
        assert_eq!(
            self.nombre_variables, autre.nombre_variables,
            "intersection of regions of different dimensions (internal invariant)"
        );
        let n = self.nombre_variables;
        let matrice: Vec<Vec<Option<i128>>> = self
            .matrice
            .iter()
            .zip(&autre.matrice)
            .map(|(ligne_a, ligne_b)| {
                ligne_a
                    .iter()
                    .zip(ligne_b)
                    .map(|(a, b)| match (a, b) {
                        (Some(a), Some(b)) => Some((*a).min(*b)),
                        (Some(a), None) => Some(*a),
                        (None, Some(b)) => Some(*b),
                        (None, None) => None,
                    })
                    .collect()
            })
            .collect();
        Self::fermer(n, matrice)
    }

    /// Does the point `point[i] = x_i` belong to the region?
    pub fn contient(&self, point: &[u64]) -> bool {
        if self.est_vide() {
            return false;
        }
        assert_eq!(
            point.len(),
            self.nombre_variables,
            "point of wrong dimension (internal invariant)"
        );
        (0..self.nombre_variables).all(|i| {
            (0..self.nombre_variables).all(|j| match self.matrice[i][j] {
                Some(c) => i128::from(point[i]) - i128::from(point[j]) <= c,
                None => true,
            })
        })
    }
}

/// DBM region-key algebra over a per-variable bounded domain:
/// `Val(ℓ) = [0, taille_domaine)`, a key = a convex region of the box,
/// `δ(ℓ)` = the empty region [`INCOHERENT`].
#[derive(Clone)]
pub struct DbmKeys {
    taille_domaine: u64,
}

impl DbmKeys {
    /// Algebra for variables with bounded domain `[0, taille_domaine)`.
    pub fn nouvelle(taille_domaine: u64) -> Self {
        DbmKeys { taille_domaine }
    }

    /// Domain size of a variable, `Val(ℓ) = [0, taille)`.
    pub fn taille_domaine(&self) -> u64 {
        self.taille_domaine
    }

    /// Coverage: does the family of regions cover the whole bounded box
    /// `[0, taille_domaine)^n` (n = dimension of the regions)?
    ///
    /// Key contract declared controller-side (roadmap § 9:
    /// "normal form, coverage, intersection"). Decidable by
    /// universe sweep **only if the universe is bounded to
    /// enumerable regions** (spec Q9): on the continuous domain, the sweep
    /// does not compile and the free DDD realm is assumed — coverage is
    /// then declared by the explicit arcs of the diagram, it is not
    /// computed.
    /// **Lebesgue measure** of the zone intersected with the box
    /// `[0, taille_domaine)²`, returned **doubled** (area × 2, exact in
    /// `i128`: the area can be half-integer). Reserved to **2D** zones:
    /// a 2-variable DBM is a band `−b ≤ x − y ≤ a` (the only
    /// off-diagonal constraints are `matrice[0][1]` and `matrice[1][0]`),
    /// and the area of the band ∩ square `[0, N]²` equals `f(−b) − f(a)` with
    /// `f(t)` = area of `{x − y > t}`: 0 if `t ≥ N`, `N²` if `t ≤ −N`,
    /// `(N − t)²/2` if `0 ≤ t < N`, `N² − (N + t)²/2` if `−N < t < 0`.
    /// Empty zone → 0. (Spec Q9 (ii):
    /// "polyhedral regions" — the exact 2D case; n > 2 outside fragment.)
    pub fn mesure_zone(&self, zone: &Dbm) -> i128 {
        if zone.est_vide() {
            return 0;
        }
        assert_eq!(
            zone.nombre_variables(),
            2,
            "exact measure reserved to 2D zones (internal invariant)"
        );
        let n = i128::from(self.taille_domaine);
        // Doubled area of {x − y > t} in the square [0, N]²: triangle of
        // sides N − t if t ≥ 0, square minus triangle of sides N + t if
        // t < 0.
        let au_dessus = |t: i128| -> i128 {
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
        let a = zone.matrice()[0][1];
        let b = zone.matrice()[1][0];
        // Area of the band −b ≤ x−y ≤ a = {x−y > −b} minus {x−y > a}.
        let aire_doublee = match b {
            None => 2 * n * n,
            Some(b) => au_dessus(-b),
        } - match a {
            None => 0,
            Some(a) => au_dessus(a),
        };
        debug_assert!(
            aire_doublee >= 0,
            "negative area: zone not closed (internal invariant)"
        );
        aire_doublee.max(0)
    }

    pub fn couverture(&self, regions: &[Dbm]) -> bool {
        // The empty region covers nothing; the dimension is that of the
        // non-empty regions (a family of empty regions covers nothing).
        let Some(n) = regions
            .iter()
            .find(|region| !region.est_vide())
            .map(Dbm::nombre_variables)
        else {
            return false;
        };
        for region in regions {
            if !region.est_vide() {
                assert_eq!(
                    region.nombre_variables(),
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

impl KeyAlgebra for DbmKeys {
    type Key = Dbm;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        key.normaliser()
    }

    fn cardinality(&self, _level: Level) -> u64 {
        self.taille_domaine
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(INCOHERENT)
    }
}

/// DBM region-key controller: boolean payload, `D_δ` = empty region →
/// `false` (mirror of `Set` presence: the absent arc leads to
/// `terminal(false)` and R-default removes it).
pub struct DbmController {
    keys: DbmKeys,
}

impl DbmController {
    /// Controller for the given DBM region-key algebra.
    pub fn nouveau(keys: DbmKeys) -> Self {
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
