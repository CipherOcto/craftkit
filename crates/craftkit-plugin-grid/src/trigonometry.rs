//! The **trigonometry** of a formula.
//!
//! A formula may ask about an **angle**: its sine and its cosine, its
//! tangent and their inverses, the hyperbolic ones and their inverses,
//! the reciprocals of the three, and the turn of an angle from
//! degrees to radians and back.
//!
//! The rules are the **tree's**:
//!
//! - an angle of a trigonometric function is bounded — the tree's
//!   bound is `2^27`, and an angle beyond it is `#NUM!`;
//! - `ASIN` and `ACOS` are of a number between minus one and one, and
//!   `ACOSH` is of a number of one or more, and `ATANH` is of a
//!   number between minus one and one;
//! - `COT`, `COTH`, `CSC` and `CSCH` are the reciprocals, and a
//!   reciprocal of nothing is `#DIV/0!`;
//! - `ATAN2` is the angle of a **point**, and the point `0,0` is no
//!   point and is `#DIV/0!`.

use crate::formula::{Book, CellRef, Expr, Value};

/// The tree's bound on the argument of a trigonometric function:
/// `2^27`.
const TRIG_LIMIT: f64 = 134_217_728.0;

/// Whether a name is one of the **trigonometric functions**.
pub fn is_trigonometric_function(name: &str) -> bool {
    matches!(
        name,
        "SIN"
            | "COS"
            | "TAN"
            | "ASIN"
            | "ACOS"
            | "ATAN"
            | "ATAN2"
            | "SINH"
            | "COSH"
            | "TANH"
            | "ASINH"
            | "ACOSH"
            | "ATANH"
            | "COT"
            | "COTH"
            | "CSC"
            | "CSCH"
            | "SEC"
            | "SECH"
            | "ACOT"
            | "ACOTH"
            | "DEGREES"
            | "RADIANS"
    )
}

/// An **angle** a trigonometric function may take.
fn angle(number: f64) -> Result<f64, String> {
    if number.abs() >= TRIG_LIMIT {
        return Err("#NUM!".to_string());
    }
    Ok(number)
}

/// A number that is **not nothing**.
fn nonzero(number: f64) -> Result<f64, String> {
    if number == 0.0 { Err("#DIV/0!".to_string()) } else { Ok(number) }
}

impl Book {
    /// The **trigonometric functions**.
    pub(crate) fn trigonometry(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        // `ATAN2` is the angle of a **point**.
        if name == "ATAN2" {
            let x = match self.number_at(sheet, arguments, 0, being) {
                Ok(number) => number,
                Err(message) => return Value::Error(message),
            };
            let y = match self.number_at(sheet, arguments, 1, being) {
                Ok(number) => number,
                Err(message) => return Value::Error(message),
            };
            if x == 0.0 && y == 0.0 {
                return Value::Error("#DIV/0!".to_string());
            }
            return Value::Number(y.atan2(x));
        }
        let number = match self.number_at(sheet, arguments, 0, being) {
            Ok(number) => number,
            Err(message) => return Value::Error(message),
        };
        match name {
            "SIN" => match angle(number) {
                Ok(angle) => Value::Number(angle.sin()),
                Err(message) => Value::Error(message),
            },
            "COS" => match angle(number) {
                Ok(angle) => Value::Number(angle.cos()),
                Err(message) => Value::Error(message),
            },
            "TAN" => match angle(number) {
                Ok(angle) => Value::Number(angle.tan()),
                Err(message) => Value::Error(message),
            },
            "ASIN" => {
                if number.abs() > 1.0 {
                    Value::Error("#NUM!".to_string())
                } else {
                    Value::Number(number.asin())
                }
            }
            "ACOS" => {
                if number.abs() > 1.0 {
                    Value::Error("#NUM!".to_string())
                } else {
                    Value::Number(number.acos())
                }
            }
            "ATAN" => Value::Number(number.atan()),
            "SINH" => Value::Number(number.sinh()),
            "COSH" => Value::Number(number.cosh()),
            "TANH" => Value::Number(number.tanh()),
            "ASINH" => Value::Number(number.asinh()),
            "ACOSH" => {
                if number < 1.0 {
                    Value::Error("#NUM!".to_string())
                } else {
                    Value::Number(number.acosh())
                }
            }
            "ATANH" => {
                if number.abs() >= 1.0 {
                    Value::Error("#NUM!".to_string())
                } else {
                    Value::Number(number.atanh())
                }
            }
            "COT" => match angle(number).and_then(|angle| nonzero(angle.tan())) {
                Ok(tangent) => Value::Number(1.0 / tangent),
                Err(message) => Value::Error(message),
            },
            "COTH" => match angle(number).and_then(|angle| nonzero(angle.tanh())) {
                Ok(tangent) => Value::Number(1.0 / tangent),
                Err(message) => Value::Error(message),
            },
            "CSC" => match angle(number).and_then(|angle| nonzero(angle.sin())) {
                Ok(sine) => Value::Number(1.0 / sine),
                Err(message) => Value::Error(message),
            },
            "CSCH" => match angle(number).and_then(|angle| nonzero(angle.sinh())) {
                Ok(sine) => Value::Number(1.0 / sine),
                Err(message) => Value::Error(message),
            },
            "SEC" => match angle(number) {
                Ok(angle) => Value::Number(1.0 / angle.cos()),
                Err(message) => Value::Error(message),
            },
            "SECH" => match angle(number) {
                Ok(angle) => Value::Number(1.0 / angle.cosh()),
                Err(message) => Value::Error(message),
            },
            "ACOT" => Value::Number(std::f64::consts::PI / 2.0 - number.atan()),
            "ACOTH" => {
                if number.abs() <= 1.0 {
                    Value::Error("#NUM!".to_string())
                } else {
                    Value::Number(0.5 * ((number + 1.0) / (number - 1.0)).ln())
                }
            }
            "DEGREES" => Value::Number(number * 180.0 / std::f64::consts::PI),
            "RADIANS" => Value::Number(number * std::f64::consts::PI / 180.0),
            other => Value::Error(format!("there is no function `{other}`")),
        }
    }
}
