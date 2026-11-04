//! The **shape** of a distribution.
//!
//! A formula may ask how a set of numbers is spread out:
//!
//! - `GAMMA(number)` and `GAMMALN(number)` — the **gamma function** and
//!   its logarithm;
//! - `ERF(x)`, `ERF.PRECISE(x)`, `ERFC(x)` and `ERFC.PRECISE(x)` — the
//!   **error function** and its complement;
//! - `NORM.DIST(x, mean, deviation, cumulative)` — the **normal**
//!   distribution;
//! - `NORM.INV(probability, mean, deviation)` — the **inverse** of it;
//! - `NORM.S.DIST(z, cumulative)`, `NORM.S.INV(probability)` — the same
//!   for the **standard** normal;
//! - `GAUSS(z)` — the chance that a standard normal value falls
//!   between the mean and `z`;
//! - `PHI(z)` — the **density** of the standard normal.
//!
//! The rules are the **tree's**: a **deviation** of nothing is a
//! number error, a probability outside the open interval of zero and
//! one is a number error, and a gamma of a number that is not a number
//! is a number error.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **distribution functions**.
pub fn is_distribution_function(name: &str) -> bool {
    matches!(
        name,
        "GAMMA"
            | "GAMMALN"
            | "GAMMALN.PRECISE"
            | "ERF"
            | "ERF.PRECISE"
            | "ERFC"
            | "ERFC.PRECISE"
            | "NORM.DIST"
            | "NORMDIST"
            | "NORM.INV"
            | "NORMINV"
            | "NORM.S.DIST"
            | "NORMSDIST"
            | "NORM.S.INV"
            | "NORMSINV"
            | "GAUSS"
            | "PHI"
            | "GAMMA.DIST"
            | "GAMMADIST"
            | "GAMMA.INV"
            | "GAMMAINV"
            | "T.DIST"
            | "T.DIST.2T"
            | "T.DIST.RT"
            | "TDIST"
            | "T.INV"
            | "T.INV.2T"
            | "TINV"
            | "CHISQ.DIST"
            | "CHISQ.DIST.RT"
            | "CHIDIST"
            | "CHISQ.INV"
            | "CHISQ.INV.RT"
            | "F.DIST"
            | "F.DIST.RT"
            | "F.INV"
            | "FINV"
            | "BINOM.DIST"
            | "BINOMDIST"
            | "BINOM.INV"
            | "POISSON"
            | "POISSON.DIST"
            | "EXPON"
            | "EXPONDIST"
            | "EXPON.DIST"
            | "BETA.DIST"
            | "BETADIST"
            | "BETA.INV"
            | "LOGNORM.DIST"
            | "LOGNORMDIST"
            | "LOGNORM.INV"
            | "LOGINV"
            | "WEIBULL"
            | "WEIBULL.DIST"
            | "HYPGEOM.DIST"
            | "HYPGEOMDIST"
    )
}

/// A value read as a **truth**.
fn as_bool(value: &Value) -> Result<bool, String> {
    match value {
        Value::Empty => Ok(false),
        Value::Number(number) => Ok(*number != 0.0),
        Value::Bool(value) => Ok(*value),
        Value::Text(text) => {
            if text.eq_ignore_ascii_case("TRUE") {
                Ok(true)
            } else if text.eq_ignore_ascii_case("FALSE") {
                Ok(false)
            } else {
                Err("#VALUE!".to_string())
            }
        }
        Value::Error(message) => Err(message.clone()),
    }
}

/// The **degrees of freedom** an argument says.
fn degrees_of(number: f64) -> Result<f64, String> {
    let number = number.trunc();
    if number < 1.0 {
        return Err("#NUM!".to_string());
    }
    Ok(number)
}

/// The **tail** of Student's t above a number.
fn t_upper(t: f64, degrees: f64) -> f64 {
    0.5 * crate::special::reg_inc_beta(degrees / (degrees + t * t), degrees / 2.0, 0.5)
}

/// The **cumulative** of Student's t.
fn t_cdf(t: f64, degrees: f64) -> f64 {
    if t >= 0.0 { 1.0 - t_upper(t, degrees) } else { t_upper(-t, degrees) }
}

/// The **density** of Student's t.
fn t_pdf(t: f64, degrees: f64) -> f64 {
    (crate::special::ln_gamma((degrees + 1.0) / 2.0)
        - crate::special::ln_gamma(degrees / 2.0)
        - 0.5 * (degrees * std::f64::consts::PI).ln()
        - (degrees + 1.0) / 2.0 * (1.0 + t * t / degrees).ln())
    .exp()
}

/// The **tail** of Student's t, inverted: a number of zero or more at
/// which the tail is the probability.
fn t_inv_upper(probability: f64, degrees: f64) -> f64 {
    if probability >= 0.5 {
        return 0.0;
    }
    crate::special::invert(|t| t_upper(t, degrees), probability, 0.0, 1.0, 1e60)
}

/// The **inverse** of the chi-square cumulative.
fn chi_inv(probability: f64, degrees: f64) -> f64 {
    if probability <= 0.0 {
        return 0.0;
    }
    crate::special::invert(
        |x| crate::special::reg_gamma_p(degrees / 2.0, x / 2.0),
        probability,
        0.0,
        degrees.max(1.0),
        1e60,
    )
}

/// The **inverse** of the right tail of the chi-square.
fn chi_inv_rt(probability: f64, degrees: f64) -> f64 {
    if probability >= 1.0 {
        return 0.0;
    }
    crate::special::invert(
        |x| crate::special::reg_gamma_q(degrees / 2.0, x / 2.0),
        probability,
        0.0,
        degrees.max(1.0),
        1e60,
    )
}

/// The **cumulative** of the F distribution.
fn f_cdf(x: f64, first: f64, second: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    crate::special::reg_inc_beta(first * x / (first * x + second), first / 2.0, second / 2.0)
}

/// The **tail** of the F distribution.
fn f_upper(x: f64, first: f64, second: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    crate::special::reg_inc_beta(second / (second + first * x), second / 2.0, first / 2.0)
}

/// The **density** of the F distribution.
fn f_pdf(x: f64, first: f64, second: f64) -> f64 {
    if x == 0.0 {
        return if first < 2.0 {
            f64::INFINITY
        } else if first == 2.0 {
            1.0
        } else {
            0.0
        };
    }
    ((first / 2.0) * (first / second).ln() + (first / 2.0 - 1.0) * x.ln()
        - ((first + second) / 2.0) * (1.0 + first * x / second).ln()
        - crate::special::ln_beta(first / 2.0, second / 2.0))
    .exp()
}

/// The logarithm of the number of **combinations**.
fn ln_combination(n: f64, k: f64) -> f64 {
    crate::special::ln_gamma(n + 1.0)
        - crate::special::ln_gamma(k + 1.0)
        - crate::special::ln_gamma(n - k + 1.0)
}

/// The **chance** of a number of successes in a number of trials.
fn binom_pmf(k: f64, n: f64, p: f64) -> f64 {
    if k < 0.0 || k > n {
        return 0.0;
    }
    if p == 0.0 {
        return if k == 0.0 { 1.0 } else { 0.0 };
    }
    if p == 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }
    (ln_combination(n, k) + k * p.ln() + (n - k) * (1.0 - p).ln()).exp()
}

/// The **cumulative** chance of a number of successes.
fn binom_cdf(k: f64, n: f64, p: f64) -> f64 {
    if k < 0.0 {
        return 0.0;
    }
    if k >= n {
        return 1.0;
    }
    if n <= 10_000.0 {
        let mut sum = 0.0;
        let mut index = 0.0;
        while index <= k {
            sum += binom_pmf(index, n, p);
            index += 1.0;
        }
        return sum.min(1.0);
    }
    if p == 0.0 {
        return 1.0;
    }
    if p == 1.0 {
        return 0.0;
    }
    crate::special::reg_inc_beta(1.0 - p, n - k, k + 1.0)
}

/// The **chance** of a number of things in a set.
fn poisson_pmf(x: f64, mean: f64) -> f64 {
    if mean == 0.0 {
        return if x == 0.0 { 1.0 } else { 0.0 };
    }
    (-mean + x * mean.ln() - crate::special::ln_gamma(x + 1.0)).exp()
}

/// The **cumulative** chance of a number of things.
fn poisson_cdf(x: f64, mean: f64) -> f64 {
    if mean == 0.0 {
        return 1.0;
    }
    if x <= 1000.0 {
        let mut sum = 0.0;
        let mut index = 0.0;
        while index <= x {
            sum += poisson_pmf(index, mean);
            index += 1.0;
        }
        return sum.min(1.0);
    }
    crate::special::reg_gamma_q(x + 1.0, mean)
}

impl Book {
    /// The **distribution functions**.
    pub(crate) fn distributions(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "ERF" | "ERF.PRECISE" | "ERFC" | "ERFC.PRECISE" => {
                let low = match self.number_at(sheet, arguments, 0, being) {
                    Ok(low) => low,
                    Err(message) => return Value::Error(message),
                };
                if name == "ERFC" || name == "ERFC.PRECISE" {
                    let value = crate::special::erfc(low);
                    return if value.is_finite() {
                        Value::Number(value)
                    } else {
                        Value::Error("#NUM!".to_string())
                    };
                }
                if name == "ERF" && arguments.len() > 1 {
                    // The error function between two numbers.
                    let high = match self.number_at(sheet, arguments, 1, being) {
                        Ok(high) => high,
                        Err(message) => return Value::Error(message),
                    };
                    let value = crate::special::erf(high) - crate::special::erf(low);
                    return if value.is_finite() {
                        Value::Number(value)
                    } else {
                        Value::Error("#NUM!".to_string())
                    };
                }
                let value = crate::special::erf(low);
                if value.is_finite() { Value::Number(value) } else { Value::Error("#NUM!".to_string()) }
            }
            "GAMMA" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let value = crate::special::gamma(number);
                if value.is_finite() { Value::Number(value) } else { Value::Error("#NUM!".to_string()) }
            }
            "GAMMALN" | "GAMMALN.PRECISE" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                if number <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(crate::special::ln_gamma(number))
            }
            "PHI" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                Value::Number(crate::special::norm_pdf(number))
            }
            "GAUSS" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                Value::Number(0.5 * crate::special::erf(number / std::f64::consts::SQRT_2))
            }
            "NORM.S.DIST" | "NORMSDIST" => {
                let z = match self.number_at(sheet, arguments, 0, being) {
                    Ok(z) => z,
                    Err(message) => return Value::Error(message),
                };
                let cumulative = if arguments.len() > 1 {
                    let value = self.evaluate(sheet, &arguments[1], being);
                    match as_bool(&value) {
                        Ok(cumulative) => cumulative,
                        Err(message) => return Value::Error(message),
                    }
                } else {
                    true
                };
                Value::Number(if cumulative {
                    crate::special::norm_cdf(z)
                } else {
                    crate::special::norm_pdf(z)
                })
            }
            "NORM.S.INV" | "NORMSINV" => {
                let probability = match self.number_at(sheet, arguments, 0, being) {
                    Ok(probability) => probability,
                    Err(message) => return Value::Error(message),
                };
                if !(probability > 0.0 && probability < 1.0) {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(crate::special::norm_inv(probability))
            }
            "NORM.DIST" | "NORMDIST" => {
                let (x, mean, deviation) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(x), Ok(mean), Ok(deviation)) => (x, mean, deviation),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let Some(fourth) = arguments.get(3) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, fourth, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                if deviation <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let z = (x - mean) / deviation;
                Value::Number(if cumulative {
                    crate::special::norm_cdf(z)
                } else {
                    crate::special::norm_pdf(z) / deviation
                })
            }
            "GAMMA.DIST" | "GAMMADIST" => {
                let (x, alpha, beta) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(x), Ok(alpha), Ok(beta)) => (x, alpha, beta),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let Some(fourth) = arguments.get(3) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, fourth, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                if x < 0.0 || alpha <= 0.0 || beta <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                if cumulative {
                    return Value::Number(crate::special::reg_gamma_p(alpha, x / beta));
                }
                if x == 0.0 {
                    return if alpha < 1.0 {
                        Value::Error("#NUM!".to_string())
                    } else if alpha == 1.0 {
                        Value::Number(1.0 / beta)
                    } else {
                        Value::Number(0.0)
                    };
                }
                Value::Number(
                    ((alpha - 1.0) * x.ln() - x / beta - alpha * beta.ln() - crate::special::ln_gamma(alpha))
                        .exp(),
                )
            }
            "GAMMA.INV" | "GAMMAINV" => {
                let (probability, alpha, beta) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(probability), Ok(alpha), Ok(beta)) => (probability, alpha, beta),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if !(0.0..1.0).contains(&probability) || alpha <= 0.0 || beta <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                if probability == 0.0 {
                    return Value::Number(0.0);
                }
                Value::Number(crate::special::invert(
                    |x| crate::special::reg_gamma_p(alpha, x / beta),
                    probability,
                    0.0,
                    (alpha * beta).max(1e-3),
                    1e60,
                ))
            }
            "T.DIST" | "T.DIST.2T" | "T.DIST.RT" | "TDIST" | "T.INV" | "T.INV.2T" | "TINV" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let degrees = match self.number_at(sheet, arguments, 1, being) {
                    Ok(degrees) => match degrees_of(degrees) {
                        Ok(degrees) => degrees,
                        Err(message) => return Value::Error(message),
                    },
                    Err(message) => return Value::Error(message),
                };
                match name {
                    "T.DIST" => {
                        let Some(third) = arguments.get(2) else {
                            return Value::Error("#VALUE!".to_string());
                        };
                        let value = self.evaluate(sheet, third, being);
                        let cumulative = match as_bool(&value) {
                            Ok(cumulative) => cumulative,
                            Err(message) => return Value::Error(message),
                        };
                        Value::Number(if cumulative { t_cdf(first, degrees) } else { t_pdf(first, degrees) })
                    }
                    "T.DIST.2T" => {
                        if first < 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(2.0 * t_upper(first, degrees))
                    }
                    "T.DIST.RT" => Value::Number(1.0 - t_cdf(first, degrees)),
                    "TDIST" => {
                        let tails = match self.number_at(sheet, arguments, 2, being) {
                            Ok(tails) => tails.trunc(),
                            Err(message) => return Value::Error(message),
                        };
                        if first < 0.0 || !(tails == 1.0 || tails == 2.0) {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(tails * t_upper(first, degrees))
                    }
                    "T.INV" => {
                        if !(first > 0.0 && first < 1.0) {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(if first < 0.5 {
                            -t_inv_upper(first, degrees)
                        } else {
                            t_inv_upper(1.0 - first, degrees)
                        })
                    }
                    _ => {
                        // `T.INV.2T` and `TINV`.
                        if !(first > 0.0 && first <= 1.0) {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(t_inv_upper(first / 2.0, degrees))
                    }
                }
            }
            "CHISQ.DIST" | "CHISQ.DIST.RT" | "CHIDIST" | "CHISQ.INV" | "CHISQ.INV.RT" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let degrees = match self.number_at(sheet, arguments, 1, being) {
                    Ok(degrees) => match degrees_of(degrees) {
                        Ok(degrees) => degrees,
                        Err(message) => return Value::Error(message),
                    },
                    Err(message) => return Value::Error(message),
                };
                match name {
                    "CHISQ.DIST" => {
                        let Some(third) = arguments.get(2) else {
                            return Value::Error("#VALUE!".to_string());
                        };
                        let value = self.evaluate(sheet, third, being);
                        let cumulative = match as_bool(&value) {
                            Ok(cumulative) => cumulative,
                            Err(message) => return Value::Error(message),
                        };
                        if first < 0.0 || degrees > 1e10 {
                            return Value::Error("#NUM!".to_string());
                        }
                        if cumulative {
                            return Value::Number(crate::special::reg_gamma_p(degrees / 2.0, first / 2.0));
                        }
                        if first == 0.0 {
                            return if degrees == 2.0 {
                                Value::Number(0.5)
                            } else if degrees < 2.0 {
                                Value::Error("#NUM!".to_string())
                            } else {
                                Value::Number(0.0)
                            };
                        }
                        Value::Number(
                            ((degrees / 2.0 - 1.0) * first.ln()
                                - first / 2.0
                                - (degrees / 2.0) * 2f64.ln()
                                - crate::special::ln_gamma(degrees / 2.0))
                            .exp(),
                        )
                    }
                    "CHISQ.DIST.RT" | "CHIDIST" => {
                        if first < 0.0 || degrees > 1e10 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(crate::special::reg_gamma_q(degrees / 2.0, first / 2.0))
                    }
                    "CHISQ.INV" => {
                        if !(0.0..1.0).contains(&first) || degrees > 1e10 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(chi_inv(first, degrees))
                    }
                    _ => {
                        // `CHISQ.INV.RT`.
                        if !(first > 0.0 && first <= 1.0) || degrees > 1e10 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(chi_inv_rt(first, degrees))
                    }
                }
            }
            "F.DIST" | "F.DIST.RT" | "F.INV" | "FINV" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let (one, two) = match (
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(one), Ok(two)) => (one.trunc(), two.trunc()),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                if one < 1.0 || two < 1.0 || one >= 1e10 || two >= 1e10 {
                    return Value::Error("#NUM!".to_string());
                }
                match name {
                    "F.DIST" => {
                        let Some(fourth) = arguments.get(3) else {
                            return Value::Error("#VALUE!".to_string());
                        };
                        let value = self.evaluate(sheet, fourth, being);
                        let cumulative = match as_bool(&value) {
                            Ok(cumulative) => cumulative,
                            Err(message) => return Value::Error(message),
                        };
                        if first < 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(if cumulative {
                            f_cdf(first, one, two)
                        } else {
                            f_pdf(first, one, two)
                        })
                    }
                    "F.DIST.RT" => {
                        if first < 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(f_upper(first, one, two))
                    }
                    "F.INV" => {
                        if !(0.0..1.0).contains(&first) {
                            return Value::Error("#NUM!".to_string());
                        }
                        if first == 0.0 {
                            return Value::Number(0.0);
                        }
                        Value::Number(crate::special::invert(|x| f_cdf(x, one, two), first, 0.0, 1.0, 1e60))
                    }
                    _ => {
                        // `FINV`.
                        if !(first > 0.0 && first <= 1.0) {
                            return Value::Error("#NUM!".to_string());
                        }
                        if first == 1.0 {
                            return Value::Number(0.0);
                        }
                        Value::Number(crate::special::invert(|x| f_upper(x, one, two), first, 0.0, 1.0, 1e60))
                    }
                }
            }
            "BINOM.DIST" | "BINOMDIST" | "BINOM.INV" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let second = match self.number_at(sheet, arguments, 1, being) {
                    Ok(second) => second,
                    Err(message) => return Value::Error(message),
                };
                let third = match self.number_at(sheet, arguments, 2, being) {
                    Ok(third) => third,
                    Err(message) => return Value::Error(message),
                };
                if name == "BINOM.INV" {
                    let (n, p, alpha) = (first.trunc(), second, third);
                    if n < 0.0 || !(0.0..=1.0).contains(&p) || !(0.0..=1.0).contains(&alpha) {
                        return Value::Error("#NUM!".to_string());
                    }
                    let (mut low, mut high) = (0.0f64, n);
                    if binom_cdf(0.0, n, p) >= alpha {
                        return Value::Number(0.0);
                    }
                    while high - low > 1.0 {
                        let middle = ((low + high) / 2.0).floor();
                        if middle <= low || middle >= high {
                            break;
                        }
                        if binom_cdf(middle, n, p) >= alpha {
                            high = middle;
                        } else {
                            low = middle;
                        }
                    }
                    return Value::Number(high);
                }
                let (k, n, p) = (first.trunc(), second.trunc(), third);
                let Some(fourth) = arguments.get(3) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, fourth, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                if k < 0.0 || k > n || !(0.0..=1.0).contains(&p) {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(if cumulative { binom_cdf(k, n, p) } else { binom_pmf(k, n, p) })
            }
            "POISSON" | "POISSON.DIST" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let mean = match self.number_at(sheet, arguments, 1, being) {
                    Ok(mean) => mean,
                    Err(message) => return Value::Error(message),
                };
                let Some(third) = arguments.get(2) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, third, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                let x = first.trunc();
                if x < 0.0 || mean < 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(if cumulative { poisson_cdf(x, mean) } else { poisson_pmf(x, mean) })
            }
            "EXPON" | "EXPONDIST" | "EXPON.DIST" => {
                let (x, lambda) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                ) {
                    (Ok(x), Ok(lambda)) => (x, lambda),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let Some(third) = arguments.get(2) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, third, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                if x < 0.0 || lambda <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(if cumulative { -(-lambda * x).exp_m1() } else { lambda * (-lambda * x).exp() })
            }
            "BETA.DIST" | "BETADIST" | "BETA.INV" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let (alpha, beta) = match (
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(alpha), Ok(beta)) => (alpha, beta),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let bounds_at = if name == "BETA.DIST" { 4 } else { 3 };
                let low = match self.optional_number(sheet, arguments, bounds_at, being) {
                    Ok(Some(low)) => low,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let high = match self.optional_number(sheet, arguments, bounds_at + 1, being) {
                    Ok(Some(high)) => high,
                    Ok(None) => 1.0,
                    Err(message) => return Value::Error(message),
                };
                if low >= high {
                    return Value::Error("#NUM!".to_string());
                }
                if name == "BETA.INV" {
                    if alpha <= 0.0 || beta <= 0.0 || !(0.0..=1.0).contains(&first) {
                        return Value::Error("#NUM!".to_string());
                    }
                    let z = if first == 0.0 {
                        0.0
                    } else if first == 1.0 {
                        1.0
                    } else {
                        crate::special::invert(
                            |x| crate::special::reg_inc_beta(x, alpha, beta),
                            first,
                            0.0,
                            1.0,
                            1.0,
                        )
                    };
                    return Value::Number(low + z * (high - low));
                }
                if alpha <= 0.0 || beta <= 0.0 || first < low || first > high {
                    return Value::Error("#NUM!".to_string());
                }
                let z = (first - low) / (high - low);
                if name == "BETADIST" {
                    return Value::Number(crate::special::reg_inc_beta(z, alpha, beta));
                }
                let Some(fourth) = arguments.get(3) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, fourth, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                if cumulative {
                    return Value::Number(crate::special::reg_inc_beta(z, alpha, beta));
                }
                if (z == 0.0 && alpha < 1.0) || (z == 1.0 && beta < 1.0) {
                    return Value::Error("#NUM!".to_string());
                }
                let log = if z == 0.0 || z == 1.0 {
                    let base = if z == 0.0 { alpha } else { beta };
                    if base == 1.0 { -crate::special::ln_beta(alpha, beta) } else { f64::NEG_INFINITY }
                } else {
                    (alpha - 1.0) * z.ln() + (beta - 1.0) * (1.0 - z).ln()
                        - crate::special::ln_beta(alpha, beta)
                };
                Value::Number(log.exp() / (high - low))
            }
            "LOGNORM.DIST" | "LOGNORMDIST" | "LOGNORM.INV" | "LOGINV" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let (mean, deviation) = match (
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(mean), Ok(deviation)) => (mean, deviation),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                if name == "LOGNORM.INV" || name == "LOGINV" {
                    if !(first > 0.0 && first < 1.0) || deviation <= 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    return Value::Number((mean + deviation * crate::special::norm_inv(first)).exp());
                }
                let cumulative = if arguments.len() > 3 {
                    let Some(fourth) = arguments.get(3) else {
                        return Value::Error("#VALUE!".to_string());
                    };
                    let value = self.evaluate(sheet, fourth, being);
                    match as_bool(&value) {
                        Ok(cumulative) => cumulative,
                        Err(message) => return Value::Error(message),
                    }
                } else {
                    true
                };
                if first <= 0.0 || deviation <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let z = (first.ln() - mean) / deviation;
                Value::Number(if cumulative {
                    crate::special::norm_cdf(z)
                } else {
                    crate::special::norm_pdf(z) / (first * deviation)
                })
            }
            "WEIBULL" | "WEIBULL.DIST" => {
                let (x, alpha, beta) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(x), Ok(alpha), Ok(beta)) => (x, alpha, beta),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let Some(fourth) = arguments.get(3) else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, fourth, being);
                let cumulative = match as_bool(&value) {
                    Ok(cumulative) => cumulative,
                    Err(message) => return Value::Error(message),
                };
                if x < 0.0 || alpha <= 0.0 || beta <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let t = (x / beta).powf(alpha);
                Value::Number(if cumulative {
                    -(-t).exp_m1()
                } else {
                    alpha / beta.powf(alpha) * x.powf(alpha - 1.0) * (-t).exp()
                })
            }
            _ => {
                // `NORM.INV` and `NORMINV`.
                let (probability, mean, deviation) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(probability), Ok(mean), Ok(deviation)) => (probability, mean, deviation),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if !(probability > 0.0 && probability < 1.0) || deviation <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(mean + deviation * crate::special::norm_inv(probability))
            }
        }
    }
}
