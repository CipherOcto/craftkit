//! The **arithmetic** of a formula.
//!
//! A formula may do arithmetic on a number: round it, cut it, take
//! the rest of a division, take its root, raise it to a power, and
//! multiply or add up a set of numbers.
//!
//! The rules are the **tree's**, and they are careful:
//!
//! - `ROUND` rounds **halves away from zero**, and it does so on the
//!   number written with fifteen significant digits — so `2.675` rounds
//!   to `2.68` and not to `2.67`;
//! - `ROUNDUP` rounds away from zero and `ROUNDDOWN` toward zero, and
//!   `TRUNC` is the same as `ROUNDDOWN`;
//! - `MOD` is the rest of a division and it keeps the **sign of the
//!   divisor**, so `MOD(-3,2)` is `1`;
//! - `POWER` of zero to a negative power is `#DIV/0!` and zero to the
//!   zeroth is `#NUM!`;
//! - `SQRT` of a negative number is `#NUM!`, and so is `LN` of a
//!   number that is not positive;
//! - `GCD` and `LCM` are the **integers** of the numbers, and a
//!   number that is not one is `#NUM!`.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **arithmetic functions**.
pub fn is_arithmetic_function(name: &str) -> bool {
    matches!(
        name,
        "ROUND"
            | "ROUNDUP"
            | "ROUNDDOWN"
            | "TRUNC"
            | "INT"
            | "MROUND"
            | "CEILING"
            | "FLOOR"
            | "MOD"
            | "QUOTIENT"
            | "ABS"
            | "SIGN"
            | "SQRT"
            | "POWER"
            | "EXP"
            | "LN"
            | "LOG"
            | "LOG10"
            | "PI"
            | "PRODUCT"
            | "SUMSQ"
            | "GCD"
            | "LCM"
            | "EVEN"
            | "ODD"
            | "SQRTPI"
    )
}

/// The number with **fifteen significant digits**: the binary noise
/// is taken off, so that `0.1 + 0.2` is `0.3`.
fn clean(number: f64) -> f64 {
    if !number.is_finite() || number == 0.0 {
        return number;
    }
    format!("{:.14e}", number).parse().unwrap_or(number)
}

/// A number rounded to a number of digits, **halves away from zero**.
///
/// The tree's rule: the number is written with fifteen significant
/// digits first, and the rounding is done on those digits, so that
/// `2.675` rounds to `2.68` and not to `2.67`.
pub(crate) fn round_half_away(number: f64, digits: i32) -> f64 {
    if !number.is_finite() || number == 0.0 {
        return number;
    }
    let digits = digits.clamp(-400, 400) as i64;
    let written = format!("{:.14e}", number.abs());
    let Some((mantissa, exponent)) = written.split_once('e') else { return number };
    let Ok(exponent) = exponent.parse::<i64>() else { return number };
    let mut digits_of: Vec<u8> =
        mantissa.bytes().filter(u8::is_ascii_digit).map(|byte| byte - b'0').collect();
    let keep = exponent + 1 + digits;
    if keep >= digits_of.len() as i64 {
        // Nothing to round off, but only fifteen significant digits
        // are kept.
        return clean(number);
    }
    if keep < 0 {
        return 0.0;
    }
    let keep = keep as usize;
    let round_up = digits_of.get(keep).is_some_and(|digit| *digit >= 5);
    digits_of.truncate(keep);
    let mut power = exponent + 1;
    if round_up {
        let mut index = keep;
        loop {
            if index == 0 {
                // The carry runs off the front: the number becomes one
                // and the zeroes, and it is one place larger.
                digits_of.insert(0, 1);
                power += 1;
                break;
            }
            index -= 1;
            if digits_of[index] == 9 {
                digits_of[index] = 0;
            } else {
                digits_of[index] += 1;
                break;
            }
        }
    }
    if digits_of.is_empty() {
        return 0.0;
    }
    // The number is **written** again and read back, so that the
    // answer is the nearest double to the decimal number and not the
    // product of two doubles.
    let written: String = digits_of.iter().map(|digit| char::from(b'0' + digit)).collect();
    let text = format!("0.{written}e{power}");
    let value: f64 = text.parse().unwrap_or(number);
    if number < 0.0 { -value } else { value }
}

/// A number rounded to a number of digits, **away from zero** or
/// **toward zero**.
///
/// The tree's rule: the number is scaled by the digits, the scale is
/// rounded, and the scale is taken off again — and the sign of the
/// number is kept.
fn round_dir(number: f64, digits: i32, up: bool) -> f64 {
    if number == 0.0 || !number.is_finite() {
        return number;
    }
    let magnitude = clean(number.abs());
    let (scaled, factor, divided) = if digits >= 0 {
        let factor = 10f64.powi(digits);
        (magnitude * factor, factor, false)
    } else {
        let factor = 10f64.powi(-digits);
        (magnitude / factor, factor, true)
    };
    if !scaled.is_finite() || scaled >= 1e17 {
        return number;
    }
    let scaled = clean(scaled);
    let rounded = if up { scaled.ceil() } else { scaled.floor() };
    let value = if !divided {
        clean(rounded / factor)
    } else if rounded == 0.0 && !up {
        0.0
    } else {
        rounded * factor
    };
    if number < 0.0 { -value } else { value }
}

/// The rest of a division, with the **sign of the divisor**.
/// The **next even** or the **next odd** whole number, away from
/// nothing.
fn even_or_odd(number: f64, odd: bool) -> f64 {
    let sign = if number < 0.0 { -1.0 } else { 1.0 };
    let magnitude = clean(number.abs());
    let mut whole = magnitude.ceil();
    if odd {
        if whole % 2.0 == 0.0 {
            whole += 1.0;
        }
    } else if whole % 2.0 != 0.0 {
        whole += 1.0;
    }
    sign * whole
}

fn modulo(number: f64, divisor: f64) -> Result<f64, String> {
    if divisor == 0.0 {
        return Err("#DIV/0!".to_string());
    }
    let rest = number % divisor;
    let rest = if rest != 0.0 && (rest < 0.0) != (divisor < 0.0) { rest + divisor } else { rest };
    if rest.abs() >= divisor.abs() {
        return Ok(0.0);
    }
    Ok(clean(rest))
}

/// A number raised to a **power**.
fn power_of(number: f64, exponent: f64) -> Result<f64, String> {
    if number == 0.0 {
        if exponent == 0.0 {
            return Err("#NUM!".to_string());
        }
        if exponent < 0.0 {
            return Err("#DIV/0!".to_string());
        }
        return Ok(0.0);
    }
    if number < 0.0 && exponent.fract() != 0.0 {
        return Err("#NUM!".to_string());
    }
    let result = number.powf(exponent);
    if result.is_finite() { Ok(result) } else { Err("#NUM!".to_string()) }
}

/// The **integers** of the numbers: the tree's rule is that they must
/// be whole and not negative.
fn integers(numbers: &[f64]) -> Result<Vec<u64>, String> {
    numbers
        .iter()
        .map(|number| {
            let whole = number.trunc();
            if !(0.0..9.007_199_254_740_992e15).contains(&whole) {
                return Err("#NUM!".to_string());
            }
            Ok(whole as u64)
        })
        .collect()
}

/// The **greatest common divisor** of two whole numbers.
fn gcd_of(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let rest = left % right;
        left = right;
        right = rest;
    }
    left
}

impl Book {
    /// The **arithmetic functions**.
    pub(crate) fn arithmetic(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        // The functions of one number.
        if matches!(
            name,
            "ABS" | "SIGN" | "SQRT" | "EXP" | "LN" | "LOG10" | "INT" | "EVEN" | "ODD" | "SQRTPI"
        ) {
            let number = match self.number_at(sheet, arguments, 0, being) {
                Ok(number) => number,
                Err(message) => return Value::Error(message),
            };
            return match name {
                "EVEN" | "ODD" | "SQRTPI" => {
                    if name == "SQRTPI" {
                        if number < 0.0 {
                            Value::Error("#NUM!".to_string())
                        } else {
                            Value::Number((number * std::f64::consts::PI).sqrt())
                        }
                    } else if name == "EVEN" {
                        Value::Number(even_or_odd(number, false))
                    } else {
                        Value::Number(even_or_odd(number, true))
                    }
                }
                "ABS" => Value::Number(number.abs()),
                "SIGN" => Value::Number(if number > 0.0 {
                    1.0
                } else if number < 0.0 {
                    -1.0
                } else {
                    0.0
                }),
                "SQRT" => {
                    if number < 0.0 {
                        Value::Error("#NUM!".to_string())
                    } else {
                        Value::Number(number.sqrt())
                    }
                }
                "EXP" => Value::Number(number.exp()),
                "LN" => {
                    if number <= 0.0 {
                        Value::Error("#NUM!".to_string())
                    } else {
                        Value::Number(number.ln())
                    }
                }
                "LOG10" => {
                    if number <= 0.0 {
                        Value::Error("#NUM!".to_string())
                    } else {
                        Value::Number(number.log10())
                    }
                }
                _ => Value::Number(number.floor()),
            };
        }
        if name == "PI" {
            return Value::Number(std::f64::consts::PI);
        }
        // The functions of a number and a number of digits.
        if matches!(name, "ROUND" | "ROUNDUP" | "ROUNDDOWN" | "TRUNC") {
            let number = match self.number_at(sheet, arguments, 0, being) {
                Ok(number) => number,
                Err(message) => return Value::Error(message),
            };
            let digits = match self.digits_at(sheet, arguments, 1, being) {
                Ok(digits) => digits,
                Err(message) => return Value::Error(message),
            };
            return match name {
                "ROUND" => Value::Number(round_half_away(number, digits)),
                "ROUNDUP" => Value::Number(round_dir(number, digits, true)),
                _ => Value::Number(round_dir(number, digits, false)),
            };
        }
        // The functions of a number and another number.
        // `LOG`: the base is ten unless it is given.
        if name == "LOG" {
            let number = match self.number_at(sheet, arguments, 0, being) {
                Ok(number) => number,
                Err(message) => return Value::Error(message),
            };
            let base = match arguments.get(1) {
                Some(_) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(base) => base,
                    Err(message) => return Value::Error(message),
                },
                None => 10.0,
            };
            if number <= 0.0 || base <= 0.0 {
                return Value::Error("#NUM!".to_string());
            }
            if base == 1.0 {
                return Value::Error("#DIV/0!".to_string());
            }
            if base == 10.0 {
                return Value::Number(number.log10());
            }
            if base == 2.0 {
                return Value::Number(number.log2());
            }
            return Value::Number(number.ln() / base.ln());
        }
        if matches!(name, "MROUND" | "CEILING" | "FLOOR" | "MOD" | "QUOTIENT" | "POWER") {
            let number = match self.number_at(sheet, arguments, 0, being) {
                Ok(number) => number,
                Err(message) => return Value::Error(message),
            };
            let other = match self.number_at(sheet, arguments, 1, being) {
                Ok(other) => other,
                Err(message) => return Value::Error(message),
            };
            return match name {
                "MROUND" => {
                    if other == 0.0 || number == 0.0 {
                        Value::Number(0.0)
                    } else if (number > 0.0) != (other > 0.0) {
                        Value::Error("#NUM!".to_string())
                    } else {
                        Value::Number(clean(round_half_away(number / other, 0) * other))
                    }
                }
                "CEILING" => {
                    if number == 0.0 || other == 0.0 {
                        Value::Number(0.0)
                    } else if number > 0.0 && other < 0.0 {
                        Value::Error("#NUM!".to_string())
                    } else {
                        Value::Number(clean(clean(number / other).ceil() * other))
                    }
                }
                "FLOOR" => {
                    if other == 0.0 {
                        if number == 0.0 { Value::Number(0.0) } else { Value::Error("#DIV/0!".to_string()) }
                    } else if number == 0.0 {
                        Value::Number(0.0)
                    } else if number > 0.0 && other < 0.0 {
                        Value::Error("#NUM!".to_string())
                    } else {
                        Value::Number(clean(clean(number / other).floor() * other))
                    }
                }
                "MOD" => match modulo(number, other) {
                    Ok(rest) => Value::Number(rest),
                    Err(message) => Value::Error(message),
                },
                "QUOTIENT" => {
                    if other == 0.0 {
                        Value::Error("#DIV/0!".to_string())
                    } else {
                        Value::Number(clean(number / other).trunc())
                    }
                }
                "POWER" => match power_of(number, other) {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                },
                _ => Value::Error(format!("there is no function `{name}`")),
            };
        }
        // The functions of a set of numbers.
        let mut numbers: Vec<f64> = Vec::new();
        for index in 0..arguments.len() {
            match self.numbers_of(sheet, arguments, index, being) {
                Ok(found) => numbers.extend(found),
                Err(message) => return Value::Error(message),
            }
        }
        match name {
            "PRODUCT" => {
                if numbers.is_empty() {
                    return Value::Number(0.0);
                }
                Value::Number(numbers.iter().product())
            }
            "SUMSQ" => Value::Number(numbers.iter().map(|number| number * number).sum()),
            "GCD" => match integers(&numbers) {
                Ok(numbers) => {
                    Value::Number(numbers.iter().fold(0u64, |acc, number| gcd_of(acc, *number)) as f64)
                }
                Err(message) => Value::Error(message),
            },
            "LCM" => match integers(&numbers) {
                Ok(numbers) => {
                    let mut accumulator = 1f64;
                    for number in numbers {
                        if number == 0 {
                            return Value::Number(0.0);
                        }
                        let divisor = gcd_of(accumulator as u64, number);
                        accumulator = accumulator / divisor as f64 * number as f64;
                        if accumulator >= 9.007_199_254_740_992e15 {
                            return Value::Error("#NUM!".to_string());
                        }
                    }
                    Value::Number(accumulator)
                }
                Err(message) => Value::Error(message),
            },
            other => Value::Error(format!("there is no function `{other}`")),
        }
    }

    /// The **digits** an argument says: a whole number of digits, and
    /// the tree's bound of four hundred.
    fn digits_at(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<i32, String> {
        let number = self.number_at(sheet, arguments, index, being)?;
        Ok(number.trunc().clamp(-400.0, 400.0) as i32)
    }
}
