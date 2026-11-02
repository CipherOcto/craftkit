//! The **number format** of a cell.
//!
//! A number format says how a number is **written**: how many decimals
//! it has, whether the thousands are separated, and whether it is a
//! percentage. The tree's format is a whole language — dates, times,
//! currency, accounting, scientific, fractions, colours and fills — and
//! this tree has a **subset** of it:
//!
//! - `general` — the value as it is;
//! - `0` and `0.00` — a fixed number of decimals;
//! - `#,##0` and `#,##0.00` — the thousands separated;
//! - `0%` and `0.00%` — a percentage, and the value is multiplied by a
//!   hundred.
//!
//! A code that is **not one of those** is **refused**: a format that is
//! quietly ignored is a format the user believes in and the program
//! does not have.
//!
//! The tree's **rounding** is the tree's: a number is rounded away from
//! zero, so `0.125` with two decimals is `0.13` and not `0.12`. The
//! tree's **General** format fits the number to the width of the
//! column, which is a thing about a screen and not about a document;
//! here the General format is the number as it is.
//!
//! # What it does not do
//!
//! - **The tree's format is a style, and a style is shared.** In the
//!   tree a cell has a style and the style has the format, so a
//!   hundred cells with one format carry one format. Here the format
//!   is a **field of the cell**, and a hundred cells with one format
//!   carry a hundred formats. The tree's style also carries the font,
//!   the colour and the alignment, and none of those are here.
//! - **The tree's dates, times, currency and scientific formats are
//!   not here**, and neither are its **sections** — a format with a
//!   different code for negative numbers is not here.

use crate::formula::Value;

/// The most characters a format code may have.
///
/// The tree's own bound: Excel's limit, and the tree's bound on every
/// allocation that scales with the placeholders in a code.
pub const MAX_FORMAT_CHARS: usize = 255;

/// The most decimals a format may ask for.
const MAX_DECIMALS: usize = 15;

/// A number format this tree understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    /// The value as it is.
    General,
    /// A fixed number of decimals.
    Decimals {
        /// How many decimals are written.
        decimals: usize,
        /// Whether the thousands are separated by a comma.
        thousands: bool,
        /// Whether the value is a percentage and is multiplied by a
        /// hundred.
        percent: bool,
    },
}

impl Format {
    /// The format a code says, if it says one this tree knows.
    pub fn parse(code: &str) -> Option<Format> {
        let code = code.trim();
        if code.is_empty() || code.eq_ignore_ascii_case("general") {
            return Some(Format::General);
        }
        let (thousands, rest) = match code.contains(',') {
            true => (true, code.replace(',', "")),
            false => (false, code.to_string()),
        };
        let (percent, rest) = match rest.strip_suffix('%') {
            Some(rest) => (true, rest.to_string()),
            None => (false, rest),
        };
        let (whole, decimals) = match rest.split_once('.') {
            Some((whole, decimals)) => {
                if decimals.is_empty() || !decimals.chars().all(|digit| digit.is_ascii_digit()) {
                    return None;
                }
                (whole.to_string(), decimals.len())
            }
            None => (rest.clone(), 0),
        };
        if decimals > MAX_DECIMALS {
            return None;
        }
        // The whole part is a placeholder: `0` always shows a digit and
        // `#` leaves it out when it is zero.
        if whole.is_empty()
            || !whole.chars().all(|digit| digit == '#' || digit == '0')
            || !whole.contains('0')
        {
            return None;
        }
        Some(Format::Decimals { decimals, thousands, percent })
    }

    /// The **text** a value is written with.
    pub fn text(&self, value: &Value) -> String {
        match value {
            Value::Empty => String::new(),
            Value::Number(number) => match self {
                Format::General => general(*number),
                Format::Decimals { decimals, thousands, percent } => {
                    fixed(*number, *decimals, *thousands, *percent)
                }
            },
            // A format does not change a text: the tree's text sections
            // are not here.
            Value::Text(text) => text.clone(),
            Value::Bool(value) => {
                if *value {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            Value::Error(message) => message.clone(),
        }
    }
}

/// A number, as it is.
///
/// A whole number is written without its decimals, and the rest is
/// written as the number is.
fn general(number: f64) -> String {
    if !number.is_finite() {
        return format!("{number}");
    }
    if number.fract() == 0.0 && number.abs() < 1e15 {
        return format!("{}", number as i64);
    }
    format!("{number}")
}

/// A number with a fixed number of decimals.
///
/// The number is **rounded away from zero**, and the thousands are
/// separated when the format says so. A number too large to be written
/// with its decimals is written as it is.
fn fixed(number: f64, decimals: usize, thousands: bool, percent: bool) -> String {
    let number = if percent { number * 100.0 } else { number };
    if !number.is_finite() {
        return general(number);
    }
    let factor = 10f64.powi(decimals as i32);
    let scaled = (number.abs() * factor).round();
    if !scaled.is_finite() || scaled >= 9.0e15 {
        return general(number);
    }
    let units = scaled as u64;
    let divisor = factor as u64;
    let integer = units / divisor;
    let fraction = units % divisor;
    let mut out = String::new();
    if number < 0.0 {
        out.push('-');
    }
    out.push_str(&grouped(integer, thousands));
    if decimals > 0 {
        out.push('.');
        let digits = format!("{fraction:0width$}", width = decimals);
        out.push_str(&digits);
    }
    if percent {
        out.push('%');
    }
    out
}

/// A whole number, with the thousands separated when the format says
/// so.
fn grouped(number: u64, thousands: bool) -> String {
    let digits = number.to_string();
    if !thousands {
        return digits;
    }
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
