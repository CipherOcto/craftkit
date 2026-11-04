//! The **combinations** of things.
//!
//! A formula may ask how many ways there are to choose:
//!
//! - `FACT(number)` — the **factorial** of a number;
//! - `FACTDOUBLE(number)` — the **double factorial**: every other
//!   number counted down;
//! - `COMBIN(number, chosen)` — the **combinations** without
//!   repetition;
//! - `COMBINA(number, chosen)` — the combinations **with** repetition;
//! - `PERMUT(number, chosen)` — the **permutations** without
//!   repetition: the same choices in a different order;
//! - `PERMUTATIONA(number, chosen)` — the permutations with
//!   repetition;
//! - `MULTINOMIAL(number, ...)` — the ways of dividing a number of
//!   things between a number of people.
//!
//! The rules are the **tree's**: the numbers are **whole** numbers, a
//! negative number is a number error, and a factorial of more than a
//! hundred and seventy is a number error, because the answer is
//! beyond a double.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **combination functions**.
pub fn is_combination_function(name: &str) -> bool {
    matches!(name, "FACT" | "FACTDOUBLE" | "COMBIN" | "COMBINA" | "PERMUT" | "PERMUTATIONA" | "MULTINOMIAL")
}

/// The **factorial** of a number.
fn factorial_of(number: f64) -> Result<f64, String> {
    let number = number.trunc();
    if number < 0.0 {
        return Err("#NUM!".to_string());
    }
    if number > 170.0 {
        return Err("#NUM!".to_string());
    }
    let mut product = 1.0f64;
    let mut index = 2.0;
    while index <= number {
        product *= index;
        index += 1.0;
    }
    Ok(product)
}

/// The **double factorial** of a number.
fn double_factorial_of(number: f64) -> Result<f64, String> {
    let number = number.trunc();
    if number < -1.0 {
        return Err("#NUM!".to_string());
    }
    if number > 300.0 {
        return Err("#NUM!".to_string());
    }
    let mut product = 1.0f64;
    let mut index = number;
    while index > 1.0 {
        product *= index;
        index -= 2.0;
    }
    Ok(product)
}

/// The number of **combinations** of `chosen` out of `number`.
fn combinations_of(number: f64, chosen: f64) -> Result<f64, String> {
    let (number, chosen) = (number.trunc(), chosen.trunc());
    if number < 0.0 || chosen < 0.0 || chosen > number {
        return Err("#NUM!".to_string());
    }
    let chosen = chosen.min(number - chosen);
    if chosen > 1.0e7 {
        let result = (crate::special::ln_gamma(number + 1.0)
            - crate::special::ln_gamma(chosen + 1.0)
            - crate::special::ln_gamma(number - chosen + 1.0))
        .exp();
        return if result.is_finite() { Ok(result) } else { Err("#NUM!".to_string()) };
    }
    let mut result = 1.0f64;
    let mut index = 1.0;
    while index <= chosen {
        result = result * (number - chosen + index) / index;
        if !result.is_finite() {
            return Err("#NUM!".to_string());
        }
        index += 1.0;
    }
    Ok(if result < 9.0e15 { result.round() } else { result })
}

impl Book {
    /// The **combination functions**.
    pub(crate) fn combinations(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "MULTINOMIAL" => {
                let mut numbers: Vec<f64> = Vec::new();
                for index in 0..arguments.len() {
                    match self.numbers_of(sheet, arguments, index, being) {
                        Ok(found) => numbers.extend(found),
                        Err(message) => return Value::Error(message),
                    }
                }
                let mut total = 0.0;
                let mut result = 1.0f64;
                for number in numbers {
                    let count = number.trunc();
                    if count < 0.0 {
                        return Value::Error("#NUM!".to_string());
                    }
                    total += count;
                    match combinations_of(total, count) {
                        Ok(found) => result *= found,
                        Err(message) => return Value::Error(message),
                    }
                    if !result.is_finite() {
                        return Value::Error("#NUM!".to_string());
                    }
                }
                Value::Number(result)
            }
            _ => {
                let first = match self.number_at(sheet, arguments, 0, being) {
                    Ok(first) => first,
                    Err(message) => return Value::Error(message),
                };
                match name {
                    "FACT" => match factorial_of(first) {
                        Ok(value) => Value::Number(value),
                        Err(message) => Value::Error(message),
                    },
                    "FACTDOUBLE" => match double_factorial_of(first) {
                        Ok(value) => Value::Number(value),
                        Err(message) => Value::Error(message),
                    },
                    "PERMUTATIONA" => {
                        let second = match self.number_at(sheet, arguments, 1, being) {
                            Ok(second) => second,
                            Err(message) => return Value::Error(message),
                        };
                        let (first, second) = (first.trunc(), second.trunc());
                        if first < 0.0 || second < 0.0 {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(first.powf(second))
                    }
                    _ => {
                        let second = match self.number_at(sheet, arguments, 1, being) {
                            Ok(second) => second,
                            Err(message) => return Value::Error(message),
                        };
                        match name {
                            "COMBIN" => match combinations_of(first, second) {
                                Ok(value) => Value::Number(value),
                                Err(message) => Value::Error(message),
                            },
                            "COMBINA" => {
                                let (first, second) = (first.trunc(), second.trunc());
                                if first < 0.0 || second < 0.0 {
                                    return Value::Error("#NUM!".to_string());
                                }
                                if second == 0.0 {
                                    return Value::Number(1.0);
                                }
                                match combinations_of(first + second - 1.0, second) {
                                    Ok(value) => Value::Number(value),
                                    Err(message) => Value::Error(message),
                                }
                            }
                            _ => {
                                // `PERMUT`: the ordered choices.
                                let (first, second) = (first.trunc(), second.trunc());
                                if first < 0.0 || second < 0.0 || second > first {
                                    return Value::Error("#NUM!".to_string());
                                }
                                let mut result = 1.0f64;
                                let mut index = 0.0;
                                while index < second {
                                    result *= first - index;
                                    if !result.is_finite() {
                                        return Value::Error("#NUM!".to_string());
                                    }
                                    index += 1.0;
                                }
                                Value::Number(result)
                            }
                        }
                    }
                }
            }
        }
    }
}
