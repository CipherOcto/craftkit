//! The **bases** and the **bits**.
//!
//! A formula may ask about a number written in another base, or about
//! the bits of a number:
//!
//! - `BIN2DEC`, `BIN2HEX`, `BIN2OCT`, `DEC2BIN`, `DEC2HEX`, `DEC2OCT`,
//!   `HEX2BIN`, `HEX2DEC`, `HEX2OCT`, `OCT2BIN`, `OCT2DEC`, `OCT2HEX`
//!   — the **bases**: two, eight and sixteen;
//! - `BITAND`, `BITOR`, `BITXOR`, `BITLSHIFT`, `BITRSHIFT` — the
//!   **bits**;
//! - `DELTA` — whether two numbers are the same;
//! - `GESTEP` — whether a number has reached a step.
//!
//! The rules are the **tree's**: a number written in a base is **ten
//! digits** long at most, a negative number is written in **two's
//! complement**, and a bit is **forty-eight** bits wide at most.

use crate::formula::{Book, CellRef, Expr, Value};
pub const NAMES: &[&str] = &[
    "BIN2DEC",
    "BIN2HEX",
    "BIN2OCT",
    "DEC2BIN",
    "DEC2HEX",
    "DEC2OCT",
    "HEX2BIN",
    "HEX2DEC",
    "HEX2OCT",
    "OCT2BIN",
    "OCT2DEC",
    "OCT2HEX",
    "BITAND",
    "BITOR",
    "BITXOR",
    "BITLSHIFT",
    "BITRSHIFT",
    "DELTA",
    "GESTEP",
];

/// The **bits** a base has.
fn bits_of(radix: u32) -> i64 {
    match radix {
        2 => 10,
        8 => 30,
        _ => 40,
    }
}

/// The text of a base-conversion argument: a number is written out as
/// a whole number.
fn base_source(value: &Value) -> Result<String, String> {
    match value {
        Value::Error(message) => Err(message.clone()),
        Value::Bool(_) => Err("#VALUE!".to_string()),
        Value::Number(number) => {
            if *number < 0.0 || *number != number.trunc() {
                return Err("#NUM!".to_string());
            }
            Ok(format!("{}", *number as i64))
        }
        Value::Empty => Ok(String::new()),
        Value::Text(text) => Ok(text.trim().to_string()),
    }
}

/// A number written in a base, as a ten-digit two's complement.
fn parse_base(text: &str, radix: u32) -> Result<i64, String> {
    if text.len() > 10 {
        return Err("#NUM!".to_string());
    }
    if text.is_empty() {
        return Ok(0);
    }
    let Ok(value) = i64::from_str_radix(text, radix) else {
        return Err("#NUM!".to_string());
    };
    if text.starts_with(['+', '-']) {
        return Err("#NUM!".to_string());
    }
    let bits = bits_of(radix);
    // A full number with the top bit set is a negative number.
    if text.len() == 10 && value >= 1i64 << (bits - 1) { Ok(value - (1i64 << bits)) } else { Ok(value) }
}

/// A number written in a base.
fn to_radix(mut value: i64, radix: u32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut digits: Vec<char> = Vec::new();
    while value > 0 {
        let digit = (value % i64::from(radix)) as u32;
        digits.push(char::from_digit(digit, radix).unwrap_or('0').to_ascii_uppercase());
        value /= i64::from(radix);
    }
    digits.iter().rev().collect()
}

/// A number written in a base, padded to a number of places.
fn format_base(value: i64, radix: u32, places: Option<f64>) -> Result<Value, String> {
    let bits = bits_of(radix);
    let least = -(1i64 << (bits - 1));
    let most = (1i64 << (bits - 1)) - 1;
    if value < least || value > most {
        return Err("#NUM!".to_string());
    }
    if value < 0 {
        let unsigned = value + (1i64 << bits);
        return Ok(Value::Text(to_radix(unsigned, radix)));
    }
    let text = to_radix(value, radix);
    let Some(places) = places else {
        return Ok(Value::Text(text));
    };
    let places = places.trunc();
    if !(1.0..=10.0).contains(&places) {
        return Err("#NUM!".to_string());
    }
    let places = places as usize;
    if text.len() > places {
        return Err("#NUM!".to_string());
    }
    Ok(Value::Text(format!("{}{}", "0".repeat(places - text.len()), text)))
}

/// The **bit** a number is: forty-eight bits at most.
fn bit_operand(number: f64) -> Result<u64, String> {
    const LIMIT: f64 = 281_474_976_710_656.0;
    if !(0.0..LIMIT).contains(&number) || number != number.trunc() {
        return Err("#NUM!".to_string());
    }
    Ok(number as u64)
}

/// The **shift** of a number by a number of bits.
fn shift(number: u64, places: f64, left: bool) -> Result<f64, String> {
    const LIMIT: f64 = 281_474_976_710_656.0;
    let places = places.trunc();
    if places.abs() > 53.0 {
        return Err("#NUM!".to_string());
    }
    let places = if left { places as i64 } else { -(places as i64) };
    if places >= 0 {
        let shifted = (u128::from(number)) << places;
        if shifted >= LIMIT as u128 {
            return Err("#NUM!".to_string());
        }
        Ok(shifted as f64)
    } else {
        Ok((number >> (-places)) as f64)
    }
}

impl Book {
    /// The **engineering functions**.
    pub(crate) fn engineering(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "DELTA" | "GESTEP" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let second = match self.optional_number(sheet, arguments, 1, being) {
                    Ok(Some(second)) => second,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                let same = if name == "DELTA" { first == second } else { first >= second };
                Value::Number(if same { 1.0 } else { 0.0 })
            }
            "BITAND" | "BITOR" | "BITXOR" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let second = match self.number_at(sheet, arguments, 1, being) {
                    Ok(second) => second,
                    Err(message) => return Value::Error(message),
                };
                let (Ok(first), Ok(second)) = (bit_operand(first), bit_operand(second)) else {
                    return Value::Error("#NUM!".to_string());
                };
                let result = match name {
                    "BITAND" => first & second,
                    "BITOR" => first | second,
                    _ => first ^ second,
                };
                Value::Number(result as f64)
            }
            "BITLSHIFT" | "BITRSHIFT" => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                let second = match self.number_at(sheet, arguments, 1, being) {
                    Ok(second) => second,
                    Err(message) => return Value::Error(message),
                };
                let Ok(first) = bit_operand(first) else {
                    return Value::Error("#NUM!".to_string());
                };
                match shift(first, second, name == "BITLSHIFT") {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                }
            }
            _ => {
                // The bases: two, eight and sixteen.
                let (from, to) = match name {
                    "BIN2DEC" => (2, 10),
                    "BIN2HEX" => (2, 16),
                    "BIN2OCT" => (2, 8),
                    "HEX2BIN" => (16, 2),
                    "HEX2DEC" => (16, 10),
                    "HEX2OCT" => (16, 8),
                    "OCT2BIN" => (8, 2),
                    "OCT2DEC" => (8, 10),
                    "OCT2HEX" => (8, 16),
                    "DEC2BIN" => (10, 2),
                    "DEC2HEX" => (10, 16),
                    _ => (10, 8),
                };
                let source = match arguments.first() {
                    Some(first) => {
                        let source = self.evaluate(sheet, first, being);
                        if let Value::Error(message) = source {
                            return Value::Error(message);
                        }
                        source
                    }
                    None => Value::Empty,
                };
                let places = match self.optional_number(sheet, arguments, 1, being) {
                    Ok(Some(places)) => Some(places),
                    Ok(None) => None,
                    Err(message) => return Value::Error(message),
                };
                if from == 10 {
                    // From ten: the number itself, cut to a whole one.
                    let number = match source {
                        Value::Number(number) => number,
                        Value::Empty => 0.0,
                        Value::Error(message) => return Value::Error(message),
                        _ => return Value::Error("#VALUE!".to_string()),
                    }
                    .trunc();
                    if number.abs() > 1.0e15 {
                        return Value::Error("#NUM!".to_string());
                    }
                    return match format_base(number as i64, to, places) {
                        Ok(value) => value,
                        Err(message) => Value::Error(message),
                    };
                }
                let text = match base_source(&source) {
                    Ok(text) => text,
                    Err(message) => return Value::Error(message),
                };
                let Ok(value) = parse_base(&text, from) else {
                    return Value::Error("#NUM!".to_string());
                };
                if to == 10 {
                    return Value::Number(value as f64);
                }
                match format_base(value, to, places) {
                    Ok(value) => value,
                    Err(message) => Value::Error(message),
                }
            }
        }
    }
}
