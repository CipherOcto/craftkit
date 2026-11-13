//! The **information** of a value.
//!
//! A formula may ask what a value is:
//!
//! - `ISBLANK(value)` — whether it is **nothing**;
//! - `ISERR(value)` — whether it is an **error** and not `#N/A`;
//! - `ISERROR(value)` — whether it is an error at all;
//! - `ISNA(value)` — whether it is `#N/A`;
//! - `ISNUMBER(value)` — whether it is a number;
//! - `ISTEXT(value)` — whether it is text;
//! - `ISNONTEXT(value)` — whether it is not text;
//! - `ISLOGICAL(value)` — whether it is a **boolean**;
//! - `ISEVEN(value)` and `ISODD(value)` — whether a number is even or
//!   odd;
//! - `TYPE(value)` — the **kind** of a value: one for a number, two
//!   for text, four for a boolean and sixteen for an error;
//! - `ERROR.TYPE(value)` — the **number** that names an error;
//! - `NA()` — the error `#N/A` itself;
//! - `N(value)` — a value as a number: a number stays, a boolean
//!   becomes one or nothing, and anything else becomes nothing.
//!
//! The rules are the **tree's**: an argument that is **nothing** is
//! not text and not an error, and a value that is not a number cannot
//! be even or odd.

use crate::formula::{Book, CellRef, Expr, Value};
pub const NAMES: &[&str] = &[
    "ISBLANK",
    "ISERR",
    "ISERROR",
    "ISNA",
    "ISNUMBER",
    "ISTEXT",
    "ISNONTEXT",
    "ISLOGICAL",
    "ISEVEN",
    "ISODD",
    "TYPE",
    "ERROR.TYPE",
    "NA",
    "N",
];

/// The **number** that names an error.
fn code_of(message: &str) -> Option<f64> {
    Some(match message {
        "#NULL!" => 1.0,
        "#DIV/0!" => 2.0,
        "#VALUE!" => 3.0,
        "#REF!" => 4.0,
        "#NAME?" => 5.0,
        "#NUM!" => 6.0,
        "#N/A" => 7.0,
        "#GETTING_DATA" => 8.0,
        "#SPILL!" => 9.0,
        "#CALC!" => 14.0,
        "#CIRC!" => 15.0,
        _ => return None,
    })
}

impl Book {
    /// The **value** of an argument: a range reads its **first** cell,
    /// as the tree's own does.
    fn information_value(&self, sheet: &str, argument: &Expr, being: &mut Vec<(String, CellRef)>) -> Value {
        match argument {
            Expr::Range { sheet: named, start, .. } => {
                self.evaluate(sheet, &Expr::Cell { sheet: named.clone(), anchor: *start }, being)
            }
            other => self.evaluate(sheet, other, being),
        }
    }

    /// The **information functions**.
    pub(crate) fn information(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if name == "NA" {
            return Value::Error("#N/A".to_string());
        }
        let Some(argument) = arguments.first() else {
            // A missing argument is nothing.
            return match name {
                "TYPE" => Value::Error("#VALUE!".to_string()),
                "N" => Value::Number(0.0),
                "ISBLANK" => Value::Bool(true),
                "ISERR" | "ISERROR" | "ISNA" => Value::Bool(false),
                "ISNUMBER" => Value::Bool(false),
                "ISTEXT" => Value::Bool(false),
                "ISNONTEXT" => Value::Bool(true),
                "ISLOGICAL" => Value::Bool(false),
                "ISEVEN" => Value::Bool(true),
                "ISODD" => Value::Bool(false),
                _ => Value::Error("#VALUE!".to_string()),
            };
        };
        let value = self.information_value(sheet, argument, being);
        match name {
            "ISBLANK" => Value::Bool(matches!(value, Value::Empty)),
            "ISERR" => Value::Bool(matches!(&value, Value::Error(message) if message != "#N/A")),
            "ISERROR" => Value::Bool(matches!(value, Value::Error(_))),
            "ISNA" => Value::Bool(matches!(&value, Value::Error(message) if message == "#N/A")),
            "ISNUMBER" => Value::Bool(matches!(value, Value::Number(_))),
            "ISTEXT" => Value::Bool(matches!(value, Value::Text(_))),
            "ISNONTEXT" => Value::Bool(!matches!(value, Value::Text(_))),
            "ISLOGICAL" => Value::Bool(matches!(value, Value::Bool(_))),
            "TYPE" => Value::Number(match value {
                Value::Empty | Value::Number(_) => 1.0,
                Value::Text(_) => 2.0,
                Value::Bool(_) => 4.0,
                Value::Error(_) => 16.0,
            }),
            "ERROR.TYPE" => match &value {
                Value::Error(message) => match code_of(message) {
                    Some(code) => Value::Number(code),
                    None => Value::Error("#N/A".to_string()),
                },
                _ => Value::Error("#N/A".to_string()),
            },
            "N" => match value {
                Value::Number(number) => Value::Number(number),
                Value::Bool(value) => Value::Number(if value { 1.0 } else { 0.0 }),
                Value::Error(message) => Value::Error(message),
                _ => Value::Number(0.0),
            },
            _ => {
                // `ISEVEN` and `ISODD`: the parity of a number.
                let number = match value {
                    Value::Number(number) => number,
                    Value::Empty => 0.0,
                    Value::Bool(_) => return Value::Error("#VALUE!".to_string()),
                    Value::Error(message) => return Value::Error(message),
                    Value::Text(_) => return Value::Error("#VALUE!".to_string()),
                };
                if number.abs() > 9.007_199_254_740_992e15 {
                    return Value::Error("#NUM!".to_string());
                }
                let parity = number.trunc().abs() % 2.0;
                let even = parity == 0.0;
                Value::Bool(if name == "ISEVEN" { even } else { !even })
            }
        }
    }
}
