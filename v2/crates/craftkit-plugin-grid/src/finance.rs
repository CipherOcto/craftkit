//! The **value** of money over time.
//!
//! A formula may ask what a sum of money is worth now and later:
//!
//! - `PMT(rate, nper, pv, [fv], [type])` — the **payment** of a
//!   loan;
//! - `PV(rate, nper, pmt, [fv], [type])` — what a series of payments
//!   is worth **now**;
//! - `FV(rate, nper, pmt, [pv], [type])` — what it is worth **later**;
//! - `NPER(rate, pmt, pv, [fv], [type])` — how many payments it
//!   takes;
//! - `RATE(nper, pmt, pv, [fv], [type], [guess])` — the **rate** that
//!   makes the sum work out;
//! - `IPMT` and `PPMT` — the **interest** and the **principal** of
//!   one payment;
//! - `ISPMT` — the interest of a loan with even principal payments;
//! - `EFFECT` and `NOMINAL` — a rate that is compounded and the rate
//!   it is made of;
//! - `PDURATION` and `RRI` — how long a sum takes to grow and the
//!   rate it grows at.
//!
//! The rules are the **tree's**: a payment of no periods is `#NUM!`,
//! a rate of minus one or less is no rate, and the **type** — whether
//! the payment comes at the beginning of the period or at the end —
//! is the tree's own: any number that is not zero means the beginning.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **financial functions**.
pub fn is_financial_function(name: &str) -> bool {
    matches!(
        name,
        "PMT"
            | "PV"
            | "FV"
            | "NPER"
            | "RATE"
            | "IPMT"
            | "PPMT"
            | "ISPMT"
            | "EFFECT"
            | "NOMINAL"
            | "PDURATION"
            | "RRI"
    )
}

/// The **type** of a payment: the beginning of the period or the end.
///
/// The tree's rule: any number that is not zero means the beginning.
fn type_of(number: Option<f64>) -> f64 {
    match number {
        Some(number) if number != 0.0 => 1.0,
        _ => 0.0,
    }
}

/// The **payment** of a loan.
pub(crate) fn pmt_of(rate: f64, nper: f64, pv: f64, fv: f64, typ: f64) -> f64 {
    if rate == 0.0 {
        return -(pv + fv) / nper;
    }
    if rate <= -1.0 {
        return f64::NAN;
    }
    let log = nper * rate.ln_1p();
    let power = log.exp_m1();
    let factor = 1.0 + rate * typ;
    if power > 0.0 {
        -(pv * rate + (pv + fv) * (rate / power)) / factor
    } else if power > -0.5 {
        -(rate * (pv + fv + pv * power)) / (factor * power)
    } else {
        -(rate * (fv + pv * log.exp())) / (factor * power)
    }
}

/// The **future value** of a series of payments.
pub(crate) fn fv_of(rate: f64, nper: f64, pmt: f64, pv: f64, typ: f64) -> f64 {
    if rate == 0.0 {
        return -(pv + pmt * nper);
    }
    let power = (1.0 + rate).powf(nper);
    -(pv * power + pmt * (1.0 + rate * typ) * (power - 1.0) / rate)
}

/// The **present value** of a series of payments.
pub(crate) fn pv_of(rate: f64, nper: f64, pmt: f64, fv: f64, typ: f64) -> f64 {
    if rate == 0.0 {
        return -(fv + pmt * nper);
    }
    let power = (1.0 + rate).powf(nper);
    -(fv + pmt * (1.0 + rate * typ) * (power - 1.0) / rate) / power
}

/// The **interest** of one payment of a loan.
pub(crate) fn ipmt_of(rate: f64, per: f64, nper: f64, pv: f64, fv: f64, typ: f64) -> f64 {
    let payment = pmt_of(rate, nper, pv, fv, typ);
    if typ == 1.0 && per == 1.0 {
        return 0.0;
    }
    let balance = fv_of(rate, per - 1.0, payment, pv, typ);
    let interest = balance * rate;
    if typ == 1.0 { interest / (1.0 + rate) } else { interest }
}

/// The **rate** that makes a sum work out.
///
/// The tree's rule: a **Newton** step from a guess, and when that
/// fails a **bisection** over a grid of rates.
pub(crate) fn solve_rate(function: impl Fn(f64) -> f64, guess: f64) -> Result<f64, String> {
    let check = |rate: f64| {
        function(rate).is_finite() && function(rate).abs() < 1e-7 * (1.0 + function(guess).abs().min(1e12))
    };
    for start in [guess, 0.1, 0.01, 0.5, -0.5, 1.0, 0.001, 5.0, -0.9, 10.0] {
        if let Some(rate) = newton(&function, start)
            && rate > -1.0
            && check(rate)
        {
            return Ok(rate);
        }
    }
    // A scan for a change of sign, and a bisection.
    let grid: [f64; 16] =
        [-0.999_999, -0.99, -0.9, -0.5, -0.2, 0.0, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 100.0, 1000.0];
    for window in grid.windows(2) {
        let (mut low, mut high) = match window {
            [low, high] => (*low, *high),
            _ => continue,
        };
        let (low_value, high_value) = (function(low), function(high));
        if !(low_value.is_finite() && high_value.is_finite()) || low_value * high_value > 0.0 {
            continue;
        }
        let low_negative = low_value < 0.0;
        for _ in 0..200 {
            let middle = 0.5 * (low + high);
            let value = function(middle);
            if (value < 0.0) == low_negative {
                low = middle;
            } else {
                high = middle;
            }
        }
        return Ok(0.5 * (low + high));
    }
    Err("#NUM!".to_string())
}

/// The **Newton** step: the rate that makes the sum work out.
pub(crate) fn newton(function: &impl Fn(f64) -> f64, guess: f64) -> Option<f64> {
    let mut x = guess;
    for _ in 0..200 {
        let y = function(x);
        if !y.is_finite() {
            return None;
        }
        let step = 1e-6 * x.abs().max(1e-3);
        let slope = (function(x + step) - function(x - step)) / (2.0 * step);
        if slope == 0.0 || !slope.is_finite() {
            return None;
        }
        let mut next = x - y / slope;
        if next <= -1.0 {
            next = (x - 1.0) / 2.0;
            if next <= -1.0 {
                next = -0.999_999_999;
            }
        }
        if (next - x).abs() < 1e-12 * next.abs().max(1.0) {
            return Some(next);
        }
        x = next;
    }
    Some(x)
}

impl Book {
    /// The **financial functions**.
    pub(crate) fn finance(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let value = match name {
            "PMT" => {
                let (rate, nper, pv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(rate), Ok(nper), Ok(pv)) => (rate, nper, pv),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let fv = match self.optional_number(sheet, arguments, 3, being) {
                    Ok(fv) => fv.unwrap_or(0.0),
                    Err(message) => return Value::Error(message),
                };
                let typ = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(typ) => type_of(typ),
                    Err(message) => return Value::Error(message),
                };
                if nper == 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                pmt_of(rate, nper, pv, fv, typ)
            }
            "PV" | "FV" => {
                let (rate, nper, pmt) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(rate), Ok(nper), Ok(pmt)) => (rate, nper, pmt),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let other = match self.optional_number(sheet, arguments, 3, being) {
                    Ok(other) => other.unwrap_or(0.0),
                    Err(message) => return Value::Error(message),
                };
                let typ = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(typ) => type_of(typ),
                    Err(message) => return Value::Error(message),
                };
                if name == "PV" {
                    pv_of(rate, nper, pmt, other, typ)
                } else {
                    fv_of(rate, nper, pmt, other, typ)
                }
            }
            "NPER" => {
                let (rate, pmt, pv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(rate), Ok(pmt), Ok(pv)) => (rate, pmt, pv),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let fv = match self.optional_number(sheet, arguments, 3, being) {
                    Ok(fv) => fv.unwrap_or(0.0),
                    Err(message) => return Value::Error(message),
                };
                let typ = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(typ) => type_of(typ),
                    Err(message) => return Value::Error(message),
                };
                if rate == 0.0 {
                    if pmt == 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    -(pv + fv) / pmt
                } else {
                    let numerator = pmt * (1.0 + rate * typ) - fv * rate;
                    let denominator = pv * rate + pmt * (1.0 + rate * typ);
                    if denominator == 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    let quotient = numerator / denominator;
                    if quotient <= 0.0 || rate <= -1.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    quotient.ln() / (1.0 + rate).ln()
                }
            }
            "RATE" => {
                let (nper, pmt, pv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(nper), Ok(pmt), Ok(pv)) => (nper, pmt, pv),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let fv = match self.optional_number(sheet, arguments, 3, being) {
                    Ok(fv) => fv.unwrap_or(0.0),
                    Err(message) => return Value::Error(message),
                };
                let typ = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(typ) => type_of(typ),
                    Err(message) => return Value::Error(message),
                };
                let guess = match self.optional_number(sheet, arguments, 5, being) {
                    Ok(guess) => guess.unwrap_or(0.1),
                    Err(message) => return Value::Error(message),
                };
                if nper <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let function = |rate: f64| {
                    if rate.abs() < 1e-12 {
                        pv + pmt * nper + fv
                    } else {
                        let power = (1.0 + rate).powf(nper);
                        pv * power + pmt * (1.0 + rate * typ) * (power - 1.0) / rate + fv
                    }
                };
                match solve_rate(function, guess) {
                    Ok(rate) => rate,
                    Err(message) => return Value::Error(message),
                }
            }
            "IPMT" | "PPMT" => {
                let (rate, per, nper, pv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(rate), Ok(per), Ok(nper), Ok(pv)) => (rate, per, nper, pv),
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let fv = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(fv) => fv.unwrap_or(0.0),
                    Err(message) => return Value::Error(message),
                };
                let typ = match self.optional_number(sheet, arguments, 5, being) {
                    Ok(typ) => type_of(typ),
                    Err(message) => return Value::Error(message),
                };
                if per < 1.0 || per > nper || nper == 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let interest = ipmt_of(rate, per, nper, pv, fv, typ);
                if name == "IPMT" { interest } else { pmt_of(rate, nper, pv, fv, typ) - interest }
            }
            "ISPMT" => {
                let (rate, per, nper, pv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(rate), Ok(per), Ok(nper), Ok(pv)) => (rate, per, nper, pv),
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if nper == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                pv * rate * (per / nper - 1.0)
            }
            "EFFECT" => {
                let (nominal, periods) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                ) {
                    (Ok(nominal), Ok(periods)) => (nominal, periods),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let periods = periods.trunc();
                if nominal <= 0.0 || periods < 1.0 {
                    return Value::Error("#NUM!".to_string());
                }
                (1.0 + nominal / periods).powf(periods) - 1.0
            }
            "NOMINAL" => {
                let (effective, periods) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                ) {
                    (Ok(effective), Ok(periods)) => (effective, periods),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let periods = periods.trunc();
                if effective <= 0.0 || periods < 1.0 {
                    return Value::Error("#NUM!".to_string());
                }
                periods * ((1.0 + effective).powf(1.0 / periods) - 1.0)
            }
            "PDURATION" => {
                let (rate, pv, fv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(rate), Ok(pv), Ok(fv)) => (rate, pv, fv),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if rate <= 0.0 || pv <= 0.0 || fv <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                (fv.ln() - pv.ln()) / (1.0 + rate).ln()
            }
            _ => {
                // `RRI`: the rate a sum grows at.
                let (nper, pv, fv) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(nper), Ok(pv), Ok(fv)) => (nper, pv, fv),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if nper <= 0.0 || pv == 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                (fv / pv).powf(1.0 / nper) - 1.0
            }
        };
        if value.is_finite() { Value::Number(value) } else { Value::Error("#NUM!".to_string()) }
    }
}
