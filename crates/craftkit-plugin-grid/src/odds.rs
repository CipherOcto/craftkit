//! The **odds and ends** of the tree's formula language.
//!
//! Four functions that do not belong to a family of their own:
//!
//! - `SUMPRODUCT(array1, [array2], ...)` — the sum of the products of
//!   the numbers of the sets, taken one place at a time;
//! - `SERIESSUM(x, n, m, coefficients)` — the sum of a **power
//!   series**;
//! - `PROB(x_range, prob_range, [lower], [upper])` — the probability
//!   of a range of values;
//! - `PERCENTOF(data_subset, data_all)` — the sum of a subset as a
//!   fraction of the sum of all the values.
//!
//! The rules are the **tree's**: a set of numbers that is not of the
//! same size as the others is a value error, a **text** in a set of
//! cells counts as nothing and a text that is an argument on its own
//! is a value error, and the probabilities of a `PROB` must add up to
//! one.

use crate::formula::{Book, CellRef, Expr, Value};
pub const NAMES: &[&str] = &["SUMPRODUCT", "SERIESSUM", "PROB", "PERCENTOF"];

impl Book {
    /// The **values** an argument holds.
    ///
    /// A **range** gives the values of its cells, one after another; an
    /// argument on its own gives one value. The second half of the answer
    /// says whether the argument was a range — and that matters, because a
    /// text in a range counts as nothing and a text on its own is an
    /// error.
    fn values_of_argument(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> (Vec<Value>, bool) {
        match argument {
            Expr::Range { .. } | Expr::Name(_) => {
                (self.values_of_cells(sheet, argument, being).unwrap_or_default(), true)
            }
            other => (vec![self.evaluate(sheet, other, being)], false),
        }
    }

    /// The **odds and ends**.
    pub(crate) fn odds(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "SUMPRODUCT" => self.sumproduct(sheet, arguments, being),
            "SERIESSUM" => self.seriessum(sheet, arguments, being),
            "PROB" => self.prob(sheet, arguments, being),
            _ => self.percentof(sheet, arguments, being),
        }
    }

    /// The tree's `SUMPRODUCT`: the sum of the products of the sets,
    /// taken one place at a time.
    ///
    /// The tree's rule: the sets must be of the **same size**, and a
    /// **text** in a set of cells counts as nothing — and a text that
    /// is an argument on its own is a value error.
    fn sumproduct(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.is_empty() {
            return Value::Error("#VALUE!".to_string());
        }
        let mut sets: Vec<Vec<Value>> = Vec::new();
        for argument in arguments {
            let (values, _) = self.values_of_argument(sheet, argument, being);
            sets.push(values);
        }
        let size = sets.first().map_or(0, Vec::len);
        if sets.iter().any(|set| set.len() != size) {
            return Value::Error("#VALUE!".to_string());
        }
        let mut total = 0.0;
        for index in 0..size {
            let mut product = 1.0;
            for (set, argument) in sets.iter().zip(arguments) {
                let value = set.get(index).cloned().unwrap_or(Value::Empty);
                match value {
                    Value::Number(number) => product *= number,
                    Value::Error(message) => return Value::Error(message),
                    Value::Text(_) => {
                        // A **text** that is an argument on its own is
                        // a value error, and a text in a set of cells
                        // counts as nothing.
                        let alone = !matches!(argument, Expr::Range { .. } | Expr::Name(_));
                        if alone {
                            return Value::Error("#VALUE!".to_string());
                        }
                        product = 0.0;
                    }
                    _ => product = 0.0,
                }
            }
            total += product;
        }
        Value::Number(total)
    }

    /// The tree's `SERIESSUM`: the sum of a **power series**.
    ///
    /// The tree's rule: the coefficients are a set of cells, and a
    /// coefficient that is not a number is a value error.
    fn seriessum(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let (x, n, m) = match (
            self.number_at(sheet, arguments, 0, being),
            self.number_at(sheet, arguments, 1, being),
            self.number_at(sheet, arguments, 2, being),
        ) {
            (Ok(x), Ok(n), Ok(m)) => (x, n, m),
            (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                return Value::Error(message);
            }
        };
        let coefficients = self.values_of_cells(sheet, &arguments[3], being);
        let Ok(coefficients) = coefficients else {
            return Value::Error("#VALUE!".to_string());
        };
        let mut total = 0.0;
        for (index, value) in coefficients.iter().enumerate() {
            let coefficient = match value {
                Value::Number(number) => *number,
                Value::Error(message) => return Value::Error(message.clone()),
                _ => return Value::Error("#VALUE!".to_string()),
            };
            let exponent = n + index as f64 * m;
            match crate::arithmetic::power_of(x, exponent) {
                Ok(value) => total += coefficient * value,
                Err(message) => return Value::Error(message),
            }
        }
        Value::Number(total)
    }

    /// The tree's `PROB`: the probability of a **range** of values.
    ///
    /// The tree's rule: the two sets must be of the same size, every
    /// probability must be **more than nothing** and **not more than
    /// one**, and the probabilities must add up to **one**.
    fn prob(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let (left, right) = match (
            self.values_of_cells(sheet, &arguments[0], being),
            self.values_of_cells(sheet, &arguments[1], being),
        ) {
            (Ok(left), Ok(right)) => (left, right),
            (Err(message), _) | (_, Err(message)) => return Value::Error(message),
        };
        if left.len() != right.len() {
            return Value::Error("#N/A".to_string());
        }
        let mut pairs: Vec<(f64, f64)> = Vec::new();
        for (first, second) in left.iter().zip(right.iter()) {
            if let Value::Error(message) = first {
                return Value::Error(message.clone());
            }
            if let Value::Error(message) = second {
                return Value::Error(message.clone());
            }
            if let (Value::Number(x), Value::Number(probability)) = (first, second) {
                pairs.push((*x, *probability));
            }
        }
        let lower = match self.optional_number(sheet, arguments, 2, being) {
            Ok(Some(number)) => number,
            Ok(None) => f64::NEG_INFINITY,
            Err(message) => return Value::Error(message),
        };
        let upper = match self.optional_number(sheet, arguments, 3, being) {
            Ok(Some(number)) => number,
            Ok(None) => lower,
            Err(message) => return Value::Error(message),
        };
        if pairs.iter().any(|(_, probability)| *probability <= 0.0 || *probability > 1.0) {
            return Value::Error("#NUM!".to_string());
        }
        let total: f64 = pairs.iter().map(|(_, probability)| probability).sum();
        if (total - 1.0).abs() > 1e-7 {
            return Value::Error("#NUM!".to_string());
        }
        let answer: f64 = pairs
            .iter()
            .filter(|(x, _)| *x >= lower && *x <= upper)
            .map(|(_, probability)| probability)
            .sum();
        Value::Number(answer)
    }

    /// The tree's `PERCENTOF`: the sum of a subset as a fraction of
    /// the sum of all the values.
    ///
    /// The tree's rule: a total of nothing is a division by nothing.
    fn percentof(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let part = match self.numbers_of(sheet, arguments, 0, being) {
            Ok(numbers) => numbers,
            Err(message) => return Value::Error(message),
        };
        let all = match self.numbers_of(sheet, arguments, 1, being) {
            Ok(numbers) => numbers,
            Err(message) => return Value::Error(message),
        };
        let all: f64 = all.iter().sum();
        if all == 0.0 {
            return Value::Error("#DIV/0!".to_string());
        }
        Value::Number(part.iter().sum::<f64>() / all)
    }
}
