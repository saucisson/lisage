//! Algèbre de clés-régions, fragment **DBM** (backlog § 9 de la feuille de
//! route, question ouverte 9 de la spec, veine #83 après le fragment
//! intervalles #85-précédent) : la clé d'arc est une **matrice de bornes de
//! différences** (DBM), région convexe de la boîte bornée `[0, N)^n`.
//!
//! Forme normale canonique = **clôture par plus courts chemins**
//! (Floyd-Warshall) : deux écritures d'une même région ont la même clôture,
//! l'égalité est décidable, C1 tient relativement à l'algèbre de régions.
//! Une région incohérente (diagonale négative après clôture) est normalisée
//! au canonique [`INCOHERENT`], la région vide — l'« absent » continu δ(ℓ).
//!
//! Les bornes sont stockées en `i128` en interne : la clôture somme des
//! bornes `i64` ne peut pas déborder. Le pli sur dimension continue
//! (intégrale, semi-anneau à mesures) reste hors de ce fragment.

use lisage_algebra::interface::{Controller, ControllerId, KeyAlgebra, Level};

/// Nombre de variables d'une DBM nulle (région vide canonique sans variable).
pub const INCOHERENT: Dbm = Dbm {
    nombre_variables: 0,
    matrice: Vec::new(),
};

/// Clé-région convexe : DBM sur `nombre_variables` variables, `matrice[i][j]`
/// = borne `Some(c)` de la contrainte `x_i − x_j ≤ c` (`None` = pas de
/// contrainte). Toute construction passe par [`Dbm::nouveau`], qui pose la
/// diagonale à `x_i − x_i ≤ 0` et normalise : clôture canonique, et région
/// incohérente → [`INCOHERENT`]. L'égalité dérivée est alors l'égalité des
/// régions.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Dbm {
    nombre_variables: usize,
    matrice: Vec<Vec<Option<i128>>>,
}

impl Dbm {
    /// DBM sur `nombre_variables` variables à partir de contraintes
    /// `(i, j, c)` signifiant `x_i − x_j ≤ c`, puis normalisation (clôture
    /// canonique ; incohérence → [`INCOHERENT`]).
    pub fn nouveau(nombre_variables: usize, contraintes: &[(usize, usize, i64)]) -> Self {
        let n = nombre_variables;
        let mut matrice = vec![vec![None; n]; n];
        for (i, ligne) in matrice.iter_mut().enumerate() {
            ligne[i] = Some(0);
        }
        for &(i, j, c) in contraintes {
            assert!(
                i < n && j < n,
                "contrainte hors domaine (invariant interne)"
            );
            let borne = i128::from(c);
            matrice[i][j] = Some(match matrice[i][j] {
                Some(ancienne) => ancienne.min(borne),
                None => borne,
            });
        }
        Self::fermer(n, matrice)
    }

    /// Forme normale de la région : clôture de la matrice courante (point
    /// fixe si déjà normalisée), incohérence → [`INCOHERENT`].
    fn normaliser(&self) -> Self {
        if self.est_vide() {
            return INCOHERENT;
        }
        Self::fermer(self.nombre_variables, self.matrice.clone())
    }

    /// Clôture par plus courts chemins (Floyd-Warshall) ; si une diagonale
    /// devient négative, la région est vide → [`INCOHERENT`].
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

    /// Nombre de variables de la région.
    pub fn nombre_variables(&self) -> usize {
        self.nombre_variables
    }

    /// La région est-elle vide (incohérente) ?
    pub fn est_vide(&self) -> bool {
        *self == INCOHERENT
    }

    /// Le point `point[i] = x_i` appartient-il à la région ?
    pub fn contient(&self, point: &[u64]) -> bool {
        if self.est_vide() {
            return false;
        }
        assert_eq!(
            point.len(),
            self.nombre_variables,
            "point de mauvaise dimension (invariant interne)"
        );
        (0..self.nombre_variables).all(|i| {
            (0..self.nombre_variables).all(|j| match self.matrice[i][j] {
                Some(c) => i128::from(point[i]) - i128::from(point[j]) <= c,
                None => true,
            })
        })
    }
}

/// Algèbre de clés-régions DBM sur domaine borné par variable :
/// `Val(ℓ) = [0, taille_domaine)`, une clé = une région convexe de la boîte,
/// `δ(ℓ)` = la région vide [`INCOHERENT`].
#[derive(Clone)]
pub struct DbmKeys {
    taille_domaine: u64,
}

impl DbmKeys {
    /// Algèbre pour des variables à domaine borné `[0, taille_domaine)`.
    pub fn nouvelle(taille_domaine: u64) -> Self {
        DbmKeys { taille_domaine }
    }

    /// Taille du domaine d'une variable, `Val(ℓ) = [0, taille)`.
    pub fn taille_domaine(&self) -> u64 {
        self.taille_domaine
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

/// Contrôleur à clés-régions DBM : payload booléen, `D_δ` = région vide →
/// `false` (miroir de la présence `Set` : l'arc absent mène à
/// `terminal(false)` et R-default le supprime).
pub struct DbmController {
    keys: DbmKeys,
}

impl DbmController {
    /// Contrôleur pour l'algèbre de clés-régions DBM donnée.
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
