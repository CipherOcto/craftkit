//! The **line** through a set of numbers.
//!
//! A formula may ask how two sets of numbers move together:
//!
//! - `CORREL(array1, array2)` and `PEARSON(array1, array2)` — the
//!   **correlation**;
//! - `RSQ(known_ys, known_xs)` — the **square** of the correlation;
//! - `SLOPE(known_ys, known_xs)` — the **slope** of the line;
//! - `INTERCEPT(known_ys, known_xs)` — where the line crosses the
//!   axis;
//! - `STEYX(known_ys, known_xs)` — the **standard error** of the
//!   line;
//! - `FORECAST(x, known_ys, known_xs)` — the **prediction** of the
//!   line at `x`;
//! - `COVAR`, `COVARIANCE.P` and `COVARIANCE.S` — how the two sets
//!   move together.
//!
//! The rules are the **tree's**: the two sets must be of the **same
//! length**, a pair that is not two numbers is skipped, a pair that
//! holds an error is that error, and a line that does not rise is a
//! division by nothing.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **regression functions**.
pub fn is_regression_function(name: &str) -> bool {
    matches!(
        name,
        "CORREL"
            | "PEARSON"
            | "RSQ"
            | "SLOPE"
            | "INTERCEPT"
            | "STEYX"
            | "FORECAST"
            | "COVAR"
            | "COVARIANCE.P"
            | "COVARIANCE.S"
    )
}

/// The **mean** of a set of numbers.
fn mean(numbers: &[f64]) -> Result<f64, String> {
    if numbers.is_empty() {
        return Err("#DIV/0!".to_string());
    }
    Ok(numbers.iter().sum::<f64>() / numbers.len() as f64)
}

/// The sums of the **deviations**: the cross, the square of the first
/// and the square of the second, and the two means.
fn moments(x: &[f64], y: &[f64]) -> Result<(f64, f64, f64, f64, f64), String> {
    let mean_x = mean(x)?;
    let mean_y = mean(y)?;
    let mut cross = 0.0;
    let mut square_x = 0.0;
    let mut square_y = 0.0;
    for (first, second) in x.iter().zip(y) {
        cross += (first - mean_x) * (second - mean_y);
        square_x += (first - mean_x) * (first - mean_x);
        square_y += (second - mean_y) * (second - mean_y);
    }
    Ok((cross, square_x, square_y, mean_x, mean_y))
}

impl Book {
    /// The **values** an argument holds: the cells of a range, and the
    /// value of a plain argument.
    fn regression_values(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<Vec<Value>, String> {
        match argument {
            Expr::Range { .. } | Expr::Name(_) => {
                let (name, area) = self.area_of(sheet, argument)?;
                let mut values = Vec::new();
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        values.push(self.value_of_guarded(&name, CellRef { col, row }, being));
                    }
                }
                Ok(values)
            }
            other => Ok(vec![self.evaluate(sheet, other, being)]),
        }
    }

    /// The **pairs** of numbers two arguments hold.
    ///
    /// The two must be of the same length, a pair that is not two
    /// numbers is skipped, and a pair that holds an error is that
    /// error.
    fn pairs_of(
        &self,
        sheet: &str,
        first: &Expr,
        second: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<(Vec<f64>, Vec<f64>), String> {
        let first = self.regression_values(sheet, first, being)?;
        let second = self.regression_values(sheet, second, being)?;
        if first.len() != second.len() {
            return Err("#N/A".to_string());
        }
        let mut numbers_x = Vec::new();
        let mut numbers_y = Vec::new();
        for (left, right) in first.iter().zip(second.iter()) {
            if let Value::Error(message) = left {
                return Err(message.clone());
            }
            if let Value::Error(message) = right {
                return Err(message.clone());
            }
            if let (Value::Number(left), Value::Number(right)) = (left, right) {
                numbers_x.push(*left);
                numbers_y.push(*right);
            }
        }
        Ok((numbers_x, numbers_y))
    }

    /// The **regression functions**.
    pub(crate) fn regression(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        // The two sets of numbers, and which of the arguments they
        // are.
        let (first, second) = match name {
            "FORECAST" => (1, 2),
            _ => (0, 1),
        };
        let Some(left) = arguments.get(first) else {
            return Value::Error("#VALUE!".to_string());
        };
        let Some(right) = arguments.get(second) else {
            return Value::Error("#VALUE!".to_string());
        };
        // The two sets of numbers: the first argument and the second.
        let (first, second) = match self.pairs_of(sheet, left, right, being) {
            Ok(pairs) => pairs,
            Err(message) => return Value::Error(message),
        };
        match name {
            "CORREL" | "PEARSON" => {
                if first.len() < 2 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let (cross, square_x, square_y, _, _) = match moments(&first, &second) {
                    Ok(moments) => moments,
                    Err(message) => return Value::Error(message),
                };
                if square_x == 0.0 || square_y == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                Value::Number(cross / (square_x * square_y).sqrt())
            }
            "RSQ" => {
                if first.len() < 2 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let (cross, square_x, square_y, _, _) = match moments(&first, &second) {
                    Ok(moments) => moments,
                    Err(message) => return Value::Error(message),
                };
                if square_x == 0.0 || square_y == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let correlation = cross / (square_x * square_y).sqrt();
                Value::Number(correlation * correlation)
            }
            "SLOPE" | "INTERCEPT" => {
                // The first argument is the **ys** and the second the
                // **xs**, and the line is of the xs.
                if first.is_empty() {
                    return Value::Error("#DIV/0!".to_string());
                }
                let (cross, square_x, _, mean_x, mean_y) = match moments(&second, &first) {
                    Ok(moments) => moments,
                    Err(message) => return Value::Error(message),
                };
                if square_x == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                if name == "SLOPE" {
                    Value::Number(cross / square_x)
                } else {
                    Value::Number(mean_y - cross / square_x * mean_x)
                }
            }
            "STEYX" => {
                let count = first.len() as f64;
                if first.len() < 3 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let (cross, square_x, square_y, _, _) = match moments(&second, &first) {
                    Ok(moments) => moments,
                    Err(message) => return Value::Error(message),
                };
                if square_x == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let error = ((square_y - cross * cross / square_x) / (count - 2.0)).max(0.0).sqrt();
                Value::Number(error)
            }
            "FORECAST" => {
                let at = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                if first.is_empty() {
                    return Value::Error("#DIV/0!".to_string());
                }
                // The **ys** are the second of the pair and the **xs**
                // are the third.
                let (cross, square_x, _, mean_x, mean_y) = match moments(&second, &first) {
                    Ok(moments) => moments,
                    Err(message) => return Value::Error(message),
                };
                if square_x == 0.0 {
                    return Value::Error("#DIV/0!".to_string());
                }
                let intercept = mean_y - cross / square_x * mean_x;
                Value::Number(intercept + cross / square_x * at)
            }
            _ => {
                // `COVAR`, `COVARIANCE.P` and `COVARIANCE.S`.
                let sample = name == "COVARIANCE.S";
                let count = first.len();
                if count == 0 || (sample && count < 2) {
                    return Value::Error("#DIV/0!".to_string());
                }
                let (cross, _, _, _, _) = match moments(&first, &second) {
                    Ok(moments) => moments,
                    Err(message) => return Value::Error(message),
                };
                let divisor = if sample { count as f64 - 1.0 } else { count as f64 };
                Value::Number(cross / divisor)
            }
        }
    }
}
