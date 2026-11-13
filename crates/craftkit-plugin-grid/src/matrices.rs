//! The **determinant** of a square of numbers.
//!
//! The tree has four functions of a **matrix**: `MDETERM`, `MMULT`,
//! `MINVERSE` and `MUNIT`. Three of them give back a **table** of
//! values, and so they are refused with the tree's arrays (§885) — and
//! the one that gives back a **number** is here.
//!
//! # The rule
//!
//! The determinant is the number that multiplies a row of the square
//! with another row to give nothing. The tree works it out with a
//! **partial pivoting**: the largest of the numbers below the diagonal
//! is brought up, and the sign of the answer changes when two rows
//! are swapped.
//!
//! The square must be a **square** — a square that is not one is
//! `#VALUE!` — and every cell of it must be a **number**.
//!
//! The answer is written with **fifteen significant digits**, so that
//! a determinant that should be a whole number is one.

use crate::formula::{Book, CellRef, Expr, Value};
pub const NAMES: &[&str] = &["MDETERM"];

/// The **determinant** of a square of numbers.
///
/// The tree's rule: the numbers are worked out with a partial
/// pivoting, and the answer is zero when the square has no pivot to
/// bring up.
fn determinant(size: usize, mut square: Vec<f64>) -> f64 {
    let mut det = 1.0;
    for col in 0..size {
        // The **largest** of the numbers below the diagonal is the
        // pivot, and bringing it up swaps two rows — and that swaps
        // the sign of the answer.
        let mut pivot = col;
        let mut best = 0.0;
        for row in col..size {
            let value = square[row * size + col].abs();
            if value > best {
                best = value;
                pivot = row;
            }
        }
        if best == 0.0 {
            return 0.0;
        }
        if pivot != col {
            for k in 0..size {
                square.swap(pivot * size + k, col * size + k);
            }
            det = -det;
        }
        let p = square[col * size + col];
        det *= p;
        for row in col + 1..size {
            let f = square[row * size + col] / p;
            if f == 0.0 {
                continue;
            }
            for k in col..size {
                let v = square[col * size + k];
                square[row * size + k] -= f * v;
            }
        }
    }
    det
}

impl Book {
    /// The **matrix functions**.
    pub(crate) fn matrices(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "MDETERM" => {
                let Some(argument) = arguments.first() else {
                    return Value::Error("#VALUE!".to_string());
                };
                // A plain number is a square of one cell.
                let (name, area) = match argument {
                    Expr::Range { .. } | Expr::Name(_) => match self.area_of(sheet, argument) {
                        Ok(area) => area,
                        Err(message) => return Value::Error(message),
                    },
                    other => {
                        let value = self.evaluate(sheet, other, being);
                        let Value::Number(number) = value else {
                            return match value {
                                Value::Error(message) => Value::Error(message),
                                _ => Value::Error("#VALUE!".to_string()),
                            };
                        };
                        return Value::Number(crate::arithmetic::clean(number));
                    }
                };
                let rows = (area.end.row - area.start.row + 1) as usize;
                let columns = (area.end.col - area.start.col + 1) as usize;
                if rows != columns {
                    return Value::Error("#VALUE!".to_string());
                }
                if rows > 2000 {
                    return Value::Error("#NUM!".to_string());
                }
                let mut numbers = Vec::with_capacity(rows * columns);
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                        match value {
                            Value::Number(number) => numbers.push(number),
                            Value::Error(message) => return Value::Error(message),
                            _ => return Value::Error("#VALUE!".to_string()),
                        }
                    }
                }
                let det = determinant(rows, numbers);
                if det.abs() > 1e-300 {
                    Value::Number(crate::arithmetic::clean(det))
                } else {
                    Value::Number(det)
                }
            }
            other => Value::Error(format!("there is no function `{other}`")),
        }
    }
}
