//! The **shaping** of a text.
//!
//! A formula may shape a text: take a part of it, count it, fold it
//! or clean it. The tree's functions are here:
//!
//! - `LEFT(text, [count])` — the **first** characters of the text,
//!   and the first one when the count is not given;
//! - `RIGHT(text, [count])` — the **last** characters of the text;
//! - `MID(text, start, [count])` — the characters from the **start**,
//!   and the start is counted from one;
//! - `LEN(text)` — how many characters the text holds;
//! - `TRIM(text)` — the text without the spaces that are not between
//!   words;
//! - `UPPER(text)` and `LOWER(text)` — the text in one case or the
//!   other;
//! - `PROPER(text)` — the text with every **word** capitalised.
//!
//! The text of a value is the text it is **written** as: a number is
//! written as a number, a boolean as `TRUE` or `FALSE`, and an empty
//! value as nothing at all. And a value that is an **error** is that
//! error: the shaping of an error is the error.

use crate::formula::{Book, Expr, Value};

/// Whether a name is one of the **text functions**.
pub fn is_text_function(name: &str) -> bool {
    matches!(name, "LEFT" | "RIGHT" | "MID" | "LEN" | "TRIM" | "UPPER" | "LOWER" | "PROPER")
}

/// The **text** of a value: what it is written as.
///
/// An error is an error: the text of an error is not a text.
pub fn value_text(value: &Value) -> Result<String, String> {
    match value {
        Value::Error(message) => Err(message.clone()),
        other => Ok(display_text(other)),
    }
}

/// The **text** of a value, as a cell shows it.
fn display_text(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Error(message) => message.clone(),
    }
}

/// The **count** an argument holds: a whole number of characters.
///
/// A count that is negative is an error, and a count that is not a
/// number is an error. A count that is empty is zero — the tree's
/// rule: an argument that is there and holds nothing is zero.
pub fn count_argument(value: &Value) -> Result<usize, String> {
    let number = match value {
        Value::Empty => 0.0,
        other => other.as_number().ok_or_else(|| "#VALUE!".to_string())?,
    };
    if number.is_nan() || number < 0.0 {
        return Err("#VALUE!".to_string());
    }
    Ok(if number > 1e9 { 1_000_000_000 } else { number.trunc() as usize })
}

/// The **word** of a text: every word capitalised and the rest of it
/// in small letters.
///
/// A letter that is not in a word starts a new one, and so does
/// anything that is not a letter.
fn proper(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_word = false;
    for character in text.chars() {
        if character.is_alphabetic() {
            if in_word {
                out.extend(character.to_lowercase());
            } else {
                out.extend(character.to_uppercase());
            }
            in_word = true;
        } else {
            out.push(character);
            in_word = false;
        }
    }
    out
}

impl Book {
    /// The **text functions**: the shaping of a text.
    pub(crate) fn shaping(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Value {
        if arguments.is_empty() {
            return Value::Error("#VALUE!".to_string());
        }
        let first = self.evaluate(sheet, &arguments[0], being);
        let text = match value_text(&first) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        match name {
            "LEFT" => match self.count_at(sheet, arguments, 1, 1.0, being) {
                Ok(count) => Value::Text(text.chars().take(count).collect()),
                Err(message) => Value::Error(message),
            },
            "RIGHT" => match self.count_at(sheet, arguments, 1, 1.0, being) {
                Ok(count) => {
                    let characters: Vec<char> = text.chars().collect();
                    let start = characters.len().saturating_sub(count);
                    Value::Text(characters[start..].iter().collect())
                }
                Err(message) => Value::Error(message),
            },
            "MID" => {
                let start = match arguments.get(1) {
                    Some(expression) => self.evaluate(sheet, expression, being).as_number(),
                    None => None,
                };

                let Some(start) = start else {
                    return Value::Error("#VALUE!".to_string());
                };
                if start.is_nan() || start < 1.0 {
                    return Value::Error("#VALUE!".to_string());
                }
                let start = if start > 1e9 { 1_000_000_000 } else { start.trunc() as usize };
                match self.count_at(sheet, arguments, 2, 0.0, being) {
                    Ok(count) => Value::Text(text.chars().skip(start - 1).take(count).collect()),
                    Err(message) => Value::Error(message),
                }
            }
            "LEN" => Value::Number(text.chars().count() as f64),
            "TRIM" => {
                Value::Text(text.split(' ').filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" "))
            }
            "UPPER" => Value::Text(text.to_uppercase()),
            "LOWER" => Value::Text(text.to_lowercase()),
            "PROPER" => Value::Text(proper(&text)),
            _ => Value::Error(format!("there is no function `{name}`")),
        }
    }

    /// The **count** an argument holds: how many characters.
    fn count_at(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        default: f64,
        being: &mut Vec<(String, crate::formula::CellRef)>,
    ) -> Result<usize, String> {
        match arguments.get(index) {
            None => Ok(default.trunc() as usize),
            Some(expression) => {
                let value = self.evaluate(sheet, expression, being);
                count_argument(&value)
            }
        }
    }
}
