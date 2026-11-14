//! Region key algebra, **intervals** fragment (backlog § 9 of the roadmap,
//! open question 9 of the spec): the arc key is a bounded interval over a
//! level's domain — a 1D convex region, first case of "intervals then DBM".
//!
//! A level's domain stays finite (`[0, taille)`); the key describes a *set
//! of points* of the domain, with decidable equality and canonical normal
//! form: C1 becomes relative to the region algebra. The empty interval is
//! the continuous "absent" δ(ℓ), mirror of `ABSENT` for `Set`. The fold
//! over a continuous dimension (integral, measure semiring) and DBMs remain
//! outside this fragment.

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level};

/// Canonical empty interval: `borne_inf > borne_sup`, normalized to `(1, 0)`.
pub const VIDE: Intervalle = Intervalle {
    borne_inf: 1,
    borne_sup: 0,
};

/// Region key: closed interval `[borne_inf, borne_sup]` of points of a
/// bounded domain. Every construction goes through [`Intervalle::nouveau`],
/// which normalizes the empty interval to [`VIDE`]: the derived equality is
/// then set equality, and hash-consing respects C1 relative to the region
/// algebra.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Intervalle {
    borne_inf: u64,
    borne_sup: u64,
}

impl Intervalle {
    /// Closed interval `[borne_inf, borne_sup]`; if `borne_inf > borne_sup`,
    /// the interval is empty and normalized to [`VIDE`].
    pub fn nouveau(borne_inf: u64, borne_sup: u64) -> Self {
        if borne_inf > borne_sup {
            VIDE
        } else {
            Intervalle {
                borne_inf,
                borne_sup,
            }
        }
    }

    /// Interval reduced to the point `v`.
    pub fn point(v: u64) -> Self {
        Intervalle::nouveau(v, v)
    }

    /// Lower bound, included.
    pub fn borne_inf(&self) -> u64 {
        self.borne_inf
    }

    /// Upper bound, included.
    pub fn borne_sup(&self) -> u64 {
        self.borne_sup
    }

    /// Is the interval empty?
    pub fn est_vide(&self) -> bool {
        self.borne_inf > self.borne_sup
    }

    /// Number of points of the interval (0 if empty); saturated at
    /// `u64::MAX` for `Intervalle::nouveau(0, u64::MAX)` (the public
    /// constructor accepts any pair of `u64`).
    pub fn taille(&self) -> u64 {
        if self.est_vide() {
            0
        } else {
            self.borne_sup
                .saturating_sub(self.borne_inf)
                .saturating_add(1)
        }
    }

    /// Does the point `v` belong to the interval?
    pub fn contient(&self, v: u64) -> bool {
        !self.est_vide() && self.borne_inf <= v && v <= self.borne_sup
    }
}

/// Region key algebra over per-level bounded domains: `Val(ℓ) = [0,
/// domaines[ℓ])`, a key = an interval of points of that domain, `δ(ℓ)` =
/// the empty interval.
#[derive(Clone)]
pub struct RegionKeys {
    domaines: Vec<u64>,
}

impl RegionKeys {
    /// Algebra for per-level domain sizes (index = level).
    pub fn nouvelle(domaines: Vec<u64>) -> Self {
        RegionKeys { domaines }
    }

    /// Domain size of level `ℓ`, i.e. `Val(ℓ) = [0, taille)`.
    pub fn taille_domaine(&self, level: Level) -> u64 {
        self.domaines[level.0 as usize]
    }
}

impl KeyAlgebra for RegionKeys {
    type Key = Intervalle;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        Intervalle::nouveau(key.borne_inf(), key.borne_sup())
    }

    fn cardinality(&self, level: Level) -> u64 {
        self.taille_domaine(level)
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(VIDE)
    }
}

/// Region-key controller: boolean payload, `D_δ` = empty region → `false`
/// (mirror of `Set` presence: the absent arc leads to `terminal(false)` and
/// R-default removes it).
pub struct RegionController {
    keys: RegionKeys,
}

impl RegionController {
    /// Controller for the given region key algebra.
    pub fn nouveau(keys: RegionKeys) -> Self {
        RegionController { keys }
    }
}

impl Controller for RegionController {
    type Key = Intervalle;
    type Payload = bool;
    type Keys = RegionKeys;

    const FINGERPRINT: ControllerId = ControllerId(4);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}
