//! The **value of a text**: a text that holds a number, a date or a
//! time.
//!
//! `VALUE` reads a text as a number, `DATEVALUE` reads a text as a
//! date and `TIMEVALUE` reads a text as a time. A date in a
//! spreadsheet is a number and a format, and so a time is a fraction
//! of a day.
//!
//! # The text that is a number
//!
//! The tree's rules, and they are careful:
//!
//! - a **sign** may stand before the number: `-5`, `+5`;
//! - a **currency sign** may stand before it: `$10`, and `-$10`;
//! - **thousands separators** may stand in it, in groups of three;
//! - a **percent sign** may stand after it, and the number is then a
//!   hundredth of itself: `50%` is a half;
//! - an **exponent** may stand in it: `1.5e3`;
//! - **parentheses** around it make it negative: `(5)` is `-5`, and
//!   so are the accounting forms `$(5)` and `(5)%`;
//! - a **date** or a **time** written as text is a number too.
//!
//! # What the port found
//!
//! - **The tree's value of a blank cell is zero** and the value of an
//!   empty text is not: `VALUE("")` is `#VALUE!`, as the tree's own
//!   note says it observed. Here the same, and the two are not the
//!   same thing.
//! - **The tree's currency sign and percent sign may not stand
//!   together**: `$5%` is not a number. Here the same.
//! - **The tree's thousands separators must be in groups of three**,
//!   and `1,23` is not a number. Here the same.
//! - **The tree's date is read in the calendar of the workbook**, and
//!   so the day that never happened is the day that never happened
//!   here too.

use crate::dates::System;
pub(crate) const NAMES: &[&str] = &["VALUE", "DATEVALUE", "TIMEVALUE", "NUMBERVALUE"];

/// The **number inside accounting parentheses**, as plain text.
fn parenthesized(text: &str) -> Option<String> {
    let (currency, rest) = match text.strip_prefix('$') {
        Some(rest) => ("$", rest.trim_start()),
        None => ("", text),
    };
    let rest = rest.strip_prefix('(')?;
    let (body, percent) = match rest.strip_suffix(')') {
        Some(body) => (body, ""),
        None => (rest.strip_suffix('%')?.trim_end().strip_suffix(')')?, "%"),
    };
    Some(format!("{currency}{}{percent}", body.trim()))
}

/// A **plain number**: digits, with an optional fraction.
fn plain_number(text: &str, fraction: bool) -> Option<f64> {
    let (whole, decimal) = match text.split_once('.') {
        Some((whole, decimal)) if fraction => (whole, Some(decimal)),
        _ => (text, None),
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(whole) || decimal.is_some_and(|decimal| !digits(decimal)) {
        return None;
    }
    text.parse().ok().filter(|number: &f64| number.is_finite())
}

/// A **number** with a sign, a currency sign, thousands separators, a
/// percent sign or an exponent.
fn parse_plain(text: &str) -> Option<f64> {
    let mut rest = text;
    let mut negative = false;
    let mut currency = false;
    let mut percent = false;
    if let Some(after) = rest.strip_prefix('-') {
        negative = true;
        rest = after.trim_start();
    } else if let Some(after) = rest.strip_prefix('+') {
        rest = after.trim_start();
    }
    if let Some(after) = rest.strip_prefix('$') {
        currency = true;
        rest = after.trim_start();
        if !negative && let Some(after) = rest.strip_prefix('-') {
            negative = true;
            rest = after;
        }
    }
    if let Some(after) = rest.strip_suffix('%') {
        percent = true;
        rest = after.trim_end();
    }
    // A currency sign and a percent sign may not stand together.
    if rest.is_empty() || (currency && percent) {
        return None;
    }
    let separated = rest.contains(',');
    let digits = if separated {
        // The separators are in groups of three before the point.
        let (whole, fraction) =
            rest.split_once('.').map_or((rest, None), |(whole, fraction)| (whole, Some(fraction)));
        let groups: Vec<&str> = whole.split(',').collect();
        let first = groups.first()?;
        if first.is_empty() || first.len() > 3 || groups.iter().skip(1).any(|group| group.len() != 3) {
            return None;
        }
        let mut digits = groups.concat();
        if let Some(fraction) = fraction {
            digits.push('.');
            digits.push_str(fraction);
        }
        digits
    } else {
        rest.to_string()
    };
    let bytes = digits.as_bytes();
    if !bytes.iter().all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-')) {
        return None;
    }
    if !bytes.first().is_some_and(|byte| byte.is_ascii_digit() || *byte == b'.') {
        return None;
    }
    let mut number: f64 = digits.parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    if percent {
        number /= 100.0;
    }
    if negative {
        number = -number;
    }
    Some(number)
}

/// A **fraction**: `1 1/2`, `0 3/4`.
fn parse_fraction(text: &str) -> Option<f64> {
    let (whole, fraction) = text.split_once(' ')?;
    let negative = whole.starts_with('-');
    let whole = plain_number(whole.trim_start_matches('-'), false)?;
    let (top, bottom) = fraction.trim().split_once('/')?;
    let top = plain_number(top, false)?;
    let bottom = plain_number(bottom, false)?;
    if bottom == 0.0 || !whole.is_finite() {
        return None;
    }
    let value = whole + top / bottom;
    if !value.is_finite() {
        return None;
    }
    Some(if negative { -value } else { value })
}

/// The **date** a text holds, and the **time** of it.
fn parse_date(text: &str, system: System) -> Option<f64> {
    let (date, _) = crate::dates::parse(text, system)?;
    crate::dates::to_serial(date.year, date.month, date.day, system)
}

/// The **fraction of a day** a text holds.
fn parse_time(text: &str) -> Option<f64> {
    let (time, _) = crate::dates::parse_time(text)?;
    Some(crate::dates::time_fraction(time.hour, time.minute, time.second))
}

/// The **number** a text holds.
pub(crate) fn number_of(text: &str, system: System) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(inner) = parenthesized(text) {
        // No sign and no nested parentheses inside.
        if inner.strip_prefix('$').map_or(inner.as_str(), str::trim_start).starts_with(['-', '+', '(']) {
            return None;
        }
        let number = parse_plain(&inner).or_else(|| parse_fraction(&inner))?;
        return Some(-number);
    }
    if let Some(number) = parse_plain(text) {
        return Some(number);
    }
    if let Some(number) = parse_fraction(text) {
        return Some(number);
    }
    if let Some(serial) = parse_date(text, system) {
        return Some(serial);
    }
    parse_time(text)
}

/// The **date** a text holds: the number of the day.
pub(crate) fn date_of(text: &str, system: System) -> Option<f64> {
    parse_date(text.trim(), system)
}

/// The **time** a text holds: the fraction of the day.
pub(crate) fn time_of(text: &str, system: System) -> Option<f64> {
    let _ = system;
    parse_time(text.trim())
}
