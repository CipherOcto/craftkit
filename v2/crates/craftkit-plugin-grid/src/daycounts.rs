//! The **days** between two dates.
//!
//! The tree's own day counts, in the tree's own bases:
//!
//! - the basis **zero** — the American 30/360;
//! - the basis **one** — the actual days over the actual days of the
//!   year;
//! - the basis **two** — the actual days over a year of three hundred
//!   and sixty days;
//! - the basis **three** — the actual days over a year of three hundred
//!   and sixty-five days;
//! - the basis **four** — the European 30/360.
//!
//! A basis is a number from zero to four and no other, and a
//! **frequency** — how many times a year a security pays — is one, two
//! or four and no other.

use crate::dates::{System, days_in_month, from_serial, is_leap, to_serial};

/// The year, the month and the day of a **serial**.
pub(crate) fn ymd(system: System, serial: f64) -> Option<(i32, u32, u32)> {
    let date = from_serial(serial.trunc(), system)?;
    Some((date.year, date.month, date.day.max(1)))
}

/// The **30/360** day count: the American one and the European one.
pub(crate) fn days360(system: System, first: f64, second: f64, us: bool) -> Option<f64> {
    let (y1, m1, mut d1) = ymd(system, first)?;
    let (y2, m2, mut d2) = ymd(system, second)?;
    if us {
        let february_end_first = m1 == 2 && d1 >= days_in_month(y1, 2);
        let february_end_second = m2 == 2 && d2 >= days_in_month(y2, 2);
        if february_end_first && february_end_second {
            d2 = 30;
        }
        if february_end_first {
            d1 = 30;
        }
        if d2 == 31 && d1 >= 30 {
            d2 = 30;
        }
        if d1 == 31 {
            d1 = 30;
        }
    } else {
        d1 = d1.min(30);
        d2 = d2.min(30);
    }
    Some((y2 - y1) as f64 * 360.0 + (m2 as f64 - m1 as f64) * 30.0 + (d2 as f64 - d1 as f64))
}

/// The **days** between two dates under a basis.
pub(crate) fn day_count(system: System, first: f64, second: f64, basis: u32) -> Option<f64> {
    match basis {
        0 => days360(system, first, second, true),
        4 => days360(system, first, second, false),
        _ => Some(second - first),
    }
}

/// The **days in a year** for discount securities.
pub(crate) fn year_basis(system: System, basis: u32, settlement: f64) -> Option<f64> {
    Some(match basis {
        0 | 2 | 4 => 360.0,
        3 => 365.0,
        _ => {
            let (year, _, _) = ymd(system, settlement)?;
            if is_leap(year) { 366.0 } else { 365.0 }
        }
    })
}

/// The **fraction of a year** between two dates under a basis.
pub(crate) fn year_frac(system: System, first: f64, second: f64, basis: u32) -> Option<f64> {
    let (first, second) = if first <= second { (first, second) } else { (second, first) };
    match basis {
        0 => Some(days360(system, first, second, true)? / 360.0),
        2 => Some((second - first) / 360.0),
        3 => Some((second - first) / 365.0),
        4 => Some(days360(system, first, second, false)? / 360.0),
        _ => {
            let (y1, m1, d1) = ymd(system, first)?;
            let (y2, m2, d2) = ymd(system, second)?;
            let days = second - first;
            if y1 == y2 {
                return Some(days / if is_leap(y1) { 366.0 } else { 365.0 });
            }
            let within_year = y2 == y1 + 1 && (m2 < m1 || (m2 == m1 && d2 <= d1));
            if within_year {
                let leap_first = is_leap(y1) && (m1 < 2 || (m1 == 2 && d1 <= 29));
                let leap_second = is_leap(y2) && (m2 > 2 || (m2 == 2 && d2 == 29));
                let length = if leap_first || leap_second { 366.0 } else { 365.0 };
                return Some(days / length);
            }
            let start = to_serial(y1, 1, 1, system)?;
            let end = to_serial(y2 + 1, 1, 1, system).unwrap_or(system.max_serial() as f64 + 1.0);
            let average = (end - start) / f64::from(y2 - y1 + 1);
            Some(days / average)
        }
    }
}

/// The date **months** before or after an anchor date, keeping the
/// end of the month when the anchor is the end of the month.
pub(crate) fn shift_months(
    system: System,
    anchor: (i32, u32, u32),
    months: i64,
    end_of_month: bool,
) -> Option<f64> {
    let (year, month, day) = anchor;
    let total = i64::from(year) * 12 + i64::from(month) - 1 + months;
    let next_year = total.div_euclid(12);
    let next_month = (total.rem_euclid(12) + 1) as u32;
    if !(1900..=9999).contains(&next_year) {
        return None;
    }
    let length = days_in_month(next_year as i32, next_month);
    let day = if end_of_month { length } else { day.min(length) };
    to_serial(next_year as i32, next_month, day, system)
}

/// The **basis** a number says, if it says one.
pub(crate) fn basis_of(number: Option<f64>) -> Result<u32, String> {
    let basis = number.unwrap_or(0.0).trunc();
    if !(0.0..=4.0).contains(&basis) {
        return Err("#NUM!".to_string());
    }
    Ok(basis as u32)
}

/// The **frequency** a number says: one, two or four times a year.
pub(crate) fn frequency_of(number: f64) -> Result<f64, String> {
    let frequency = number.trunc();
    if frequency != 1.0 && frequency != 2.0 && frequency != 4.0 {
        return Err("#NUM!".to_string());
    }
    Ok(frequency)
}
