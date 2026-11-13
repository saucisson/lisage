//! Analytic profitability oracle (M3-3, statement C4): **first predictive
//! model** — domain size, expected density, encoding cost, number of
//! downstream requests — and **differential guardrail**: the prediction is
//! compared to the actual measurement, up to a factor (C4 is falsifiable: a
//! model that deviates out of bounds is rejected).
//!
//! Model (universe sweep over `Set(D)`): a random set over a domain of `n`
//! keys, each key present with probability `d`, has a chain of about `n·d`
//! nodes (+ the terminal); each downstream request costs the predicted size
//! at the kingdom's encoding cost.

/// Greatest common divisor (Euclid).
fn pgcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

/// Exact rational, normalized (strictly positive denominator): exact
/// arithmetic by default (roadmap § 10, decision M5-4). Private fields:
/// only [`Rational::new`] constructs it, the `den > 0` invariant cannot
/// be bypassed by a literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rational {
    num: u64,
    den: u64,
}

impl Rational {
    /// Rational `num/den`, normalized by the gcd; `den == 0` ⇒ 0/1.
    pub fn new(num: u64, den: u64) -> Self {
        if den == 0 {
            return Rational { num: 0, den: 1 };
        }
        let divisor = pgcd(num, den);
        Rational {
            num: num / divisor,
            den: den / divisor,
        }
    }

    /// Integer rational.
    pub fn entier(value: u64) -> Self {
        Rational { num: value, den: 1 }
    }

    /// Numerator (after normalization).
    pub fn num(&self) -> u64 {
        self.num
    }

    /// Denominator (> 0).
    pub fn den(&self) -> u64 {
        self.den
    }

    /// Product of two rationals (saturating on overflow).
    pub fn multiplier(&self, other: &Self) -> Self {
        let num = self.num.saturating_mul(other.num);
        let den = self.den.saturating_mul(other.den);
        Rational::new(num, den)
    }

    /// `self <= other` in exact arithmetic (cross products, u128).
    pub fn lower_or_equal(&self, other: &Self) -> bool {
        (self.num as u128) * (other.den as u128) <= (other.num as u128) * (self.den as u128)
    }
}

/// Lower bound of the guardrail's compliance: measurement ≥ predicted / 2.
pub const BOUND_INFERIEURE: Rational = Rational { num: 1, den: 2 };
/// Upper bound of the guardrail's compliance: measurement ≤ 2 × predicted.
pub const BOUND_SUPERIEURE: Rational = Rational { num: 2, den: 1 };

/// Typed error for oracle parameter construction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorOracle {
    /// Expected density out of [0, 1].
    DensityOutBounds,
}

impl std::fmt::Display for ErrorOracle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorOracle::DensityOutBounds => {
                write!(formatter, "expected density out of [0, 1]")
            }
        }
    }
}

impl std::error::Error for ErrorOracle {}

/// Input parameters of the predictive model (M3-3). Private fields: only
/// [`ParametersOracle::new`] constructs it, the density is validated in
/// [0, 1] there and cannot be bypassed by a literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParametersOracle {
    size_domain: u64,
    density_expected: Rational,
    cost_encoding: u64,
    requests_downstream: u64,
}

impl ParametersOracle {
    /// Validated parameters: the expected density must lie in [0, 1].
    pub fn new(
        size_domain: u64,
        density_expected: Rational,
        cost_encoding: u64,
        requests_downstream: u64,
    ) -> Result<Self, ErrorOracle> {
        let zero = Rational::new(0, 1);
        let one = Rational::entier(1);
        if !(zero.lower_or_equal(&density_expected) && density_expected.lower_or_equal(&one)) {
            return Err(ErrorOracle::DensityOutBounds);
        }
        Ok(ParametersOracle {
            size_domain,
            density_expected,
            cost_encoding,
            requests_downstream,
        })
    }

    /// Domain size: number of keys swept.
    pub fn size_domain(&self) -> u64 {
        self.size_domain
    }

    /// Expected density (validated in [0, 1]).
    pub fn density_expected(&self) -> Rational {
        self.density_expected
    }

    /// Encoding cost of one level under the kingdom's encoding.
    pub fn cost_encoding(&self) -> u64 {
        self.cost_encoding
    }

    /// Number of downstream requests expected on the diagram.
    pub fn requests_downstream(&self) -> u64 {
        self.requests_downstream
    }
}

/// Prediction of the predictive model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Prediction {
    /// Predicted diagram size (nodes, terminal included).
    pub size_predicted: u64,
    /// Predicted total cost (encoding units): size × encoding cost ×
    /// downstream requests.
    pub cost_total_predicted: u64,
}

/// Analytic profitability oracle (M3-3): the predictive model and its
/// differential guardrail.
pub struct Oracle;

impl Oracle {
    /// Prediction: `size = 1 + n·d` nodes (chain of the random set +
    /// terminal), `total cost = size × cost_encoding × requests_downstream`
    /// (saturating).
    pub fn predire(parameters: &ParametersOracle) -> Prediction {
        let presents = (parameters.size_domain as u128) * (parameters.density_expected.num as u128)
            / (parameters.density_expected.den as u128);
        let size_predicted = (presents.min(u64::MAX as u128) as u64).saturating_add(1);
        let cost_total_predicted = size_predicted
            .saturating_mul(parameters.cost_encoding)
            .saturating_mul(parameters.requests_downstream);
        Prediction {
            size_predicted,
            cost_total_predicted,
        }
    }

    /// Differential guardrail (C4): compares the prediction to the actual
    /// measurement. Compliant if the measurement lies in [predicted/2, 2 × predicted].
    pub fn keeps_wild(predicted: u64, measure: u64) -> VerdictKeepsWild {
        VerdictKeepsWild::new(predicted, measure)
    }
}

/// Verdict of the differential guardrail: prediction against measurement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerdictKeepsWild {
    /// Cost predicted by the model.
    pub predicted: u64,
    /// Cost actually measured.
    pub measure: u64,
}

impl VerdictKeepsWild {
    /// Verdict between a prediction and a measurement.
    pub fn new(predicted: u64, measure: u64) -> Self {
        VerdictKeepsWild { predicted, measure }
    }

    /// Ratio measurement/prediction in exact arithmetic. By convention of
    /// [`Rational::new`], `predicted == 0` gives the ratio 0/1 (and not a
    /// saturated value); `is_compliant` does not go through `ratio` and
    /// handles `predicted == 0` directly.
    pub fn ratio(&self) -> Rational {
        Rational::new(self.measure, self.predicted)
    }

    /// Compliant if the measurement lies in [BOUND_INFERIEURE × predicted,
    /// BOUND_SUPERIEURE × predicted] — exact comparison without division
    /// (cross products in u128), hence also correct for `predicted == 0`
    /// (compliant iff `measure == 0`). Out of bounds, the model is falsified (C4).
    pub fn is_compliant(&self) -> bool {
        let predicted = self.predicted as u128;
        let measure = self.measure as u128;
        // BOUND_INFERIEURE <= measure/predicted <=> min.num·predicted <= measure·min.den
        // measure/predicted <= BOUND_SUPERIEURE <=> measure·max.den <= max.num·predicted
        (BOUND_INFERIEURE.num as u128) * predicted <= measure * (BOUND_INFERIEURE.den as u128)
            && measure * (BOUND_SUPERIEURE.den as u128)
                <= (BOUND_SUPERIEURE.num as u128) * predicted
    }
}
