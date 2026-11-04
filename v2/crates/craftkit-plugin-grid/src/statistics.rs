//! The **statistics** of a formula.
//!
//! A formula may ask about the **shape** of a set of numbers:
//!
//! - `MEDIAN(numbers)` — the **middle** of the numbers, and the mean
//!   of the two middle ones when there are an even number of them;
//! - `MODE(numbers)` — the number that occurs most often, and
//!   `#N/A` when no number occurs twice;
//! - `STDEV(numbers)` and `STDEVP(numbers)` — the **spread**, the
//!   sample one and the population one;
//! - `VAR(numbers)` and `VARP(numbers)` — the **variance**, the
//!   sample one and the population one;
//! - `LARGE(numbers, k)` and `SMALL(numbers, k)` — the k-th largest
//!   and the k-th smallest;
//! - `RANK(number, numbers, [order])` — the **place** of a number in
//!   a list, counting from the largest unless the order says
//!   otherwise;
//! - `PERCENTILE(numbers, k)` and `QUARTILE(numbers, q)` — the
//!   value at a fraction of the way through the numbers, and the
//!   quarter of it;
//! - `COUNTBLANK(range)` — how many cells of the range hold
//!   nothing.
//!
//! The rules are the **tree's**, and they are the tree's own rules
//! about errors: a middle of no numbers is `#NUM!`, a spread of fewer
//! than two numbers is `#DIV/0!`, and a k that is not in the numbers
//! is `#NUM!`.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **statistical functions**.
pub fn is_statistical_function(name: &str) -> bool {
    matches!(
        name,
        "MEDIAN"
            | "MODE"
            | "STDEV"
            | "STDEVP"
            | "VAR"
            | "VARP"
            | "LARGE"
            | "SMALL"
            | "RANK"
            | "PERCENTILE"
            | "QUARTILE"
            | "COUNTBLANK"
            | "AVEDEV"
            | "DEVSQ"
            | "GEOMEAN"
            | "HARMEAN"
            | "TRIMMEAN"
            | "KURT"
            | "SKEW"
            | "SKEW.P"
            | "STANDARDIZE"
            | "FISHER"
            | "FISHERINV"
            | "MODE.SNGL"
            | "STDEV.S"
            | "STDEV.P"
            | "VAR.S"
            | "VAR.P"
            | "RANK.EQ"
            | "RANK.AVG"
            | "PERCENTILE.INC"
            | "PERCENTILE.EXC"
            | "QUARTILE.INC"
            | "QUARTILE.EXC"
            | "PERCENTRANK"
            | "PERCENTRANK.INC"
            | "PERCENTRANK.EXC"
            | "SUMX2MY2"
            | "SUMX2PY2"
            | "SUMXMY2"
    )
}

/// The **numbers** of a set, in the order a reader reads them.
fn sorted(mut numbers: Vec<f64>) -> Vec<f64> {
    numbers.sort_by(f64::total_cmp);
    numbers
}

/// The **mean** of a set of numbers.
fn mean(numbers: &[f64]) -> Result<f64, String> {
    if numbers.is_empty() {
        return Err("#DIV/0!".to_string());
    }
    Ok(numbers.iter().sum::<f64>() / numbers.len() as f64)
}

/// The **variance** of a set of numbers: of a sample or of the whole.
fn variance_of(numbers: &[f64], sample: bool) -> Result<f64, String> {
    if sample {
        if numbers.len() < 2 {
            return Err("#DIV/0!".to_string());
        }
        return Ok(devsq(numbers) / (numbers.len() - 1) as f64);
    }
    if numbers.is_empty() {
        return Err("#DIV/0!".to_string());
    }
    Ok(devsq(numbers) / numbers.len() as f64)
}

/// The **k-th percentile**, the tree's two ways.
fn percentile_of(numbers: Vec<f64>, k: f64, exclusive: bool) -> Value {
    if exclusive {
        let count = numbers.len() as f64;
        if numbers.is_empty() || !(k > 0.0 && k < 1.0) {
            return Value::Error("#NUM!".to_string());
        }
        let position = k * (count + 1.0);
        if position < 1.0 || position > count {
            return Value::Error("#NUM!".to_string());
        }
        return match interpolate(&sorted(numbers), position - 1.0) {
            Ok(value) => Value::Number(value),
            Err(message) => Value::Error(message),
        };
    }
    if numbers.is_empty() || !(0.0..=1.0).contains(&k) {
        return Value::Error("#NUM!".to_string());
    }
    let numbers = sorted(numbers);
    match interpolate(&numbers, k * (numbers.len() - 1) as f64) {
        Ok(value) => Value::Number(value),
        Err(message) => Value::Error(message),
    }
}

/// The **interpolation** between two numbers of a sorted list.
fn interpolate(numbers: &[f64], position: f64) -> Result<f64, String> {
    let index = position.floor();
    let fraction = position - index;
    let index = index as usize;
    let low = *numbers.get(index).ok_or_else(|| "#NUM!".to_string())?;
    match numbers.get(index + 1) {
        Some(high) if fraction > 0.0 => Ok(low + fraction * (high - low)),
        _ => Ok(low),
    }
}

/// A number cut to a number of **digits**.
fn cut_digits(number: f64, digits: f64) -> f64 {
    let factor = 10f64.powi(digits as i32);
    if !factor.is_finite() {
        return number;
    }
    ((number * factor) + 1e-9).floor() / factor
}

/// The **place** of a number in a list: the tree's rank.
fn rank_of(numbers: &[f64], number: f64, ascending: bool, average: bool) -> Result<f64, String> {
    let equal = numbers.iter().filter(|other| **other == number).count();
    if equal == 0 {
        return Err("#N/A".to_string());
    }
    let before =
        numbers.iter().filter(|other| if ascending { **other < number } else { **other > number }).count();
    let rank = before as f64 + 1.0;
    Ok(if average { rank + (equal as f64 - 1.0) / 2.0 } else { rank })
}

/// The **sum of the squared deviations** from the mean.
fn devsq(numbers: &[f64]) -> f64 {
    if numbers.is_empty() {
        return 0.0;
    }
    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    numbers.iter().map(|number| (number - mean) * (number - mean)).sum()
}

/// The **middle** of the numbers.
///
/// The tree's rule: the middle one when the count is odd, and the
/// mean of the two middle ones when it is even.
fn median(numbers: Vec<f64>) -> Value {
    if numbers.is_empty() {
        return Value::Error("#NUM!".to_string());
    }
    let numbers = sorted(numbers);
    let count = numbers.len();
    let high = numbers.get(count / 2).copied().unwrap_or(0.0);
    if count % 2 == 1 {
        return Value::Number(high);
    }
    let low = numbers.get(count / 2 - 1).copied().unwrap_or(0.0);
    Value::Number(low + (high - low) / 2.0)
}

/// The **modes** of the numbers: the values that occur at least twice,
/// in the order they first occur.
fn modes(numbers: &[f64]) -> Vec<f64> {
    let mut counts: Vec<(f64, usize)> = Vec::new();
    for number in numbers {
        match counts.iter_mut().find(|entry| entry.0 == *number) {
            Some(entry) => entry.1 += 1,
            None => counts.push((*number, 1)),
        }
    }
    let best = counts.iter().map(|entry| entry.1).max().unwrap_or(0);
    if best < 2 {
        return Vec::new();
    }
    counts.into_iter().filter(|entry| entry.1 == best).map(|entry| entry.0).collect()
}

/// The k-th **largest** or **smallest** of the numbers.
///
/// The tree's rule: the k is rounded up, and a k that is not in the
/// numbers is `#NUM!`.
fn kth(numbers: Vec<f64>, k: f64, largest: bool) -> Value {
    let k = k.ceil();
    if k.is_nan() || k < 1.0 || k > numbers.len() as f64 {
        return Value::Error("#NUM!".to_string());
    }
    let numbers = sorted(numbers);
    let k = k as usize;
    let index = if largest { numbers.len() - k } else { k - 1 };
    match numbers.get(index) {
        Some(number) => Value::Number(*number),
        None => Value::Error("#NUM!".to_string()),
    }
}

/// The value at a **fraction** of the way through the numbers.
///
/// The tree's rule: the fraction is between zero and one, and the
/// value is found by reading between the two numbers it falls
/// between.
fn percentile(numbers: Vec<f64>, k: f64) -> Value {
    if numbers.is_empty() || !(0.0..=1.0).contains(&k) {
        return Value::Error("#NUM!".to_string());
    }
    let numbers = sorted(numbers);
    let position = k * (numbers.len() - 1) as f64;
    let index = position.floor();
    let fraction = position - index;
    let index = index as usize;
    let Some(low) = numbers.get(index) else {
        return Value::Error("#NUM!".to_string());
    };
    match numbers.get(index + 1) {
        Some(high) if fraction > 0.0 => Value::Number(low + fraction * (high - low)),
        _ => Value::Number(*low),
    }
}

/// The **pairs** of two sets at the same places.
///
/// Two sets of different sizes have no pairs: that is `#N/A`. A pair
/// of two numbers is a pair; a pair with a text in it is not a pair,
/// and an error is the error.
pub(crate) fn pairs_of(left: &[Value], right: &[Value]) -> Result<(Vec<f64>, Vec<f64>), String> {
    if left.len() != right.len() {
        return Err("#N/A".to_string());
    }
    let mut first = Vec::new();
    let mut second = Vec::new();
    for (one, other) in left.iter().zip(right) {
        match (one, other) {
            (Value::Error(message), _) | (_, Value::Error(message)) => return Err(message.clone()),
            (Value::Number(one), Value::Number(other)) => {
                first.push(*one);
                second.push(*other);
            }
            _ => {}
        }
    }
    Ok((first, second))
}

/// The **sum of the squares** of two sets: the tree's three sums.
fn sum_of_squares(left: &[Value], right: &[Value], kind: &str) -> Result<Value, String> {
    let (first, second) = pairs_of(left, right)?;
    let mut total = 0.0;
    let mut any = false;
    for (one, other) in first.iter().zip(&second) {
        let term = match kind {
            "SUMX2MY2" => one * one - other * other,
            "SUMX2PY2" => one * one + other * other,
            _ => (one - other) * (one - other),
        };
        total += term;
        any = true;
    }
    if !any {
        return Err("#DIV/0!".to_string());
    }
    Ok(Value::Number(total))
}

impl Book {
    /// The **statistical functions**.
    pub(crate) fn statistics(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "SUMX2MY2" | "SUMX2PY2" | "SUMXMY2" => {
                if arguments.len() != 2 {
                    return Value::Error("#VALUE!".to_string());
                }
                let (left, right) = match (
                    self.values_of_cells(sheet, &arguments[0], being),
                    self.values_of_cells(sheet, &arguments[1], being),
                ) {
                    (Ok(left), Ok(right)) => (left, right),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                match sum_of_squares(&left, &right, name) {
                    Ok(value) => value,
                    Err(message) => Value::Error(message),
                }
            }
            "COUNTBLANK" => self.countblank_call(sheet, arguments, being),
            "RANK" => self.rank_call(sheet, arguments, being),
            "LARGE" => match self.numbers_of(sheet, arguments, 0, being) {
                Ok(numbers) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(k) => kth(numbers, k, true),
                    Err(message) => Value::Error(message),
                },
                Err(message) => Value::Error(message),
            },
            "SMALL" => match self.numbers_of(sheet, arguments, 0, being) {
                Ok(numbers) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(k) => kth(numbers, k, false),
                    Err(message) => Value::Error(message),
                },
                Err(message) => Value::Error(message),
            },
            "PERCENTILE" | "PERCENTILE.INC" => match self.numbers_of(sheet, arguments, 0, being) {
                Ok(numbers) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(k) => percentile_of(numbers, k, false),
                    Err(message) => Value::Error(message),
                },
                Err(message) => Value::Error(message),
            },
            "PERCENTILE.EXC" => match self.numbers_of(sheet, arguments, 0, being) {
                Ok(numbers) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(k) => percentile_of(numbers, k, true),
                    Err(message) => Value::Error(message),
                },
                Err(message) => Value::Error(message),
            },
            "QUARTILE.INC" | "QUARTILE.EXC" => match self.numbers_of(sheet, arguments, 0, being) {
                Ok(numbers) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(q) => {
                        let q = q.trunc();
                        if !(0.0..=4.0).contains(&q) {
                            return Value::Error("#NUM!".to_string());
                        }
                        percentile_of(numbers, q / 4.0, name == "QUARTILE.EXC")
                    }
                    Err(message) => Value::Error(message),
                },
                Err(message) => Value::Error(message),
            },
            "PERCENTRANK" | "PERCENTRANK.INC" | "PERCENTRANK.EXC" => {
                let numbers = match self.numbers_of(sheet, arguments, 0, being) {
                    Ok(numbers) => numbers,
                    Err(message) => return Value::Error(message),
                };
                let number = match self.number_at(sheet, arguments, 1, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let digits = match self.optional_number(sheet, arguments, 2, being) {
                    Ok(Some(digits)) => digits.trunc(),
                    Ok(None) => 3.0,
                    Err(message) => return Value::Error(message),
                };
                if digits < 1.0 || numbers.is_empty() {
                    return Value::Error("#NUM!".to_string());
                }
                let numbers = sorted(numbers);
                let first = numbers.first().copied().unwrap_or(0.0);
                let last = numbers.last().copied().unwrap_or(0.0);
                if number < first || number > last {
                    return Value::Error("#N/A".to_string());
                }
                let count = numbers.len() as f64;
                let exclusive = name == "PERCENTRANK.EXC";
                let rank_of = |value: f64| -> f64 {
                    let less = numbers.iter().filter(|other| **other < value).count() as f64;
                    if exclusive {
                        (less + 1.0) / (count + 1.0)
                    } else if count <= 1.0 {
                        1.0
                    } else {
                        less / (count - 1.0)
                    }
                };
                let rank = if numbers.contains(&number) {
                    rank_of(number)
                } else {
                    let low = numbers
                        .iter()
                        .copied()
                        .filter(|other| *other < number)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let high =
                        numbers.iter().copied().filter(|other| *other > number).fold(f64::INFINITY, f64::min);
                    let (low_rank, high_rank) = (rank_of(low), rank_of(high));
                    low_rank + (number - low) / (high - low) * (high_rank - low_rank)
                };
                Value::Number(cut_digits(rank, digits))
            }
            "RANK.EQ" | "RANK.AVG" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let numbers = match self.numbers_of(sheet, arguments, 1, being) {
                    Ok(numbers) => numbers,
                    Err(message) => return Value::Error(message),
                };
                let order = match self.optional_number(sheet, arguments, 2, being) {
                    Ok(Some(order)) => order,
                    Ok(None) => 0.0,
                    Err(message) => return Value::Error(message),
                };
                match rank_of(&numbers, number, order != 0.0, name == "RANK.AVG") {
                    Ok(rank) => Value::Number(rank),
                    Err(message) => Value::Error(message),
                }
            }
            "STANDARDIZE" => {
                let (number, average, deviation) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(number), Ok(average), Ok(deviation)) => (number, average, deviation),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                if deviation <= 0.0 {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number((number - average) / deviation)
            }
            "FISHER" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                if number <= -1.0 || number >= 1.0 {
                    return Value::Error("#NUM!".to_string());
                }
                Value::Number(0.5 * ((1.0 + number) / (1.0 - number)).ln())
            }
            "FISHERINV" => {
                let number = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                Value::Number(number.tanh())
            }
            "TRIMMEAN" => {
                let numbers = match self.numbers_of(sheet, arguments, 0, being) {
                    Ok(numbers) => numbers,
                    Err(message) => return Value::Error(message),
                };
                let percent = match self.number_at(sheet, arguments, 1, being) {
                    Ok(percent) => percent,
                    Err(message) => return Value::Error(message),
                };
                if !(0.0..1.0).contains(&percent) || numbers.is_empty() {
                    return Value::Error("#NUM!".to_string());
                }
                let count = numbers.len();
                let per_side = ((count as f64 * percent) / 2.0).floor() as usize;
                let numbers = sorted(numbers);
                let kept = numbers.get(per_side..count - per_side).unwrap_or(&[]);
                match mean(kept) {
                    Ok(mean) => Value::Number(mean),
                    Err(message) => Value::Error(message),
                }
            }
            "QUARTILE" => match self.numbers_of(sheet, arguments, 0, being) {
                Ok(numbers) => match self.number_at(sheet, arguments, 1, being) {
                    Ok(q) => {
                        let q = q.trunc();
                        if !(0.0..=4.0).contains(&q) {
                            return Value::Error("#NUM!".to_string());
                        }
                        percentile(numbers, q / 4.0)
                    }
                    Err(message) => Value::Error(message),
                },
                Err(message) => Value::Error(message),
            },
            _ => {
                let mut numbers: Vec<f64> = Vec::new();
                for index in 0..arguments.len() {
                    match self.numbers_of(sheet, arguments, index, being) {
                        Ok(found) => numbers.extend(found),
                        Err(message) => return Value::Error(message),
                    }
                }
                match name {
                    "MEDIAN" => median(numbers),
                    "MODE" => match modes(&numbers).first() {
                        Some(mode) => Value::Number(*mode),
                        None => Value::Error("#N/A".to_string()),
                    },
                    "STDEV" => match variance(&numbers, true) {
                        Ok(variance) => Value::Number(variance.sqrt()),
                        Err(message) => Value::Error(message),
                    },
                    "STDEVP" => match variance(&numbers, false) {
                        Ok(variance) => Value::Number(variance.sqrt()),
                        Err(message) => Value::Error(message),
                    },
                    "VAR" | "VAR.S" => match variance_of(&numbers, true) {
                        Ok(variance) => Value::Number(variance),
                        Err(message) => Value::Error(message),
                    },
                    "VARP" | "VAR.P" => match variance_of(&numbers, false) {
                        Ok(variance) => Value::Number(variance),
                        Err(message) => Value::Error(message),
                    },
                    "STDEV.S" => match variance_of(&numbers, true) {
                        Ok(variance) => Value::Number(variance.sqrt()),
                        Err(message) => Value::Error(message),
                    },
                    "STDEV.P" => match variance_of(&numbers, false) {
                        Ok(variance) => Value::Number(variance.sqrt()),
                        Err(message) => Value::Error(message),
                    },
                    "MODE.SNGL" => match modes(&numbers).first() {
                        Some(mode) => Value::Number(*mode),
                        None => Value::Error("#N/A".to_string()),
                    },
                    "DEVSQ" => {
                        if numbers.is_empty() {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(devsq(&numbers))
                    }
                    "AVEDEV" => match mean(&numbers) {
                        Ok(mean) => Value::Number(
                            numbers.iter().map(|number| (number - mean).abs()).sum::<f64>()
                                / numbers.len() as f64,
                        ),
                        Err(message) => Value::Error(message),
                    },
                    "GEOMEAN" => {
                        if numbers.is_empty() || numbers.iter().any(|number| *number <= 0.0) {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(
                            (numbers.iter().map(|number| number.ln()).sum::<f64>() / numbers.len() as f64)
                                .exp(),
                        )
                    }
                    "HARMEAN" => {
                        if numbers.is_empty() || numbers.iter().any(|number| *number <= 0.0) {
                            return Value::Error("#NUM!".to_string());
                        }
                        Value::Number(
                            numbers.len() as f64 / numbers.iter().map(|number| 1.0 / number).sum::<f64>(),
                        )
                    }
                    "KURT" => {
                        let count = numbers.len() as f64;
                        if numbers.len() < 4 {
                            return Value::Error("#DIV/0!".to_string());
                        }
                        let mean = match mean(&numbers) {
                            Ok(mean) => mean,
                            Err(message) => return Value::Error(message),
                        };
                        let deviation = match variance_of(&numbers, true) {
                            Ok(variance) => variance.sqrt(),
                            Err(message) => return Value::Error(message),
                        };
                        if deviation == 0.0 {
                            return Value::Error("#DIV/0!".to_string());
                        }
                        let sum =
                            numbers.iter().map(|number| ((number - mean) / deviation).powi(4)).sum::<f64>();
                        Value::Number(
                            count * (count + 1.0) / ((count - 1.0) * (count - 2.0) * (count - 3.0)) * sum
                                - 3.0 * (count - 1.0).powi(2) / ((count - 2.0) * (count - 3.0)),
                        )
                    }
                    "SKEW" => {
                        let count = numbers.len() as f64;
                        if numbers.len() < 3 {
                            return Value::Error("#DIV/0!".to_string());
                        }
                        let mean = match mean(&numbers) {
                            Ok(mean) => mean,
                            Err(message) => return Value::Error(message),
                        };
                        let deviation = match variance_of(&numbers, true) {
                            Ok(variance) => variance.sqrt(),
                            Err(message) => return Value::Error(message),
                        };
                        if deviation == 0.0 {
                            return Value::Error("#DIV/0!".to_string());
                        }
                        let sum =
                            numbers.iter().map(|number| ((number - mean) / deviation).powi(3)).sum::<f64>();
                        Value::Number(count / ((count - 1.0) * (count - 2.0)) * sum)
                    }
                    "SKEW.P" => {
                        let mean = match mean(&numbers) {
                            Ok(mean) => mean,
                            Err(message) => return Value::Error(message),
                        };
                        let deviation = match variance_of(&numbers, false) {
                            Ok(variance) => variance.sqrt(),
                            Err(message) => return Value::Error(message),
                        };
                        if deviation == 0.0 {
                            return Value::Error("#DIV/0!".to_string());
                        }
                        let sum =
                            numbers.iter().map(|number| ((number - mean) / deviation).powi(3)).sum::<f64>();
                        Value::Number(sum / numbers.len() as f64)
                    }
                    other => Value::Error(format!("there is no function `{other}`")),
                }
            }
        }
    }

    /// The tree's `COUNTBLANK`: how many cells of the range hold
    /// **nothing** — an empty cell, or a cell whose text is empty.
    fn countblank_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        let Some(argument) = arguments.first() else { return Value::Error("#VALUE!".to_string()) };
        let blank = |value: &Value| match value {
            Value::Empty => true,
            Value::Text(text) => text.is_empty(),
            _ => false,
        };
        match argument {
            Expr::Range { .. } | Expr::Name(_) => {
                let (name, area) = match self.area_of(sheet, argument) {
                    Ok(area) => area,
                    Err(message) => return Value::Error(message),
                };
                let mut count = 0usize;
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                        if blank(&value) {
                            count += 1;
                        }
                    }
                }
                Value::Number(count as f64)
            }
            other => {
                let value = self.evaluate(sheet, other, being);
                if let Value::Error(message) = &value {
                    return Value::Error(message.clone());
                }
                Value::Number(if blank(&value) { 1.0 } else { 0.0 })
            }
        }
    }

    /// The tree's `RANK`: the **place** of a number in a list.
    ///
    /// The tree's rule: the order is the largest first unless the
    /// order is not zero, and a number that is not in the list is
    /// `#N/A`.
    fn rank_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let number = match self.number_at(sheet, arguments, 0, being) {
            Ok(number) => number,
            Err(message) => return Value::Error(message),
        };
        let numbers = match self.numbers_of(sheet, arguments, 1, being) {
            Ok(numbers) => numbers,
            Err(message) => return Value::Error(message),
        };
        let order = match arguments.get(2) {
            Some(_) => match self.number_at(sheet, arguments, 2, being) {
                Ok(order) => order,
                Err(message) => return Value::Error(message),
            },
            None => 0.0,
        };
        let ascending = order != 0.0;
        let equal = numbers.iter().filter(|other| **other == number).count();
        if equal == 0 {
            return Value::Error("#N/A".to_string());
        }
        let before = numbers
            .iter()
            .filter(|other| if ascending { **other < number } else { **other > number })
            .count();
        Value::Number((before + 1) as f64)
    }
}

/// The **variance** of the numbers: the sample one or the population
/// one.
///
/// The tree's rule: the sample variance needs two numbers and the
/// population one needs one, and a set too small is `#DIV/0!`.
fn variance(numbers: &[f64], sample: bool) -> Result<f64, String> {
    if sample {
        if numbers.len() < 2 {
            return Err("#DIV/0!".to_string());
        }
        return Ok(devsq(numbers) / (numbers.len() - 1) as f64);
    }
    if numbers.is_empty() {
        return Err("#DIV/0!".to_string());
    }
    Ok(devsq(numbers) / numbers.len() as f64)
}
