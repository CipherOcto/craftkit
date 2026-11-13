//! The **price** of a discount security.
//!
//! A formula may ask about a security that is sold at a discount and
//! redeemed at its value:
//!
//! - `DISC(settlement, maturity, price, redemption, [basis])` — the
//!   **discount** of such a security;
//! - `PRICEDISC(settlement, maturity, discount, redemption, [basis])`
//!   — the **price** of one, from its discount;
//! - `YIELDDISC(settlement, maturity, price, redemption, [basis])` —
//!   the **yield** of one;
//! - `INTRATE(settlement, maturity, investment, redemption, [basis])`
//!   — the **rate of interest** of one;
//! - `RECEIVED(settlement, maturity, investment, discount, [basis])`
//!   — the amount **received** at the end;
//! - `TBILLPRICE(settlement, maturity, discount)` — the price of a
//!   **bill**;
//! - `TBILLYIELD(settlement, maturity, price)` — the yield of a bill;
//! - `TBILLEQ(settlement, maturity, discount)` — the **equivalent
//!   yield** of a bill, on the same footing as a bond's yield.
//!
//! The rules are the **tree's**: a settlement that is not before the
//! maturity is a number error, a price or a redemption of nothing is a
//! number error, and a **basis** is a number from zero to four and no
//! other.

use crate::dates::System;
use crate::formula::{Book, CellRef, Expr, Value};
pub const NAMES: &[&str] =
    &["DISC", "PRICEDISC", "YIELDDISC", "INTRATE", "RECEIVED", "TBILLPRICE", "TBILLYIELD", "TBILLEQ"];

/// The **fraction of a year** between a settlement and a maturity.
fn fraction(system: System, settlement: f64, maturity: f64, basis: u32) -> Option<f64> {
    let days = crate::daycounts::day_count(system, settlement, maturity, basis)?;
    Some(days / crate::daycounts::year_basis(system, basis, settlement)?)
}

/// The date **twelve months** after a date, for the rule that a bill
/// is at most a year long.
fn a_year_after(system: System, settlement: f64) -> Option<f64> {
    let anchor = crate::daycounts::ymd(system, settlement)?;
    crate::daycounts::shift_months(system, anchor, 12, false)
}

/// The **settlement**, the **maturity** and the two prices of a
/// discount security, and the basis.
fn discount_terms(settlement: f64, maturity: f64, first_price: f64, second_price: f64) -> Result<(), String> {
    if settlement >= maturity || first_price <= 0.0 || second_price <= 0.0 {
        return Err("#NUM!".to_string());
    }
    Ok(())
}

impl Book {
    /// The **security functions**.
    pub(crate) fn securities(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let system = self.system();
        match name {
            "TBILLPRICE" | "TBILLYIELD" | "TBILLEQ" => {
                let (settlement, maturity, third) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(settlement), Ok(maturity), Ok(third)) => (settlement, maturity, third),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if settlement >= maturity || third <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let Some(limit) = a_year_after(system, settlement) else {
                    return Value::Error("#NUM!".to_string());
                };
                if maturity > limit {
                    return Value::Error("#NUM!".to_string());
                }
                let days = maturity - settlement;
                match name {
                    "TBILLPRICE" => {
                        let price = 100.0 * (1.0 - third * days / 360.0);
                        if price <= 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(price)
                    }
                    "TBILLEQ" => {
                        if days <= 182.0 {
                            return Value::Number(365.0 * third / (360.0 - third * days));
                        }
                        let price = 100.0 * (1.0 - third * days / 360.0);
                        if price <= 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        let t = days / 365.0;
                        let discriminant = t * t - (2.0 * t - 1.0) * (1.0 - 100.0 / price);
                        if discriminant < 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number((-t + discriminant.sqrt()) / (t - 0.5))
                    }
                    _ => Value::Number((100.0 - third) / third * 360.0 / days),
                }
            }
            _ => {
                // The discount securities: settlement, maturity, two
                // numbers and a basis.
                let (settlement, maturity, first, second) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(settlement), Ok(maturity), Ok(first), Ok(second)) => {
                        (settlement, maturity, first, second)
                    }
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let basis = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(Some(basis)) => basis,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let basis = match crate::daycounts::basis_of(Some(basis)) {
                    Ok(basis) => basis,
                    Err(message) => return Value::Error(message),
                };
                if let Err(message) = discount_terms(settlement, maturity, first, second) {
                    return Value::Error(message);
                }
                let Some(fraction) = fraction(system, settlement, maturity, basis) else {
                    return Value::Error("#NUM!".to_string());
                };
                match name {
                    "DISC" => Value::Number((1.0 - first / second) / fraction),
                    "PRICEDISC" => Value::Number(second - first * second * fraction),
                    "YIELDDISC" => Value::Number((second / first - 1.0) / fraction),
                    "INTRATE" => Value::Number((second - first) / first / fraction),
                    _ => {
                        // `RECEIVED`: what is paid at the end.
                        let denominator = 1.0 - second * fraction;
                        if denominator <= 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(first / denominator)
                    }
                }
            }
        }
    }
}
