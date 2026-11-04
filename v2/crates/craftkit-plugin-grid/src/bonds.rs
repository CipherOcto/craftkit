//! The **coupons** of a bond, and the **price** of one.
//!
//! A formula may ask about a bond — a security that pays its interest
//! on a fixed day of the year and is redeemed at its value:
//!
//! - `COUPDAYBS(settlement, maturity, frequency, [basis])` — the days
//!   from the **beginning** of the coupon period to the settlement;
//! - `COUPDAYS(settlement, maturity, frequency, [basis])` — the days
//!   of the **coupon period**;
//! - `COUPDAYSNC(settlement, maturity, frequency, [basis])` — the days
//!   from the settlement to the **next** coupon;
//! - `COUPNUM(settlement, maturity, frequency, [basis])` — the number
//!   of coupons;
//! - `COUPNCD(settlement, maturity, frequency, [basis])` — the date of
//!   the **next** coupon;
//! - `COUPPCD(settlement, maturity, frequency, [basis])` — the date of
//!   the **previous** coupon;
//! - `PRICE(settlement, maturity, rate, yield, redemption, frequency,
//!   [basis])` — the **price** of the bond;
//! - `YIELD(settlement, maturity, rate, price, redemption, frequency,
//!   [basis])` — the **yield** of the bond;
//! - `DURATION` and `MDURATION` — how long the money is tied up.
//!
//! The rules are the **tree's**: a **frequency** is one, two or four
//! and no other, a **basis** is a number from zero to four and no
//! other, and a settlement that is not before the maturity is a number
//! error.

use crate::dates::System;
use crate::daycounts::{basis_of, day_count, frequency_of, shift_months, year_frac, ymd};
use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **bond functions**.
pub fn is_bond_function(name: &str) -> bool {
    matches!(
        name,
        "COUPDAYBS"
            | "COUPDAYS"
            | "COUPDAYSNC"
            | "COUPNUM"
            | "COUPNCD"
            | "COUPPCD"
            | "PRICE"
            | "YIELD"
            | "DURATION"
            | "MDURATION"
            | "PRICEMAT"
            | "YIELDMAT"
    )
}

/// A **coupon period**: the dates around it and how many there are.
struct Coupon {
    /// The date of the previous coupon.
    pcd: f64,
    /// The date of the next coupon.
    ncd: f64,
    /// How many coupons there are.
    num: f64,
}

/// The coupon period a settlement falls in.
fn coupons(system: System, settlement: f64, maturity: f64, frequency: f64) -> Option<Coupon> {
    if settlement >= maturity {
        return None;
    }
    let anchor = ymd(system, maturity)?;
    let end_of_month = anchor.2 == crate::dates::days_in_month(anchor.0, anchor.1);
    let step = (12.0 / frequency) as i64;
    // Estimate the count from the span, then adjust it.
    let mut count = (((maturity - settlement) / 365.25 * frequency).floor() as i64).max(1);
    while count > 1 && shift_months(system, anchor, -(count - 1) * step, end_of_month)? <= settlement {
        count -= 1;
    }
    let mut guard = 0;
    while shift_months(system, anchor, -count * step, end_of_month)? > settlement {
        count += 1;
        guard += 1;
        if guard > 100_000 {
            return None;
        }
    }
    Some(Coupon {
        pcd: shift_months(system, anchor, -count * step, end_of_month)?,
        ncd: shift_months(system, anchor, -(count - 1) * step, end_of_month)?,
        num: count as f64,
    })
}

/// The days of a **coupon period** under a basis.
fn coupon_days(coupon: &Coupon, frequency: f64, basis: u32) -> f64 {
    match basis {
        1 => coupon.ncd - coupon.pcd,
        3 => 365.0 / frequency,
        _ => 360.0 / frequency,
    }
}

/// The days from the **beginning** of the coupon period to the
/// settlement.
fn coupon_daybs(system: System, coupon: &Coupon, settlement: f64, basis: u32) -> Option<f64> {
    day_count(system, coupon.pcd, settlement, basis)
}

/// The days from the settlement to the **next** coupon.
fn coupon_daysnc(
    system: System,
    coupon: &Coupon,
    settlement: f64,
    frequency: f64,
    basis: u32,
) -> Option<f64> {
    match basis {
        0 | 4 => {
            Some(coupon_days(coupon, frequency, basis) - coupon_daybs(system, coupon, settlement, basis)?)
        }
        _ => Some(coupon.ncd - settlement),
    }
}

/// A **bond**: its coupons and the days around the settlement.
struct Bond {
    /// How many coupons there are.
    num: f64,
    /// The days of a coupon period.
    days: f64,
    /// The days of accrued interest.
    accrued: f64,
    /// The days from the settlement to the next coupon.
    remaining: f64,
    /// How many times a year it pays.
    frequency: f64,
}

/// The bond a settlement falls in.
fn bond(system: System, settlement: f64, maturity: f64, frequency: f64, basis: u32) -> Option<Bond> {
    let coupon = coupons(system, settlement, maturity, frequency)?;
    let days = coupon_days(&coupon, frequency, basis);
    let accrued = coupon_daybs(system, &coupon, settlement, basis)?;
    let remaining = coupon_daysnc(system, &coupon, settlement, frequency, basis)?;
    if days <= 0.0 {
        return None;
    }
    Some(Bond { num: coupon.num, days, accrued, remaining, frequency })
}

/// The **price** of a bond.
fn bond_price(bond: &Bond, rate: f64, yield_: f64, redemption: f64) -> f64 {
    let frequency = bond.frequency;
    let coupon = 100.0 * rate / frequency;
    let accrued = coupon * bond.accrued / bond.days;
    if bond.num == 1.0 {
        return (redemption + coupon) / (1.0 + bond.remaining / bond.days * yield_ / frequency) - accrued;
    }
    let base = 1.0 + yield_ / frequency;
    let fraction = bond.remaining / bond.days;
    let mut sum = redemption / base.powf(bond.num - 1.0 + fraction);
    let mut step = 1.0;
    while step <= bond.num {
        sum += coupon / base.powf(step - 1.0 + fraction);
        step += 1.0;
    }
    sum - accrued
}

impl Book {
    /// The **bond functions**.
    pub(crate) fn bonds(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let system = self.system();
        match name {
            "COUPDAYBS" | "COUPDAYS" | "COUPDAYSNC" | "COUPNUM" | "COUPNCD" | "COUPPCD" => {
                let (settlement, maturity, frequency) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(settlement), Ok(maturity), Ok(frequency)) => (settlement, maturity, frequency),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let frequency = match frequency_of(frequency) {
                    Ok(frequency) => frequency,
                    Err(message) => return Value::Error(message),
                };
                let basis = match self.optional_number(sheet, arguments, 3, being) {
                    Ok(Some(basis)) => basis,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let basis = match basis_of(Some(basis)) {
                    Ok(basis) => basis,
                    Err(message) => return Value::Error(message),
                };
                let Some(coupon) = coupons(system, settlement, maturity, frequency) else {
                    return Value::Error("#NUM!".to_string());
                };
                match name {
                    "COUPDAYBS" => match coupon_daybs(system, &coupon, settlement, basis) {
                        Some(days) => Value::Number(days),
                        None => Value::Error("#NUM!".to_string()),
                    },
                    "COUPDAYS" => Value::Number(coupon_days(&coupon, frequency, basis)),
                    "COUPDAYSNC" => match coupon_daysnc(system, &coupon, settlement, frequency, basis) {
                        Some(days) => Value::Number(days),
                        None => Value::Error("#NUM!".to_string()),
                    },
                    "COUPNUM" => Value::Number(coupon.num),
                    "COUPNCD" => Value::Number(coupon.ncd),
                    _ => Value::Number(coupon.pcd),
                }
            }
            "DURATION" | "MDURATION" => {
                let (settlement, maturity, rate, yield_) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(settlement), Ok(maturity), Ok(rate), Ok(yield_)) => {
                        (settlement, maturity, rate, yield_)
                    }
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let frequency = match self.number_at(sheet, arguments, 4, being) {
                    Ok(frequency) => match frequency_of(frequency) {
                        Ok(frequency) => frequency,
                        Err(message) => return Value::Error(message),
                    },
                    Err(message) => return Value::Error(message),
                };
                let basis = match self.optional_number(sheet, arguments, 5, being) {
                    Ok(Some(basis)) => basis,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let basis = match basis_of(Some(basis)) {
                    Ok(basis) => basis,
                    Err(message) => return Value::Error(message),
                };
                if rate < 0.0 || yield_ < 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let Some(bond) = bond(system, settlement, maturity, frequency, basis) else {
                    return Value::Error("#NUM!".to_string());
                };
                let coupon = rate * 100.0 / frequency;
                let base = 1.0 + yield_ / frequency;
                let fraction = bond.remaining / bond.days;
                let mut weighted = 0.0;
                let mut sum = 0.0;
                let mut step = 1.0;
                while step <= bond.num {
                    let time = step - 1.0 + fraction;
                    let flow = if step == bond.num { coupon + 100.0 } else { coupon };
                    let present = flow / base.powf(time);
                    weighted += time * present;
                    sum += present;
                    step += 1.0;
                }
                if sum == 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let duration = weighted / sum / frequency;
                if name == "DURATION" {
                    Value::Number(duration)
                } else {
                    Value::Number(duration / (1.0 + yield_ / frequency))
                }
            }
            "PRICEMAT" | "YIELDMAT" => {
                // A bond that is paid out: the price of it, and the
                // yield that makes it.
                let (settlement, maturity, issue, rate, fifth) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                    self.number_at(sheet, arguments, 4, being),
                ) {
                    (Ok(settlement), Ok(maturity), Ok(issue), Ok(rate), Ok(fifth)) => {
                        (settlement, maturity, issue, rate, fifth)
                    }
                    (Err(message), _, _, _, _)
                    | (_, Err(message), _, _, _)
                    | (_, _, Err(message), _, _)
                    | (_, _, _, Err(message), _)
                    | (_, _, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let basis = match self.optional_number(sheet, arguments, 5, being) {
                    Ok(Some(basis)) => basis,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let basis = match basis_of(Some(basis)) {
                    Ok(basis) => basis,
                    Err(message) => return Value::Error(message),
                };
                if settlement >= maturity || issue >= settlement || rate < 0.0 || fifth < 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let Some(whole) = year_frac(system, issue, maturity, basis) else {
                    return Value::Error("#NUM!".to_string());
                };
                let Some(remaining) = year_frac(system, settlement, maturity, basis) else {
                    return Value::Error("#NUM!".to_string());
                };
                let Some(paid) = year_frac(system, issue, settlement, basis) else {
                    return Value::Error("#NUM!".to_string());
                };
                if name == "PRICEMAT" {
                    Value::Number(
                        (100.0 + whole * rate * 100.0) / (1.0 + remaining * fifth) - paid * rate * 100.0,
                    )
                } else {
                    if fifth <= 0.0 || remaining == 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    let base = fifth / 100.0 + paid * rate;
                    Value::Number(((1.0 + whole * rate) - base) / base / remaining)
                }
            }
            _ => {
                // `PRICE` and `YIELD`: the price of a bond and the yield
                // that makes it.
                let (settlement, maturity, rate, third, redemption) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                    self.number_at(sheet, arguments, 4, being),
                ) {
                    (Ok(settlement), Ok(maturity), Ok(rate), Ok(third), Ok(redemption)) => {
                        (settlement, maturity, rate, third, redemption)
                    }
                    (Err(message), _, _, _, _)
                    | (_, Err(message), _, _, _)
                    | (_, _, Err(message), _, _)
                    | (_, _, _, Err(message), _)
                    | (_, _, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let frequency = match self.number_at(sheet, arguments, 5, being) {
                    Ok(frequency) => match frequency_of(frequency) {
                        Ok(frequency) => frequency,
                        Err(message) => return Value::Error(message),
                    },
                    Err(message) => return Value::Error(message),
                };
                let basis = match self.optional_number(sheet, arguments, 6, being) {
                    Ok(Some(basis)) => basis,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let basis = match basis_of(Some(basis)) {
                    Ok(basis) => basis,
                    Err(message) => return Value::Error(message),
                };
                if name == "PRICE" {
                    if rate < 0.0 || third < 0.0 || redemption <= 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    let Some(bond) = bond(system, settlement, maturity, frequency, basis) else {
                        return Value::Error("#NUM!".to_string());
                    };
                    Value::Number(bond_price(&bond, rate, third, redemption))
                } else {
                    if rate < 0.0 || third <= 0.0 || redemption <= 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    let Some(bond) = bond(system, settlement, maturity, frequency, basis) else {
                        return Value::Error("#NUM!".to_string());
                    };
                    if bond.num == 1.0 {
                        let base = third / 100.0 + bond.accrued / bond.days * rate / frequency;
                        return Value::Number(
                            ((redemption / 100.0 + rate / frequency) - base) / base * frequency * bond.days
                                / bond.remaining,
                        );
                    }
                    // The price falls as the yield rises: bracket the
                    // root, then bisect it.
                    let difference = |yield_: f64| bond_price(&bond, rate, yield_, redemption) - third;
                    let (mut low, mut high) = (-0.999_999 * frequency, 1.0);
                    let mut guard = 0;
                    while difference(high) > 0.0 && guard < 60 {
                        high *= 2.0;
                        guard += 1;
                    }
                    if !(difference(low) >= 0.0 && difference(high) <= 0.0) {
                        return Value::Error("#NUM!".to_string());
                    }
                    for _ in 0..200 {
                        let middle = 0.5 * (low + high);
                        if difference(middle) > 0.0 {
                            low = middle;
                        } else {
                            high = middle;
                        }
                    }
                    Value::Number(0.5 * (low + high))
                }
            }
        }
    }
}
