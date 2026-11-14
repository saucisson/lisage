//! Analytical profitability oracle (M3-3, claim C4): **first predictive
//! model** — domain size, expected density, encoding cost,
//! number of downstream queries — and **differential guardrail**: the
//! prediction is compared to the actual measurement, within a factor (C4 is
//! falsifiable: a model that deviates out of bounds is rejected).
//!
//! Model (universe sweep on `Set(D)`): a random set over a domain of `n`
//! keys, each key present with probability `d`, has a chain of about `n·d`
//! nodes (+ the terminal); each downstream query costs the predicted size
//! at the realm's encoding cost.

/// Greatest common divisor (Euclid).
fn pgcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let reste = a % b;
        a = b;
        b = reste;
    }
    a.max(1)
}

/// Exact rational, normalized (strictly positive denominator):
/// exact arithmetic by default (roadmap § 10, M5-4 decision). Private
/// fields: only [`Rationnel::nouveau`] constructs, the `den > 0` invariant
/// cannot be bypassed by a literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rationnel {
    num: u64,
    den: u64,
}

impl Rationnel {
    /// Rational `num/den`, normalized by the gcd; `den == 0` ⇒ 0/1.
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

    /// Integer rational.
    pub fn entier(valeur: u64) -> Self {
        Rationnel {
            num: valeur,
            den: 1,
        }
    }

    /// Numerator (after normalization).
    pub fn num(&self) -> u64 {
        self.num
    }

    /// Denominator (> 0).
    pub fn den(&self) -> u64 {
        self.den
    }

    /// Product of two rationals (saturated on overflow).
    pub fn multiplier(&self, autre: &Self) -> Self {
        let num = self.num.saturating_mul(autre.num);
        let den = self.den.saturating_mul(autre.den);
        Rationnel::nouveau(num, den)
    }

    /// `self <= autre` in exact arithmetic (cross products, u128).
    pub fn inferieur_ou_egal(&self, autre: &Self) -> bool {
        (self.num as u128) * (autre.den as u128) <= (autre.num as u128) * (self.den as u128)
    }
}

/// Lower conformance bound of the guardrail: measure ≥ predicted / 2.
pub const BORNE_INFERIEURE: Rationnel = Rationnel { num: 1, den: 2 };
/// Upper conformance bound of the guardrail: measure ≤ 2 × predicted.
pub const BORNE_SUPERIEURE: Rationnel = Rationnel { num: 2, den: 1 };

/// Typed error of oracle parameter construction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurOracle {
    /// Expected density outside [0, 1].
    DensiteHorsBornes,
}

impl std::fmt::Display for ErreurOracle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurOracle::DensiteHorsBornes => {
                write!(formatter, "expected density outside [0, 1]")
            }
        }
    }
}

impl std::error::Error for ErreurOracle {}

/// Input parameters of the predictive model (M3-3). Private fields: only
/// [`ParametresOracle::nouveau`] constructs, the density is validated in
/// [0, 1] there and cannot be bypassed by a literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParametresOracle {
    taille_domaine: u64,
    densite_attendue: Rationnel,
    cout_encodage: u64,
    requetes_aval: u64,
}

impl ParametresOracle {
    /// Validated parameters: the expected density must fit in [0, 1].
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

    /// Domain size: number of swept keys.
    pub fn taille_domaine(&self) -> u64 {
        self.taille_domaine
    }

    /// Expected density (validated in [0, 1]).
    pub fn densite_attendue(&self) -> Rationnel {
        self.densite_attendue
    }

    /// Encoding cost: cost of one level according to the realm's encoding.
    pub fn cout_encodage(&self) -> u64 {
        self.cout_encodage
    }

    /// Number of downstream queries planned on the diagram.
    pub fn requetes_aval(&self) -> u64 {
        self.requetes_aval
    }
}

/// Prediction of the predictive model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Prediction {
    /// Predicted diagram size (nodes, terminal included).
    pub taille_predite: u64,
    /// Predicted total cost (encoding units): size × encoding cost ×
    /// downstream queries.
    pub cout_total_predit: u64,
}

/// Analytical profitability oracle (M3-3): the predictive model and its
/// differential guardrail.
pub struct Oracle;

impl Oracle {
    /// Prediction: `size = 1 + n·d` nodes (chain of the random set +
    /// terminal), `total cost = size × encoding_cost × downstream_queries`
    /// (saturated).
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

    /// Differential guardrail (C4): compares the prediction to the actual
    /// measurement. Conforming if the measurement fits in [predicted/2, 2 × predicted].
    pub fn garde_fou(predit: u64, mesure: u64) -> VerdictGardeFou {
        VerdictGardeFou::nouveau(predit, mesure)
    }
}

/// Verdict of the differential guardrail: prediction against measurement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerdictGardeFou {
    /// Cost predicted by the model.
    pub predit: u64,
    /// Cost actually measured.
    pub mesure: u64,
}

impl VerdictGardeFou {
    /// Verdict between a prediction and a measurement.
    pub fn nouveau(predit: u64, mesure: u64) -> Self {
        VerdictGardeFou { predit, mesure }
    }

    /// Ratio measure/predicted in exact arithmetic. By convention of
    /// [`Rationnel::nouveau`], `predicted == 0` gives ratio 0/1 (not a
    /// saturated value); `est_conforme` does not go through `ratio` and
    /// handles `predicted == 0` directly.
    pub fn ratio(&self) -> Rationnel {
        Rationnel::nouveau(self.mesure, self.predit)
    }

    /// Conforming if the measurement fits in [BORNE_INFERIEURE × predicted,
    /// BORNE_SUPERIEURE × predicted] — exact comparison without division
    /// (cross products in u128), hence correct also for `predicted == 0`
    /// (conforming iff `measurement == 0`). Out of bounds, the model is
    /// falsified (C4).
    pub fn est_conforme(&self) -> bool {
        let predit = self.predit as u128;
        let mesure = self.mesure as u128;
        // BORNE_INFERIEURE <= measure/predicted <=> min.num·predicted <= measure·min.den
        // measure/predicted <= BORNE_SUPERIEURE <=> measure·max.den <= max.num·predicted
        (BORNE_INFERIEURE.num as u128) * predit <= mesure * (BORNE_INFERIEURE.den as u128)
            && mesure * (BORNE_SUPERIEURE.den as u128) <= (BORNE_SUPERIEURE.num as u128) * predit
    }
}
