//! Oracle de rentabilité analytique (M3-3, énoncé C4) : **premier modèle
//! prédictif** — taille du domaine, densité attendue, coût d'encodage,
//! nombre de requêtes aval — et **garde-fou par différentiel** : la
//! prédiction est comparée à la mesure réelle, à un facteur près (C4 est
//! falsifiable : un modèle qui s'écarte hors bornes est rejeté).
//!
//! Modèle (balayage d'univers sur `Set(D)`) : un ensemble aléatoire sur
//! un domaine de `n` clés, chaque clé présente avec probabilité `d`, a une
//! chaîne d'environ `n·d` nœuds (+ le terminal) ; chaque requête aval
//! coûte la taille prédite au coût d'encodage du royaume.

/// Plus grand commun diviseur (Euclide).
fn pgcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let reste = a % b;
        a = b;
        b = reste;
    }
    a.max(1)
}

/// Rationnel exact, normalisé (dénominateur strictement positif) :
/// arithmétique exacte par défaut (roadmap § 10, décision M5-4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rationnel {
    /// Numérateur.
    pub num: u64,
    /// Dénominateur (> 0).
    pub den: u64,
}

impl Rationnel {
    /// Rationnel `num/den`, normalisé par le pgcd ; `den == 0` ⇒ 0/1.
    pub fn nouveau(num: u64, den: u64) -> Self {
        if den == 0 {
            return Rationnel { num: 0, den: 1 };
        }
        let diviseur = pgcd(num, den);
        Rationnel {
            num: num / diviseur,
            den: den / diviseur,
        }
    }

    /// Rationnel entier.
    pub fn entier(valeur: u64) -> Self {
        Rationnel {
            num: valeur,
            den: 1,
        }
    }

    /// Produit de deux rationnels (saturé en cas de dépassement).
    pub fn multiplier(&self, autre: &Self) -> Self {
        let num = self.num.saturating_mul(autre.num);
        let den = self.den.saturating_mul(autre.den);
        Rationnel::nouveau(num, den)
    }

    /// `self <= autre` en arithmétique exacte (produits croisés, u128).
    pub fn inferieur_ou_egal(&self, autre: &Self) -> bool {
        (self.num as u128) * (autre.den as u128) <= (autre.num as u128) * (self.den as u128)
    }
}

/// Borne inférieure de conformité du garde-fou : mesure ≥ prédit / 2.
pub const BORNE_INFERIEURE: Rationnel = Rationnel { num: 1, den: 2 };
/// Borne supérieure de conformité du garde-fou : mesure ≤ 2 × prédit.
pub const BORNE_SUPERIEURE: Rationnel = Rationnel { num: 2, den: 1 };

/// Erreur typée de construction des paramètres de l'oracle.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurOracle {
    /// Densité attendue hors de [0, 1].
    DensiteHorsBornes,
}

impl std::fmt::Display for ErreurOracle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurOracle::DensiteHorsBornes => {
                write!(formatter, "densité attendue hors de [0, 1]")
            }
        }
    }
}

impl std::error::Error for ErreurOracle {}

/// Paramètres d'entrée du modèle prédictif (M3-3). Construire par
/// [`ParametresOracle::nouveau`] : la densité y est validée dans [0, 1].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParametresOracle {
    /// Taille du domaine : nombre de clés balayées.
    pub taille_domaine: u64,
    /// Densité attendue : fraction des clés présentes, dans [0, 1].
    pub densite_attendue: Rationnel,
    /// Coût d'encodage : coût d'un niveau selon l'encodage du royaume.
    pub cout_encodage: u64,
    /// Nombre de requêtes aval prévues sur le diagramme.
    pub requetes_aval: u64,
}

impl ParametresOracle {
    /// Paramètres validés : la densité attendue doit tenir dans [0, 1].
    pub fn nouveau(
        taille_domaine: u64,
        densite_attendue: Rationnel,
        cout_encodage: u64,
        requetes_aval: u64,
    ) -> Result<Self, ErreurOracle> {
        let zero = Rationnel::nouveau(0, 1);
        let un = Rationnel::entier(1);
        if !(zero.inferieur_ou_egal(&densite_attendue) && densite_attendue.inferieur_ou_egal(&un)) {
            return Err(ErreurOracle::DensiteHorsBornes);
        }
        Ok(ParametresOracle {
            taille_domaine,
            densite_attendue,
            cout_encodage,
            requetes_aval,
        })
    }
}

/// Prédiction du modèle prédictif.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Prediction {
    /// Taille de diagramme prédite (nœuds, terminal compris).
    pub taille_predite: u64,
    /// Coût total prédit (unités d'encodage) : taille × coût d'encodage ×
    /// requêtes aval.
    pub cout_total_predit: u64,
}

/// Oracle de rentabilité analytique (M3-3) : le modèle prédictif et son
/// garde-fou par différentiel.
pub struct Oracle;

impl Oracle {
    /// Prédiction : `taille = 1 + n·d` nœuds (chaîne du set aléatoire +
    /// terminal), `coût total = taille × coût_encodage × requêtes_aval`
    /// (saturé).
    pub fn predire(parametres: &ParametresOracle) -> Prediction {
        let presents = (parametres.taille_domaine as u128)
            * (parametres.densite_attendue.num as u128)
            / (parametres.densite_attendue.den as u128);
        let taille_predite = (presents.min(u64::MAX as u128) as u64).saturating_add(1);
        let cout_total_predit = taille_predite
            .saturating_mul(parametres.cout_encodage)
            .saturating_mul(parametres.requetes_aval);
        Prediction {
            taille_predite,
            cout_total_predit,
        }
    }

    /// Garde-fou par différentiel (C4) : compare la prédiction à la mesure
    /// réelle. Conforme si la mesure tient dans [prédit/2, 2 × prédit].
    pub fn garde_fou(predit: u64, mesure: u64) -> VerdictGardeFou {
        VerdictGardeFou::nouveau(predit, mesure)
    }
}

/// Verdict du garde-fou par différentiel : prédiction contre mesure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerdictGardeFou {
    /// Coût prédit par le modèle.
    pub predit: u64,
    /// Coût réellement mesuré.
    pub mesure: u64,
}

impl VerdictGardeFou {
    /// Verdict entre une prédiction et une mesure.
    pub fn nouveau(predit: u64, mesure: u64) -> Self {
        VerdictGardeFou { predit, mesure }
    }

    /// Ratio mesure/prédit en arithmétique exacte. Par convention de
    /// [`Rationnel::nouveau`], `prédit == 0` donne le ratio 0/1 (et non une
    /// valeur saturée) ; `est_conforme` ne passe pas par `ratio` et traite
    /// `prédit == 0` directement.
    pub fn ratio(&self) -> Rationnel {
        Rationnel::nouveau(self.mesure, self.predit)
    }

    /// Conforme si la mesure tient dans [BORNE_INFERIEURE × prédit,
    /// BORNE_SUPERIEURE × prédit] — comparaison exacte sans division
    /// (produits croisés en u128), donc correcte aussi pour `prédit == 0`
    /// (conforme ssi `mesure == 0`). Hors bornes, le modèle est falsifié (C4).
    pub fn est_conforme(&self) -> bool {
        let predit = self.predit as u128;
        let mesure = self.mesure as u128;
        // BORNE_INFERIEURE <= mesure/prédit <=> min.num·prédit <= mesure·min.den
        // mesure/prédit <= BORNE_SUPERIEURE <=> mesure·max.den <= max.num·prédit
        (BORNE_INFERIEURE.num as u128) * predit <= mesure * (BORNE_INFERIEURE.den as u128)
            && mesure * (BORNE_SUPERIEURE.den as u128) <= (BORNE_SUPERIEURE.num as u128) * predit
    }
}
