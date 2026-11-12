//! The **logic** of a formula.
//!
//! A formula may ask about the truth of things and may choose between
//! them:
//!
//! - `AND(logical, ...)` — every one of them is true;
//! - `OR(logical, ...)` — at least one of them is true;
//! - `XOR(logical, ...)` — an odd number of them are true;
//! - `NOT(logical)` — the reverse of one of them;
//! - `IFS(condition, value, ...)` — the value paired with the first
//!   condition that is true;
//! - `IFERROR(value, fallback)` — the fallback when the value is an
//!   error;
//! - `IFNA(value, fallback)` — the fallback when the value is
//!   `#N/A`;
//! - `SWITCH(expression, value, result, ...)` — the result paired
//!   with the first value that matches the expression;
//! - `CHOOSE(index, value, ...)` — the value at that place.
//!
//! The rules are the **tree's**: a **range** contributes its numbers
//! and its booleans and skips its texts and its empties, a **value**
//! that is written out is read as a truth, and a text that is not
//! `TRUE` or `FALSE` is a value error.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **logical functions**.
pub fn is_logical_function(name: &str) -> bool {
    matches!(name, "AND" | "OR" | "XOR" | "NOT" | "IFS" | "IFERROR" | "IFNA" | "SWITCH" | "CHOOSE")
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

/// Two values are the **same**, the tree's way.
fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Text(left), Value::Text(right)) => left.trim().eq_ignore_ascii_case(right.trim()),
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Empty, Value::Empty) => true,
        (Value::Empty, Value::Number(number)) | (Value::Number(number), Value::Empty) => *number == 0.0,
        (Value::Empty, Value::Text(text)) | (Value::Text(text), Value::Empty) => text.is_empty(),
        _ => false,
    }
}

impl Book {
    /// The **truths** an argument holds.
    ///
    /// A **range** contributes its numbers and its booleans and skips
    /// its texts and its empties; a **value** written out is read as a
    /// truth.
    fn truths(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<Vec<bool>, String> {
        let mut truths = Vec::new();
        match argument {
            Expr::Range { .. } | Expr::Name(_) => {
                let (name, area) = self.area_of(sheet, argument)?;
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                        match value {
                            Value::Number(number) => truths.push(number != 0.0),
                            Value::Bool(value) => truths.push(value),
                            Value::Error(message) => return Err(message),
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                let value = self.evaluate(sheet, argument, being);
                truths.push(as_bool(&value)?);
            }
        }
        Ok(truths)
    }

    /// The **logical functions**.
    pub(crate) fn logic(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "NOT" => {
                let Some(argument) = arguments.first() else {
                    return Value::Error("#VALUE!".to_string());
                };
                let value = self.evaluate(sheet, argument, being);
                match as_bool(&value) {
                    Ok(truth) => Value::Bool(!truth),
                    Err(message) => Value::Error(message),
                }
            }
            "IFERROR" | "IFNA" => {
                if arguments.len() != 2 {
                    return Value::Error("#VALUE!".to_string());
                }
                let value = self.evaluate(sheet, &arguments[0], being);
                let is_error = match &value {
                    Value::Error(message) => {
                        if name == "IFNA" && message != "#N/A" {
                            return Value::Error(message.clone());
                        }
                        true
                    }
                    _ => false,
                };
                if is_error { self.evaluate(sheet, &arguments[1], being) } else { value }
            }
            "CHOOSE" => {
                if arguments.len() < 2 {
                    return Value::Error("#VALUE!".to_string());
                }
                let index = match self.number_at(sheet, arguments, 0, being) {
                    Ok(index) => index.trunc(),
                    Err(message) => return Value::Error(message),
                };
                if index < 1.0 || index >= arguments.len() as f64 {
                    return Value::Error("#VALUE!".to_string());
                }
                self.evaluate(sheet, &arguments[index as usize], being)
            }
            "IFS" => {
                if !arguments.len().is_multiple_of(2) {
                    return Value::Error("#N/A".to_string());
                }
                for pair in arguments.chunks(2) {
                    let condition = self.evaluate(sheet, &pair[0], being);
                    match as_bool(&condition) {
                        Ok(true) => return self.evaluate(sheet, &pair[1], being),
                        Ok(false) => {}
                        Err(message) => return Value::Error(message),
                    }
                }
                Value::Error("#N/A".to_string())
            }
            "SWITCH" => {
                if arguments.len() < 3 {
                    return Value::Error("#VALUE!".to_string());
                }
                let expression = self.evaluate(sheet, &arguments[0], being);
                if let Value::Error(message) = expression {
                    return Value::Error(message);
                }
                let rest = &arguments[1..];
                for pair in rest.chunks(2) {
                    if pair.len() == 1 {
                        // The last one alone is the **default**.
                        return self.evaluate(sheet, &pair[0], being);
                    }
                    let case = self.evaluate(sheet, &pair[0], being);
                    if let Value::Error(message) = case {
                        return Value::Error(message);
                    }
                    if same_value(&expression, &case) {
                        return self.evaluate(sheet, &pair[1], being);
                    }
                }
                Value::Error("#N/A".to_string())
            }
            _ => {
                // `AND`, `OR` and `XOR`.
                let mut truths = Vec::new();
                for argument in arguments {
                    match self.truths(sheet, argument, being) {
                        Ok(found) => truths.extend(found),
                        Err(message) => return Value::Error(message),
                    }
                }
                if truths.is_empty() {
                    return Value::Error("#VALUE!".to_string());
                }
                let answer = match name {
                    "AND" => truths.iter().all(|truth| *truth),
                    "OR" => truths.iter().any(|truth| *truth),
                    _ => truths.iter().filter(|truth| **truth).count() % 2 == 1,
                };
                Value::Bool(answer)
            }
        }
    }
}
