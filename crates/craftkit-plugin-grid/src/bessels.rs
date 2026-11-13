//! The **Bessel** functions.
//!
//! Four functions of the tree's language that answer about the
//! solutions of Bessel's equation:
//!
//! - `BESSELJ(x, n)` — the function of the **first kind**;
//! - `BESSELY(x, n)` — the function of the **second kind**;
//! - `BESSELI(x, n)` — the **modified** function of the first kind;
//! - `BESSELK(x, n)` — the modified function of the second kind.
//!
//! The rules are the **tree's**: the order is a **whole number** that
//! is not negative and not above a hundred thousand, and a function
//! that has no value — the second kind at a number that is not
//! positive — is `#NUM!`.
//!
//! # How the numbers are worked out
//!
//! The tree's own ways, one for each family:
//!
//! - the **first kind** is the **integral** of the tree, summed with
//!   the trapezoid rule, which closes in fast because the integrand
//!   goes round and round;
//! - the **second kind** is worked out from the first kind by the
//!   **Neumann series** and then carried up by the recurrence;
//! - the **modified first kind** is its own **power series**, whose
//!   terms are all of one sign;
//! - the **modified second kind** is an **integral** too, and then the
//!   recurrence carries it up.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **Bessel functions**.
pub fn is_bessel_function(name: &str) -> bool {
    matches!(name, "BESSELJ" | "BESSELY" | "BESSELI" | "BESSELK")
}

/// The most points the **integral** of the first kind may use.
const MAX_POINTS: f64 = 4_000_000.0;

/// The Euler–Mascheroni constant.
const EULER: f64 = 0.577_215_664_901_532_9;

/// The **first kind**, `J_n(x)`.
///
/// The tree's rule: the integral is summed with the trapezoid rule,
/// and the number of points grows with the number and with the size
/// of the argument — because the integrand goes round and round, and
/// the rule closes in fast.
fn j_of(n: u32, x: f64) -> Option<f64> {
    let points = (2.0 * (x.abs() + f64::from(n)) + 64.0).ceil();
    if points > MAX_POINTS {
        // A number that is **too large** for the sum is left to the
        // expansion for a large argument.
        return hankel(n, x.abs()).map(|(j, _)| if x < 0.0 && n % 2 == 1 { -j } else { j });
    }
    let points = points as usize;
    let step = 2.0 * std::f64::consts::PI / points as f64;
    let order = f64::from(n);
    let mut sum = 0.0;
    for k in 0..points {
        let t = k as f64 * step;
        sum += (order * t - x * t.sin()).cos();
    }
    Some(sum / points as f64)
}

/// The expansion for a **large** argument: the first and the second
/// kind together.
fn hankel(n: u32, x: f64) -> Option<(f64, f64)> {
    let mu = 4.0 * f64::from(n) * f64::from(n);
    let mut p = 0.0;
    let mut q = 0.0;
    let mut term = 1.0f64;
    let mut last = f64::INFINITY;
    for k in 0..200 {
        if term.abs() > last {
            break;
        }
        let t = if k % 4 < 2 { term } else { -term };
        if k % 2 == 0 {
            p += t;
        } else {
            q += t;
        }
        last = term.abs();
        if last < 1e-17 {
            break;
        }
        let kk = (k + 1) as f64;
        let odd = 2.0 * kk - 1.0;
        term *= (mu - odd * odd) / (kk * 8.0 * x);
    }
    let chi = x - (f64::from(n) / 2.0 + 0.25) * std::f64::consts::PI;
    let factor = (2.0 / (std::f64::consts::PI * x)).sqrt();
    let j = factor * (p * chi.cos() - q * chi.sin());
    let y = factor * (p * chi.sin() + q * chi.cos());
    if j.is_finite() && y.is_finite() { Some((j, y)) } else { None }
}

/// The first kind, `J_0` to `J_m`, by Miller's **backward**
/// recurrence, normalised with `J_0 + 2ΣJ_2k = 1`.
fn j_all(x: f64) -> Vec<f64> {
    let top = x.ceil() as usize + 30 + (40.0 * x.max(1.0)).sqrt() as usize;
    let top = top + top % 2;
    let mut j = vec![0.0f64; top + 2];
    if let Some(slot) = j.get_mut(top) {
        *slot = 1e-30;
    }
    for k in (1..=top).rev() {
        let here = j.get(k).copied().unwrap_or(0.0);
        let next = j.get(k + 1).copied().unwrap_or(0.0);
        let mut previous = 2.0 * k as f64 / x * here - next;
        if previous.abs() > 1e250 {
            for value in j.iter_mut() {
                *value *= 1e-250;
            }
            previous *= 1e-250;
        }
        if let Some(slot) = j.get_mut(k - 1) {
            *slot = previous;
        }
    }
    let norm = j.first().copied().unwrap_or(0.0) + 2.0 * j.iter().skip(2).step_by(2).sum::<f64>();
    if norm != 0.0 {
        for value in j.iter_mut() {
            *value /= norm;
        }
    }
    j
}

/// The second kind, `Y_0` and `Y_1`, by the **Neumann series** over
/// the first kind.
fn y01_of(x: f64) -> Option<(f64, f64)> {
    if x > 25.0 {
        let (_, y0) = hankel(0, x)?;
        let (_, y1) = hankel(1, x)?;
        return Some((y0, y1));
    }
    let j = j_all(x);
    let at = |k: usize| j.get(k).copied().unwrap_or(0.0);
    let l = (x / 2.0).ln() + EULER;
    let mut s0 = 0.0;
    let mut s1 = 0.0;
    let mut k = 1usize;
    while 2 * k + 1 < j.len() {
        let sign = if k.is_multiple_of(2) { 1.0 } else { -1.0 };
        s0 += sign * at(2 * k) / k as f64;
        s1 += sign * (at(2 * k - 1) - at(2 * k + 1)) / k as f64;
        k += 1;
    }
    let y0 = (2.0 / std::f64::consts::PI) * l * at(0) - (4.0 / std::f64::consts::PI) * s0;
    let y1 = -(2.0 / std::f64::consts::PI) * (at(0) / x - l * at(1)) + (2.0 / std::f64::consts::PI) * s1;
    if y0.is_finite() && y1.is_finite() { Some((y0, y1)) } else { None }
}

/// The second kind, `Y_n(x)`, carried up by the recurrence.
fn y_of(n: u32, x: f64) -> Option<f64> {
    let (y0, y1) = y01_of(x)?;
    if n == 0 {
        return Some(y0);
    }
    let (mut a, mut b) = (y0, y1);
    for k in 1..n {
        let c = 2.0 * k as f64 / x * b - a;
        a = b;
        b = c;
        if !b.is_finite() {
            return None;
        }
    }
    Some(b)
}

/// The **modified** first kind, `I_n(x)`, by its **power series**.
///
/// The terms are all of one sign, and the first of them is worked out
/// through its logarithm, so that a number that is too large for the
/// series is still a number.
fn i_of(n: u32, x: f64) -> Option<f64> {
    if x == 0.0 {
        return Some(if n == 0 { 1.0 } else { 0.0 });
    }
    let ax = x.abs();
    let order = f64::from(n);
    let log_first = order * (ax / 2.0).ln() - crate::special::ln_gamma(order + 1.0);
    let q = ax * ax / 4.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut k = 0.0;
    while k < 1e6 {
        k += 1.0;
        term *= q / (k * (k + order));
        sum += term;
        if term < 1e-17 * sum {
            break;
        }
        if !sum.is_finite() {
            return None;
        }
    }
    let value = (log_first + sum.ln()).exp();
    let value = if x < 0.0 && n % 2 == 1 { -value } else { value };
    if value.is_finite() { Some(value) } else { None }
}

/// The **modified** second kind, `K_n(x)`, by its **integral** and
/// then the recurrence.
fn k_of(n: u32, x: f64) -> Option<f64> {
    let integral = |nu: f64| -> f64 {
        let step = 0.02f64;
        let mut sum = 0.5 * (-x).exp();
        let mut t = step;
        while t < 800.0 {
            let v = (-x * t.cosh() + ln_cosh(nu * t)).exp();
            sum += v;
            if v < 1e-18 * sum && x * t.cosh() > 50.0 {
                break;
            }
            t += step;
        }
        sum * step
    };
    let k0 = integral(0.0);
    if n == 0 {
        return Some(k0);
    }
    let k1 = integral(1.0);
    let (mut a, mut b) = (k0, k1);
    for k in 1..n {
        let c = a + 2.0 * k as f64 / x * b;
        a = b;
        b = c;
        if !b.is_finite() {
            return None;
        }
    }
    if b.is_finite() { Some(b) } else { None }
}

/// The logarithm of the **hyperbolic cosine**, without an overflow.
fn ln_cosh(x: f64) -> f64 {
    let a = x.abs();
    a + (-2.0 * a).exp().ln_1p() - std::f64::consts::LN_2
}

impl Book {
    /// The **Bessel functions**.
    pub(crate) fn bessel(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if arguments.len() != 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let x = match self.number_at(sheet, arguments, 0, being) {
            Ok(number) => number,
            Err(message) => return Value::Error(message),
        };
        let order = match self.number_at(sheet, arguments, 1, being) {
            Ok(number) => number.trunc(),
            Err(message) => return Value::Error(message),
        };
        // The order is a **whole number** that is not negative and not
        // above a hundred thousand.
        if order < 0.0 || order > 100_000.0 || !order.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        let order = order as u32;
        let value = match name {
            "BESSELJ" => j_of(order, x),
            "BESSELY" => {
                if x <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                y_of(order, x)
            }
            "BESSELI" => i_of(order, x),
            _ => {
                if x <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                k_of(order, x)
            }
        };
        match value {
            Some(number) if number.is_finite() => Value::Number(number),
            _ => Value::Error("#NUM!".to_string()),
        }
    }
}
