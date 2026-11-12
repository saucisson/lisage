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
//! bornes `i64` ne peut pas déborder. La mesure de Lebesgue 2D (aire de la
//! zone ∩ boîte, [`DbmKeys::mesure_zone`]) et l'intégrale au pli sur clés
//! DBM (`crate::integrale::integrale_dbm_constante`) complètent ce fragment
//! côté mesure ; le semi-anneau à densités polynomiales reste hors fragment.

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

    /// Matrice des bornes `matrice[i][j]` = `Some(c)` pour `x_i − x_j ≤ c`.
    pub fn matrice(&self) -> &[Vec<Option<i128>>] {
        &self.matrice
    }

    /// La région est-elle vide (incohérente) ?
    pub fn est_vide(&self) -> bool {
        *self == INCOHERENT
    }

    /// Intersection (meet) de deux régions : borne `x_i − x_j ≤ min(c, c')`
    /// point par point, puis clôture canonique. L'intersection de deux
    /// régions convexes est convexe : le meet de deux DBM clôturées est
    /// déjà clôturé, la fermeture est idempotente. Toute région vide
    /// donne [`INCOHERENT`].
    pub fn intersection(&self, autre: &Dbm) -> Dbm {
        if self.est_vide() || autre.est_vide() {
            return INCOHERENT;
        }
        assert_eq!(
            self.nombre_variables, autre.nombre_variables,
            "intersection de régions de dimensions différentes (invariant interne)"
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

    /// Couverture : la famille de régions couvre-t-elle toute la boîte
    /// bornée `[0, taille_domaine)^n` (n = dimension des régions) ?
    ///
    /// Contrat de clés déclaré côté contrôleur (feuille de route § 9 :
    /// « forme normale, couverture, intersection »). Décidable par
    /// balayage d'univers **seulement si l'univers est borné à régions
    /// énumérables** (spec Q9) : sur le domaine continu, le balayage ne
    /// compile pas et le royaume libre DDD est assumé — la couverture se
    /// déclare alors par les arcs explicites du diagramme, elle ne se
    /// calcule pas.
    /// **Mesure de Lebesgue** de la zone intersectée à la boîte
    /// `[0, taille_domaine)²`, rendue **doublée** (aire × 2, exacte en
    /// `i128` : l'aire peut être demi-entière). Réservé aux zones **2D** :
    /// une DBM à 2 variables est une bande `−b ≤ x − y ≤ a` (les seules
    /// contraintes hors diagonale sont `matrice[0][1]` et `matrice[1][0]`),
    /// et l'aire de la bande ∩ carré `[0, N]²` vaut `f(−b) − f(a)` avec
    /// `f(t)` = aire de `{x − y > t}` : 0 si `t ≥ N`, `N²` si `t ≤ −N`,
    /// `(N − t)²/2` si `0 ≤ t < N`, `N² − (N + t)²/2` si `−N < t < 0`.
    /// Zone vide → 0. (Spec Q9 (ii) :
    /// « régions polyédriques » — le cas 2D exact ; n > 2 hors fragment.)
    pub fn mesure_zone(&self, zone: &Dbm) -> i128 {
        if zone.est_vide() {
            return 0;
        }
        assert_eq!(
            zone.nombre_variables(),
            2,
            "mesure exacte réservée aux zones 2D (invariant interne)"
        );
        let n = i128::from(self.taille_domaine);
        // Aire doublée de {x − y > t} dans le carré [0, N]² : triangle de
        // côtés N − t si t ≥ 0, carré moins triangle de côtés N + t si
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
        // Aire de la bande −b ≤ x−y ≤ a = {x−y > −b} moins {x−y > a}.
        let aire_doublee = match b {
            None => 2 * n * n,
            Some(b) => au_dessus(-b),
        } - match a {
            None => 0,
            Some(a) => au_dessus(a),
        };
        debug_assert!(
            aire_doublee >= 0,
            "aire négative : zone non clôturée (invariant interne)"
        );
        aire_doublee.max(0)
    }

    pub fn couverture(&self, regions: &[Dbm]) -> bool {
        // La région vide ne couvre rien ; la dimension est celle des
        // régions non vides (une famille de régions vides ne couvre pas).
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
