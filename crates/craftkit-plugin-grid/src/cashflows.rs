//! The **cash flows** of a plan.
//!
//! A formula may ask about a series of payments:
//!
//! - `IRR(values, [guess])` — the **rate** at which the series of
//!   payments is worth nothing;
//! - `MIRR(values, finance_rate, reinvest_rate)` — the same rate,
//!   with the payments put out and taken in at two different rates;
//! - `XIRR(values, dates, [guess])` — the same rate, with a **date**
//!   on every payment;
//! - `XNPV(rate, values, dates)` — what the series is worth today;
//! - `FVSCHEDULE(principal, schedule)` — what a sum grows to through
//!   a schedule of rates;
//! - `CUMIPMT(rate, nper, pv, start, end, type)` — the **interest**
//!   of a run of payments;
//! - `CUMPRINC(rate, nper, pv, start, end, type)` — the **principal**
//!   of the same run;
//! - `DOLLARDE(fractional_dollar, fraction)` and `DOLLARFR` — a
//!   price written as a **fraction** of a dollar and back.
//!
//! The rules are the **tree's**: a series with no payment one way and
//! no payment the other way has no rate, a rate of minus one or less
//! is no rate, and the **type** is the tree's own — any number that is
//! not zero means the payment comes at the beginning of the period.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **cash flow functions**.
pub fn is_cash_flow_function(name: &str) -> bool {
    matches!(
        name,
        "IRR"
            | "MIRR"
            | "XIRR"
            | "XNPV"
            | "NPV"
            | "FVSCHEDULE"
            | "CUMIPMT"
            | "CUMPRINC"
            | "DOLLARDE"
            | "DOLLARFR"
    )
}

/// The **present value** of a series of dated payments.
fn xnpv_of(rate: f64, values: &[f64], dates: &[f64]) -> f64 {
    let first = dates.first().copied().unwrap_or(0.0);
    values.iter().zip(dates).map(|(value, date)| value / (1.0 + rate).powf((date - first) / 365.0)).sum()
}

/// The **digits** a fraction of a dollar is written in.
fn dollar_digits(fraction: f64) -> Result<f64, String> {
    if fraction < 0.0 {
        return Err("#NUM!".to_string());
    }
    if fraction < 1.0 {
        return Err("#DIV/0!".to_string());
    }
    Ok(10f64.powf(fraction.log10().ceil()))
}

impl Book {
    /// The **values** and the **dates** of a series of payments: the
    /// two must be of the same length, and every one of them a number.
    fn dated_flows(
        &self,
        sheet: &str,
        arguments: &[Expr],
        values_at: usize,
        dates_at: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<(Vec<f64>, Vec<f64>), String> {
        let Some(values_argument) = arguments.get(values_at) else {
            return Err("#VALUE!".to_string());
        };
        let Some(dates_argument) = arguments.get(dates_at) else {
            return Err("#VALUE!".to_string());
        };
        let values = self.values_of_cells(sheet, values_argument, being)?;
        let dates = self.values_of_cells(sheet, dates_argument, being)?;
        if values.len() != dates.len() {
            return Err("#NUM!".to_string());
        }
        let mut numbers = Vec::with_capacity(values.len());
        let mut days = Vec::with_capacity(dates.len());
        for (value, date) in values.iter().zip(dates.iter()) {
            match value {
                Value::Number(number) => numbers.push(*number),
                Value::Error(message) => return Err(message.clone()),
                _ => return Err("#VALUE!".to_string()),
            }
            match date {
                Value::Number(number) => days.push(number.trunc()),
                Value::Error(message) => return Err(message.clone()),
                _ => return Err("#VALUE!".to_string()),
            }
        }
        let Some(first) = days.first().copied() else {
            return Err("#NUM!".to_string());
        };
        if days.iter().any(|day| *day < first || *day < 0.0) {
            return Err("#NUM!".to_string());
        }
        Ok((numbers, days))
    }

    /// The **flows** of a series: the numbers of the cells of a range,
    /// and the value of a plain argument.
    ///
    /// The tree's rule: the text of a **cell** is not a flow and is
    /// not counted, and the text of a plain argument is the number it
    /// writes — and a text that is not a number is an error.
    fn flows_of(
        &self,
        sheet: &str,
        arguments: &[Expr],
        from: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<Vec<f64>, String> {
        let mut flows = Vec::new();
        for argument in arguments.iter().skip(from) {
            match argument {
                Expr::Range { .. } | Expr::Name(_) => {
                    let (name, area) = self.area_of(sheet, argument)?;
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                            match value {
                                Value::Number(number) => flows.push(number),
                                Value::Error(message) => return Err(message),
                                _ => {}
                            }
                        }
                    }
                }
                other => {
                    let value = self.evaluate(sheet, other, being);
                    match value {
                        Value::Number(number) => flows.push(number),
                        Value::Bool(flag) => flows.push(if flag { 1.0 } else { 0.0 }),
                        Value::Text(text) => match crate::criteria::number_text(&text) {
                            Some(number) => flows.push(number),
                            None => return Err("#VALUE!".to_string()),
                        },
                        Value::Error(message) => return Err(message),
                        Value::Empty => {}
                    }
                }
            }
        }
        Ok(flows)
    }

    /// The **cash flow functions**.
    pub(crate) fn cash_flows(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "NPV" => {
                let rate = match self.number_at(sheet, arguments, 0, being) {
                    Ok(rate) => rate,
                    Err(message) => return Value::Error(message),
                };
                if rate == -1.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let flows = match self.flows_of(sheet, arguments, 1, being) {
                    Ok(flows) => flows,
                    Err(message) => return Value::Error(message),
                };
                // The tree's rule: the **first** flow is discounted once,
                // and the second twice.
                let mut sum = 0.0;
                let mut divisor = 1.0;
                for flow in flows {
                    divisor *= 1.0 + rate;
                    sum += flow / divisor;
                }
                Value::Number(sum)
            }
            "XNPV" => {
                let rate = match self.number_at(sheet, arguments, 0, being) {
                    Ok(rate) => rate,
                    Err(message) => return Value::Error(message),
                };
                if rate <= -1.0 {
                    return Value::Error("#NUM!".to_string());
                }
                let (values, dates) = match self.dated_flows(sheet, arguments, 1, 2, being) {
                    Ok(flows) => flows,
                    Err(message) => return Value::Error(message),
                };
                Value::Number(xnpv_of(rate, &values, &dates))
            }
            "XIRR" => {
                let (values, dates) = match self.dated_flows(sheet, arguments, 0, 1, being) {
                    Ok(flows) => flows,
                    Err(message) => return Value::Error(message),
                };
                if !values.iter().any(|value| *value > 0.0) || !values.iter().any(|value| *value < 0.0) {
                    return Value::Error("#NUM!".to_string());
                }
                let guess = match self.optional_number(sheet, arguments, 2, being) {
                    Ok(Some(guess)) => guess,
                    Ok(None) => 0.1,
                    Err(message) => return Value::Error(message),
                };
                match crate::finance::solve_rate(|rate| xnpv_of(rate, &values, &dates), guess) {
                    Ok(rate) => Value::Number(rate),
                    Err(message) => Value::Error(message),
                }
            }
            "IRR" => {
                let values = match self.numbers_of(sheet, arguments, 0, being) {
                    Ok(values) => values,
                    Err(message) => return Value::Error(message),
                };
                if !values.iter().any(|value| *value > 0.0) || !values.iter().any(|value| *value < 0.0) {
                    return Value::Error("#NUM!".to_string());
                }
                let guess = match self.optional_number(sheet, arguments, 1, being) {
                    Ok(Some(guess)) => guess,
                    Ok(None) => 0.1,
                    Err(message) => return Value::Error(message),
                };
                let worth = |rate: f64| {
                    let mut sum = 0.0;
                    let mut divisor = 1.0;
                    for value in &values {
                        sum += value / divisor;
                        divisor *= 1.0 + rate;
                    }
                    sum
                };
                match crate::finance::solve_rate(worth, guess) {
                    Ok(rate) => Value::Number(rate),
                    Err(message) => Value::Error(message),
                }
            }
            "MIRR" => {
                let values = match self.numbers_of(sheet, arguments, 0, being) {
                    Ok(values) => values,
                    Err(message) => return Value::Error(message),
                };
                let finance_rate = match self.number_at(sheet, arguments, 1, being) {
                    Ok(rate) => rate,
                    Err(message) => return Value::Error(message),
                };
                let reinvest_rate = match self.number_at(sheet, arguments, 2, being) {
                    Ok(rate) => rate,
                    Err(message) => return Value::Error(message),
                };
                let count = values.len() as f64;
                if values.len() < 2 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let mut positive = 0.0;
                let mut negative = 0.0;
                for (index, value) in values.iter().enumerate() {
                    if *value > 0.0 {
                        positive += value / (1.0 + reinvest_rate).powi(index as i32);
                    } else if *value < 0.0 {
                        negative += value / (1.0 + finance_rate).powi(index as i32);
                    }
                }
                if negative == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let rate = (-positive * (1.0 + reinvest_rate).powf(count - 1.0) / negative)
                    .powf(1.0 / (count - 1.0))
                    - 1.0;
                Value::Number(rate)
            }
            "FVSCHEDULE" => {
                let mut principal = match self.number_at(sheet, arguments, 0, being) {
                    Ok(principal) => principal,
                    Err(message) => return Value::Error(message),
                };
                let schedule = match self.numbers_of(sheet, arguments, 1, being) {
                    Ok(schedule) => schedule,
                    Err(message) => return Value::Error(message),
                };
                for rate in schedule {
                    principal *= 1.0 + rate;
                }
                Value::Number(principal)
            }
            "CUMIPMT" | "CUMPRINC" => {
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
                let (start, end) = match (
                    self.number_at(sheet, arguments, 3, being),
                    self.number_at(sheet, arguments, 4, being),
                ) {
                    (Ok(start), Ok(end)) => (start.trunc(), end.trunc()),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let typ = match self.number_at(sheet, arguments, 5, being) {
                    Ok(typ) => typ,
                    Err(message) => return Value::Error(message),
                };
                let periods = nper.trunc();
                if rate <= 0.0
                    || periods <= 0.0
                    || pv <= 0.0
                    || start < 1.0
                    || end < start
                    || end > periods
                    || (typ != 0.0 && typ != 1.0)
                {
                    return Value::Error("#NUM!".to_string());
                }
                if end - start > 1e7 || periods > 1e15 {
                    return Value::Error("#NUM!".to_string());
                }
                let payment = crate::finance::pmt_of(rate, periods, pv, 0.0, typ);
                let mut sum = 0.0;
                let mut period = start;
                while period <= end {
                    let interest = crate::finance::ipmt_of(rate, period, periods, pv, 0.0, typ);
                    sum += if name == "CUMPRINC" { payment - interest } else { interest };
                    period += 1.0;
                }
                Value::Number(sum)
            }
            _ => {
                // `DOLLARDE` and `DOLLARFR`.
                let (value, fraction) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                ) {
                    (Ok(value), Ok(fraction)) => (value, fraction.trunc()),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let digits = match dollar_digits(fraction) {
                    Ok(digits) => digits,
                    Err(message) => return Value::Error(message),
                };
                let whole = value.trunc();
                if name == "DOLLARDE" {
                    Value::Number(whole + (value - whole) * digits / fraction)
                } else {
                    Value::Number(whole + (value - whole) * fraction / digits)
                }
            }
        }
    }
}
