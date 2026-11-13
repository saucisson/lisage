//! Region-key algebra, **interval** fragment (backlog § 9 of the
//! roadmap, open question 9 of the spec): the edge key is a
//! bounded interval over the domain of a level — a 1D convex region,
//! first case of "intervals then DBM".
//!
//! The domain of a level stays finite (`[0, size)`); the key describes a
//! *set of points* of the domain, with decidable equality and
//! canonical normal form: C1 becomes relative to the region algebra. The empty
//! interval is the continuous "absent" δ(ℓ), mirror of `ABSENT` for `Set`. The fold over
//! continuous dimension (integral, measure semiring) and DBMs remain
//! outside this fragment.

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level};

/// Canonical empty interval: `bound_inf > bound_sup`, normalized to `(1, 0)`.
pub const EMPTY: Interval = Interval {
    bound_inf: 1,
    bound_sup: 0,
};

/// Region key: closed interval `[bound_inf, bound_sup]` of points of a
/// bounded domain. Every construction goes through [`Interval::new`], which
/// normalizes the empty interval to [`EMPTY`]: the derived equality is then
/// set equality, and hash-consing respects C1 relative to
/// the region algebra.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Interval {
    bound_inf: u64,
    bound_sup: u64,
}

impl Interval {
    /// Closed interval `[bound_inf, bound_sup]`; if `bound_inf > bound_sup`,
    /// the interval is empty and normalized to [`EMPTY`].
    pub fn new(bound_inf: u64, bound_sup: u64) -> Self {
        if bound_inf > bound_sup {
            EMPTY
        } else {
            Interval {
                bound_inf,
                bound_sup,
            }
        }
    }

    /// Interval reduced to the point `v`.
    pub fn point(v: u64) -> Self {
        Interval::new(v, v)
    }

    /// Lower bound, included.
    pub fn bound_inf(&self) -> u64 {
        self.bound_inf
    }

    /// Upper bound, included.
    pub fn bound_sup(&self) -> u64 {
        self.bound_sup
    }

    /// Is the interval empty?
    pub fn is_empty(&self) -> bool {
        self.bound_inf > self.bound_sup
    }

    /// Number of points of the interval (0 if empty); saturated at `u64::MAX`
    /// for `Interval::new(0, u64::MAX)` (the public constructor
    /// accepts any pair of `u64`).
    pub fn size(&self) -> u64 {
        if self.is_empty() {
            0
        } else {
            self.bound_sup
                .saturating_sub(self.bound_inf)
                .saturating_add(1)
        }
    }

    /// Does the point `v` belong to the interval?
    pub fn contains(&self, v: u64) -> bool {
        !self.is_empty() && self.bound_inf <= v && v <= self.bound_sup
    }
}

/// Region-key algebra over per-level bounded domains:
/// `Val(ℓ) = [0, domains[ℓ])`, a key = an interval of points of this
/// domain, `δ(ℓ)` = the empty interval.
#[derive(Clone)]
pub struct RegionKeys {
    domaines: Vec<u64>,
}

impl RegionKeys {
    /// Algebra for the domain sizes per level (index = level).
    pub fn new(domaines: Vec<u64>) -> Self {
        RegionKeys { domaines }
    }

    /// Size of the domain of level `ℓ`, i.e. `Val(ℓ) = [0, size)`.
    pub fn size_domain(&self, level: Level) -> u64 {
        self.domaines[level.0 as usize]
    }
}

impl KeyAlgebra for RegionKeys {
    type Key = Interval;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        Interval::new(key.bound_inf(), key.bound_sup())
    }

    fn cardinality(&self, level: Level) -> u64 {
        self.size_domain(level)
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(EMPTY)
    }
}

/// Region-key controller: Boolean payload, `D_δ` = empty region →
/// `false` (mirror of `Set` presence: the absent edge leads to
/// `terminal(false)` and R-default removes it).
pub struct RegionController {
    keys: RegionKeys,
}

impl RegionController {
    /// Controller for the given region-key algebra.
    pub fn new(keys: RegionKeys) -> Self {
        RegionController { keys }
    }
}

impl Controller for RegionController {
    type Key = Interval;
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
