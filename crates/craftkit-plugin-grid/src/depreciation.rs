//! The **wearing out** of an asset.
//!
//! A formula may ask what a thing that wears out is worth now, a
//! period at a time:
//!
//! - `SLN(cost, salvage, life)` — the **straight line** depreciation:
//!   the same every period;
//! - `SYD(cost, salvage, life, period)` — the **sum of the years**:
//!   less and less, because the years are added up and each one takes
//!   its share;
//! - `DB(cost, salvage, life, period, [month])` — the **declining
//!   balance**;
//! - `DDB(cost, salvage, life, period, [factor])` — the **double
//!   declining balance**;
//! - `VDB(cost, salvage, life, start, end, [factor], [no_switch])` —
//!   the declining balance over a span of periods, with the switch to
//!   the straight line left in.
//!
//! The rules are the **tree's**: a life of no periods is a division by
//! nothing, a period beyond the life is a number error, and the rate of
//! the declining balance is the tree's own — a number written with
//! fifteen significant digits and rounded to three.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **depreciation functions**.
pub fn is_depreciation_function(name: &str) -> bool {
    matches!(name, "SLN" | "SYD" | "DB" | "DDB" | "VDB")
}

/// The **straight line** depreciation: the same every period.
fn sln(cost: f64, salvage: f64, life: f64) -> Value {
    if life == 0.0 {
        return Value::Error("#DIV/0!".to_string());
    }
    Value::Number((cost - salvage) / life)
}

/// The **sum of the years** depreciation.
fn syd(cost: f64, salvage: f64, life: f64, period: f64) -> Value {
    if life <= 0.0 || period <= 0.0 || period > life || salvage < 0.0 {
        return Value::Error("#NUM!".to_string());
    }
    Value::Number((cost - salvage) * (life - period + 1.0) * 2.0 / (life * (life + 1.0)))
}

/// The **declining balance** depreciation.
fn db(cost: f64, salvage: f64, life: f64, period: f64, month: f64) -> Value {
    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || period <= 0.0 || !(1.0..=12.0).contains(&month) {
        return Value::Error("#NUM!".to_string());
    }
    let max_period = if month == 12.0 { life.trunc() } else { life.trunc() + 1.0 };
    if period > max_period.max(1.0) || period > 1.0e7 {
        return Value::Error("#NUM!".to_string());
    }
    if cost == 0.0 {
        return Value::Number(0.0);
    }
    let rate = crate::arithmetic::round_half_away(1.0 - (salvage / cost).powf(1.0 / life), 3);
    let mut total = 0.0;
    let mut depreciation = 0.0;
    let mut step = 1.0;
    while step <= period {
        depreciation = if step == 1.0 {
            cost * rate * month / 12.0
        } else if step == life.trunc() + 1.0 {
            (cost - total) * rate * (12.0 - month) / 12.0
        } else {
            (cost - total) * rate
        };
        total += depreciation;
        step += 1.0;
    }
    Value::Number(depreciation)
}

/// The **double declining balance** depreciation.
fn ddb(cost: f64, salvage: f64, life: f64, period: f64, factor: f64) -> Value {
    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || period <= 0.0 || factor <= 0.0 || period > life {
        return Value::Error("#NUM!".to_string());
    }
    let mut rate = factor / life;
    let old = if rate >= 1.0 {
        rate = 1.0;
        if period == 1.0 { cost } else { 0.0 }
    } else {
        cost * (1.0 - rate).powf(period - 1.0)
    };
    let new = cost * (1.0 - rate).powf(period);
    let depreciation = if new < salvage { old - salvage } else { old - new };
    Value::Number(depreciation.max(0.0))
}

/// The schedule of the declining balance, with the switch to the
/// straight line.
fn vdb_schedule(
    cost: f64,
    salvage: f64,
    life: f64,
    factor: f64,
    no_switch: bool,
    periods: usize,
) -> Vec<f64> {
    let mut out = Vec::new();
    let mut book = cost;
    let mut straight: Option<f64> = None;
    let mut index = 0.0;
    while index < periods as f64 {
        let depreciation = if let Some(straight) = straight {
            straight.min((book - salvage).max(0.0))
        } else {
            let declining = (book * factor / life).min((book - salvage).max(0.0));
            let remaining = life - index;
            let straight_line = if remaining > 0.0 { (book - salvage) / remaining.max(1.0) } else { 0.0 };
            if !no_switch && straight_line > declining {
                straight = Some(straight_line);
                straight_line.min((book - salvage).max(0.0))
            } else {
                declining
            }
        };
        out.push(depreciation);
        book -= depreciation;
        index += 1.0;
    }
    out
}

/// The declining balance over a **span** of periods.
fn vdb(cost: f64, salvage: f64, life: f64, start: f64, end: f64, factor: f64, no_switch: bool) -> Value {
    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || start < 0.0 || end < start || end > life || factor <= 0.0
    {
        return Value::Error("#NUM!".to_string());
    }
    let periods = end.ceil();
    if periods > 1.0e7 {
        return Value::Error("#NUM!".to_string());
    }
    let schedule = vdb_schedule(cost, salvage, life, factor, no_switch, periods as usize);
    let mut total = 0.0;
    for (index, depreciation) in schedule.iter().enumerate() {
        let (from, to) = (index as f64, index as f64 + 1.0);
        let overlap = (to.min(end) - from.max(start)).max(0.0);
        total += depreciation * overlap;
    }
    Value::Number(total)
}

impl Book {
    /// The **depreciation functions**.
    pub(crate) fn depreciation(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "SLN" => {
                let (cost, salvage, life) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(cost), Ok(salvage), Ok(life)) => (cost, salvage, life),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                sln(cost, salvage, life)
            }
            "SYD" => {
                let (cost, salvage, life, period) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(cost), Ok(salvage), Ok(life), Ok(period)) => (cost, salvage, life, period),
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                syd(cost, salvage, life, period)
            }
            "DB" => {
                let (cost, salvage, life, period) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(cost), Ok(salvage), Ok(life), Ok(period)) => (cost, salvage, life, period),
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let month = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(Some(month)) => month.trunc(),
                    Ok(None) => 12.0,
                    Err(message) => return Value::Error(message),
                };
                db(cost, salvage, life, period.trunc(), month)
            }
            "DDB" => {
                let (cost, salvage, life, period) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(cost), Ok(salvage), Ok(life), Ok(period)) => (cost, salvage, life, period),
                    (Err(message), _, _, _)
                    | (_, Err(message), _, _)
                    | (_, _, Err(message), _)
                    | (_, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let factor = match self.optional_number(sheet, arguments, 4, being) {
                    Ok(Some(factor)) => factor,
                    Ok(None) => 2.0,
                    Err(message) => return Value::Error(message),
                };
                ddb(cost, salvage, life, period, factor)
            }
            _ => {
                // `VDB`: the declining balance over a span of periods.
                let (cost, salvage, life, start, end) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                    self.number_at(sheet, arguments, 4, being),
                ) {
                    (Ok(cost), Ok(salvage), Ok(life), Ok(start), Ok(end)) => {
                        (cost, salvage, life, start, end)
                    }
                    (Err(message), _, _, _, _)
                    | (_, Err(message), _, _, _)
                    | (_, _, Err(message), _, _)
                    | (_, _, _, Err(message), _)
                    | (_, _, _, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let factor = match self.optional_number(sheet, arguments, 5, being) {
                    Ok(Some(factor)) => factor,
                    Ok(None) => 2.0,
                    Err(message) => return Value::Error(message),
                };
                let no_switch = match self.optional_number(sheet, arguments, 6, being) {
                    Ok(Some(number)) => number != 0.0,
                    Ok(None) => false,
                    Err(message) => return Value::Error(message),
                };
                vdb(cost, salvage, life, start, end, factor, no_switch)
            }
        }
    }
}
