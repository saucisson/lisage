//! Algèbre de clés-régions, fragment **intervalles** (backlog § 9 de la
//! feuille de route, question ouverte 9 de la spec) : la clé d'arc est un
//! intervalle borné sur le domaine d'un niveau — une région convexe 1D,
//! premier cas de « intervalles puis DBM ».
//!
//! Le domaine d'un niveau reste fini (`[0, taille)`) ; la clé décrit un
//! *ensemble de points* du domaine, avec égalité décidable et forme normale
//! canonique : C1 devient relatif à l'algèbre de régions. L'intervalle vide
//! est l'« absent » continu δ(ℓ), miroir de `ABSENT` pour `Set`. Le pli sur
//! dimension continue (intégrale, semi-anneau à mesures) et les DBM restent
//! hors de ce fragment.

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level};

/// Intervalle vide canonique : `borne_inf > borne_sup`, normalisé à `(1, 0)`.
pub const VIDE: Intervalle = Intervalle {
    borne_inf: 1,
    borne_sup: 0,
};

/// Clé-région : intervalle fermé `[borne_inf, borne_sup]` de points d'un
/// domaine borné. Toute construction passe par [`Intervalle::nouveau`], qui
/// normalise l'intervalle vide à [`VIDE`] : l'égalité dérivée est alors
/// l'égalité ensembliste, et le hash-consing respecte C1 relativement à
/// l'algèbre de régions.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Intervalle {
    borne_inf: u64,
    borne_sup: u64,
}

impl Intervalle {
    /// Intervalle fermé `[borne_inf, borne_sup]` ; si `borne_inf > borne_sup`,
    /// l'intervalle est vide et normalisé à [`VIDE`].
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

    /// Intervalle réduit au point `v`.
    pub fn point(v: u64) -> Self {
        Intervalle::nouveau(v, v)
    }

    /// Borne inférieure, incluse.
    pub fn borne_inf(&self) -> u64 {
        self.borne_inf
    }

    /// Borne supérieure, incluse.
    pub fn borne_sup(&self) -> u64 {
        self.borne_sup
    }

    /// L'intervalle est-il vide ?
    pub fn est_vide(&self) -> bool {
        self.borne_inf > self.borne_sup
    }

    /// Nombre de points de l'intervalle (0 si vide).
    pub fn taille(&self) -> u64 {
        if self.est_vide() {
            0
        } else {
            self.borne_sup - self.borne_inf + 1
        }
    }

    /// Le point `v` appartient-il à l'intervalle ?
    pub fn contient(&self, v: u64) -> bool {
        !self.est_vide() && self.borne_inf <= v && v <= self.borne_sup
    }
}

/// Algèbre de clés-régions sur des domaines bornés par niveau :
/// `Val(ℓ) = [0, domaines[ℓ])`, une clé = un intervalle de points de ce
/// domaine, `δ(ℓ)` = l'intervalle vide.
#[derive(Clone)]
pub struct RegionKeys {
    domaines: Vec<u64>,
}

impl RegionKeys {
    /// Algèbre pour les tailles de domaine par niveau (index = niveau).
    pub fn nouvelle(domaines: Vec<u64>) -> Self {
        RegionKeys { domaines }
    }

    /// Taille du domaine du niveau `ℓ`, c'est-à-dire `Val(ℓ) = [0, taille)`.
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

/// Contrôleur à clés-régions : payload booléen, `D_δ` = région vide →
/// `false` (miroir de la présence `Set` : l'arc absent mène à
/// `terminal(false)` et R-default le supprime).
pub struct RegionController {
    keys: RegionKeys,
}

impl RegionController {
    /// Contrôleur pour l'algèbre de clés-régions donnée.
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
