//! The **complex numbers**.
//!
//! A complex number is a pair of numbers: a **real** part and an
//! **imaginary** part. It is written as text — `3+4i` — and it is
//! read back from text by the same rules.
//!
//! - a number is a complex number with no imaginary part;
//! - a text ends with `i` or `j` and is a complex number;
//! - the text before the sign is the real part, and the text after it
//!   the imaginary one, and a text with no real part has a real part
//!   of nothing;
//! - an imaginary part of nothing and one means one and minus one.
//!
//! # What the port found
//!
//! - **The tree's complex number is written with a suffix that it
//!   keeps**: a number read from `3+4j` is written back with a `j`,
//!   and a number read from `3+4i` with an `i`. Here the same, and
//!   two numbers with different suffixes may not be put together.
//! - **The tree's whole powers are exact** and are computed by
//!   repeated multiplication when the power is a whole number that is
//!   not large, so that a Gaussian integer raised to a power is the
//!   answer and not the answer with noise in it. Here the same.

/// A **complex number**: a real part and an imaginary one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cx {
    /// The real part.
    pub(crate) re: f64,
    /// The imaginary part.
    pub(crate) im: f64,
}

impl Cx {
    /// A complex number of two parts.
    pub(crate) fn new(re: f64, im: f64) -> Cx {
        Cx { re, im }
    }

    /// The **sum** of two numbers.
    fn add(self, other: Cx) -> Cx {
        Cx::new(self.re + other.re, self.im + other.im)
    }

    /// The **difference** of two numbers.
    fn sub(self, other: Cx) -> Cx {
        Cx::new(self.re - other.re, self.im - other.im)
    }

    /// The **product** of two numbers.
    fn mul(self, other: Cx) -> Cx {
        Cx::new(self.re * other.re - self.im * other.im, self.re * other.im + self.im * other.re)
    }

    /// The **quotient** of two numbers.
    fn div(self, other: Cx) -> Result<Cx, String> {
        let divisor = other.re * other.re + other.im * other.im;
        if divisor == 0.0 {
            return Err("#NUM!".to_string());
        }
        Ok(Cx::new(
            (self.re * other.re + self.im * other.im) / divisor,
            (self.im * other.re - self.re * other.im) / divisor,
        ))
    }

    /// The **modulus**: the length of the number.
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// The **argument**: the angle of the number.
    fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    /// The **exponential** of the number.
    fn exp(self) -> Cx {
        let e = self.re.exp();
        Cx::new(e * self.im.cos(), e * self.im.sin())
    }

    /// The **logarithm** of the number.
    fn ln(self) -> Result<Cx, String> {
        if self.re == 0.0 && self.im == 0.0 {
            return Err("#NUM!".to_string());
        }
        Ok(Cx::new(self.abs().ln(), self.arg()))
    }

    /// The **sine** of the number.
    fn sin(self) -> Cx {
        Cx::new(self.re.sin() * self.im.cosh(), self.re.cos() * self.im.sinh())
    }

    /// The **cosine** of the number.
    fn cos(self) -> Cx {
        Cx::new(self.re.cos() * self.im.cosh(), -self.re.sin() * self.im.sinh())
    }

    /// The **hyperbolic sine** of the number.
    fn sinh(self) -> Cx {
        Cx::new(self.re.sinh() * self.im.cos(), self.re.cosh() * self.im.sin())
    }

    /// The **hyperbolic cosine** of the number.
    fn cosh(self) -> Cx {
        Cx::new(self.re.cosh() * self.im.cos(), self.re.sinh() * self.im.sin())
    }

    /// The number raised to a **power**.
    fn powf(self, power: f64) -> Result<Cx, String> {
        if self.re == 0.0 && self.im == 0.0 {
            return if power > 0.0 { Ok(Cx::new(0.0, 0.0)) } else { Err("#NUM!".to_string()) };
        }
        let radius = self.abs().powf(power);
        let angle = self.arg() * power;
        Ok(Cx::new(radius * angle.cos(), radius * angle.sin()))
    }
}

/// The complex number **one**.
const ONE: Cx = Cx { re: 1.0, im: 0.0 };

/// Whether a **name** is the name of a complex function.
pub(crate) fn is_complex_function(name: &str) -> bool {
    matches!(
        name,
        "COMPLEX"
            | "IMABS"
            | "IMAGINARY"
            | "IMARGUMENT"
            | "IMCONJUGATE"
            | "IMCOS"
            | "IMCOSH"
            | "IMCOT"
            | "IMCSC"
            | "IMCSCH"
            | "IMDIV"
            | "IMEXP"
            | "IMLN"
            | "IMLOG10"
            | "IMLOG2"
            | "IMPOWER"
            | "IMPRODUCT"
            | "IMREAL"
            | "IMSEC"
            | "IMSECH"
            | "IMSIN"
            | "IMSINH"
            | "IMSQRT"
            | "IMSUB"
            | "IMSUM"
            | "IMTAN"
    )
}

/// A **real number** written in a complex text.
fn parse_real(text: &str) -> Option<f64> {
    if text.is_empty() || text.contains(char::is_whitespace) {
        return None;
    }
    let readable = text
        .chars()
        .all(|character| character.is_ascii_digit() || matches!(character, '.' | 'e' | 'E' | '+' | '-'));
    if !readable {
        return None;
    }
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

/// A **complex number** written as text, and the **suffix** it was
/// written with.
pub(crate) fn parse_complex(text: &str) -> Result<(Cx, Option<char>), String> {
    if text.is_empty() {
        return Ok((Cx::new(0.0, 0.0), None));
    }
    let suffix = match text.chars().last() {
        Some(character @ ('i' | 'j')) => Some(character),
        _ => None,
    };
    let Some(suffix) = suffix else {
        return parse_real(text).map(|real| (Cx::new(real, 0.0), None)).ok_or_else(|| "#NUM!".to_string());
    };
    let body = &text[..text.len() - 1];
    // The split is at the last sign that is not at the start and not
    // part of an exponent.
    let bytes = body.as_bytes();
    let mut split = None;
    for index in (1..bytes.len()).rev() {
        let character = bytes.get(index).copied().unwrap_or(0);
        let previous = bytes.get(index - 1).copied().unwrap_or(0);
        if (character == b'+' || character == b'-') && previous != b'e' && previous != b'E' {
            split = Some(index);
            break;
        }
    }
    let (real_text, imaginary_text) = match split {
        Some(index) => (&body[..index], &body[index..]),
        None => ("", body),
    };
    let real = if real_text.is_empty() { 0.0 } else { parse_real(real_text).ok_or("#NUM!")? };
    let imaginary = match imaginary_text {
        "" | "+" => 1.0,
        "-" => -1.0,
        other => parse_real(other).ok_or("#NUM!")?,
    };
    Ok((Cx::new(real, imaginary), Some(suffix)))
}

/// The complex number a **value** is.
fn cx_of(value: &crate::formula::Value) -> Result<(Cx, Option<char>), String> {
    match value {
        crate::formula::Value::Number(number) => Ok((Cx::new(*number, 0.0), None)),
        crate::formula::Value::Empty => Ok((Cx::new(0.0, 0.0), None)),
        crate::formula::Value::Bool(_) => Err("#VALUE!".to_string()),
        crate::formula::Value::Error(message) => Err(message.clone()),
        crate::formula::Value::Text(text) => parse_complex(text),
    }
}

/// The **suffixes** of two numbers must agree.
fn merge_suffix(one: Option<char>, other: Option<char>) -> Result<Option<char>, String> {
    match (one, other) {
        (Some(one), Some(other)) if one != other => Err("#VALUE!".to_string()),
        (Some(one), _) => Ok(Some(one)),
        (None, other) => Ok(other),
    }
}

/// A number with **fifteen significant digits**, so that a value like
/// `0.999999999999999999` is written as one.
fn clean(number: f64) -> f64 {
    let text = format!("{number:.14e}");
    text.parse().ok().unwrap_or(number)
}

/// A complex number, written as text.
pub(crate) fn write(number: Cx, suffix: char) -> Result<crate::formula::Value, String> {
    if !number.re.is_finite() || !number.im.is_finite() {
        return Err("#NUM!".to_string());
    }
    let real = clean(number.re);
    let imaginary = clean(number.im);
    let mut text = String::new();
    if imaginary == 0.0 {
        text.push_str(&crate::texts::value_text(&crate::formula::Value::Number(real))?);
        return Ok(crate::formula::Value::Text(text));
    }
    if real != 0.0 {
        text.push_str(&crate::texts::value_text(&crate::formula::Value::Number(real))?);
        if imaginary > 0.0 {
            text.push('+');
        }
    }
    if imaginary == -1.0 {
        text.push('-');
    } else if imaginary != 1.0 {
        text.push_str(&crate::texts::value_text(&crate::formula::Value::Number(imaginary))?);
    }
    text.push(suffix);
    Ok(crate::formula::Value::Text(text))
}

/// Call a **complex function**.
///
/// The arguments are the **values** of the arguments of the formula:
/// a range brings its cells and a plain argument brings one value.
pub(crate) fn call(
    name: &str,
    arguments: &[Vec<crate::formula::Value>],
) -> Result<crate::formula::Value, String> {
    use crate::formula::Value;
    let first = arguments.first().and_then(|values| values.first()).cloned().unwrap_or(Value::Empty);
    let second = arguments.get(1).and_then(|values| values.first()).cloned().unwrap_or(Value::Empty);
    let unary = |number: Cx| -> Result<Cx, String> {
        match name {
            "IMCONJUGATE" => Ok(Cx::new(number.re, -number.im)),
            "IMEXP" => Ok(number.exp()),
            "IMLN" => number.ln(),
            "IMLOG10" => number
                .ln()
                .map(|log| Cx::new(log.re / std::f64::consts::LN_10, log.im / std::f64::consts::LN_10)),
            "IMLOG2" => number
                .ln()
                .map(|log| Cx::new(log.re / std::f64::consts::LN_2, log.im / std::f64::consts::LN_2)),
            "IMSQRT" => {
                let modulus = number.abs();
                let real = ((modulus + number.re) / 2.0).sqrt();
                let imaginary = ((modulus - number.re) / 2.0).sqrt();
                Ok(Cx::new(real, if number.im < 0.0 { -imaginary } else { imaginary }))
            }
            "IMSIN" => Ok(number.sin()),
            "IMCOS" => Ok(number.cos()),
            "IMTAN" => number.sin().div(number.cos()),
            "IMCOT" => number.cos().div(number.sin()),
            "IMSEC" => ONE.div(number.cos()),
            "IMCSC" => ONE.div(number.sin()),
            "IMSINH" => Ok(number.sinh()),
            "IMCOSH" => Ok(number.cosh()),
            "IMSECH" => ONE.div(number.cosh()),
            "IMCSCH" => ONE.div(number.sinh()),
            _ => Err("#VALUE!".to_string()),
        }
    };
    match name {
        "IMABS" => {
            let (number, _) = cx_of(&first)?;
            Ok(Value::Number(number.abs()))
        }
        "IMREAL" => {
            let (number, _) = cx_of(&first)?;
            Ok(Value::Number(number.re))
        }
        "IMAGINARY" => {
            let (number, _) = cx_of(&first)?;
            Ok(Value::Number(number.im))
        }
        "IMARGUMENT" => {
            let (number, _) = cx_of(&first)?;
            if number.re == 0.0 && number.im == 0.0 {
                return Err("#DIV/0!".to_string());
            }
            Ok(Value::Number(number.arg()))
        }
        "IMDIV" => {
            let (one, suffix_one) = cx_of(&first)?;
            let (other, suffix_other) = cx_of(&second)?;
            let suffix = merge_suffix(suffix_one, suffix_other)?;
            write(one.div(other)?, suffix.unwrap_or('i'))
        }
        "IMSUB" => {
            let (one, suffix_one) = cx_of(&first)?;
            let (other, suffix_other) = cx_of(&second)?;
            let suffix = merge_suffix(suffix_one, suffix_other)?;
            write(one.sub(other), suffix.unwrap_or('i'))
        }
        "IMPOWER" => {
            let (number, suffix) = cx_of(&first)?;
            let power = match second {
                Value::Number(power) => power,
                Value::Error(message) => return Err(message),
                _ => return Err("#VALUE!".to_string()),
            };
            // A whole power is computed by repeated multiplication, and
            // so a Gaussian integer raised to a power is the answer.
            let result =
                if power == power.trunc() && power.abs() <= 64.0 && !(number.re == 0.0 && number.im == 0.0) {
                    let mut accumulator = ONE;
                    for _ in 0..(power.abs() as u32) {
                        accumulator = accumulator.mul(number);
                    }
                    if power < 0.0 { ONE.div(accumulator)? } else { accumulator }
                } else {
                    number.powf(power)?
                };
            write(result, suffix.unwrap_or('i'))
        }
        "IMSUM" | "IMPRODUCT" => {
            let mut accumulator = if name == "IMSUM" { Cx::new(0.0, 0.0) } else { ONE };
            let mut suffix: Option<char> = None;
            for values in arguments {
                let direct = values.len() <= 1;
                for value in values {
                    if matches!(value, Value::Empty) && !direct {
                        continue;
                    }
                    let (number, own) = cx_of(value)?;
                    suffix = merge_suffix(suffix, own)?;
                    accumulator =
                        if name == "IMSUM" { accumulator.add(number) } else { accumulator.mul(number) };
                }
            }
            write(accumulator, suffix.unwrap_or('i'))
        }
        _ => {
            let (number, suffix) = cx_of(&first)?;
            write(unary(number)?, suffix.unwrap_or('i'))
        }
    }
}
