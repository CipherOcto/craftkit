//! The **dates** of a cell.
//!
//! A date in a spreadsheet is a **number** and a **format**: the number
//! says how many days have passed, and the format says that the number
//! is a date and how it is written. The tree's date is not a different
//! kind of value; it is a number with a date format on it.
//!
//! The tree's **1900 system** is here: the serial 1 is the 1st of
//! January 1900, and — like Lotus 1-2-3 and Excel, and for a mistake
//! that is a hundred and twenty-five years old — the tree treats 1900
//! as a **leap year**, so the serial 60 is the **fictitious** 29th of
//! February 1900.
//!
//! The tree's **1904 system** is not here: it is the system of the old
//! Macintosh Excel, and a workbook that says which system it uses is a
//! workbook that was made somewhere else.

/// The months, in full.
pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The days of the week, from Sunday.
pub const WEEKDAYS: [&str; 7] =
    ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// The last serial that is a date: the 31st of December 9999.
///
/// The tree's bound, in the 1900 system.
pub const MAX_SERIAL: f64 = 2_958_465.0;

/// The days from the 31st of December 1899 to the 1st of January 1970.
///
/// The tree's own constant, and it is what makes the serial and the
/// Unix day count one number apart.
const UNIX_1899_12_30: i64 = -25_569;

/// A **calendar date**, and the day of the week it falls on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    /// The year.
    pub year: i32,
    /// The month, from one.
    pub month: u32,
    /// The day of the month, from one.
    pub day: u32,
    /// The day of the week, from zero, and zero is Sunday.
    pub weekday: u32,
}

impl Date {
    /// The date, as `2026-10-26`.
    pub fn to_text(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The date of a **serial**, in the 1900 system.
///
/// The serial 60 is the fictitious 29th of February 1900, and the
/// serial 0 is the day before the first of January 1900 — which the
/// tree writes as the zeroth of January, because a day of a month
/// cannot be zero and the tree does not pretend otherwise.
pub fn from_serial(serial: f64) -> Option<Date> {
    if !serial.is_finite() || !(0.0..MAX_SERIAL + 1.0).contains(&serial) {
        return None;
    }
    let day = serial.floor() as i64;
    // The tree's weekdays follow the **fictitious** calendar: the
    // serial 1 is a Sunday, and every date after it is a day later
    // than the day it really is.
    let weekday = ((day + 6) % 7) as u32;
    let (year, month, dom) = if day == 60 {
        (1900, 2, 29)
    } else if day == 0 {
        (1900, 1, 0)
    } else {
        let real = if day < 60 { day + 1 } else { day };
        let (year, month, dom) = civil_from_days(real + UNIX_1899_12_30);
        (year, month, dom)
    };
    Some(Date { year, month, day: dom, weekday })
}

/// The **serial** of a date, in the 1900 system.
///
/// A date that did not happen is refused: the 30th of February is not
/// a date, and the tree's `DATE()` rolls it over, but a cell that
/// holds a date holds a date that was.
pub fn to_serial(year: i32, month: u32, day: u32) -> Option<f64> {
    if !(1900..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    if day < 1 || day > days_in_month(year, month) {
        return None;
    }
    let first = days_from_civil(year, month, 1);
    // Up to February 1900 the serial is one lower than the day count
    // from the 30th of December 1899: the fictitious 29th of February
    // comes before the 1st of March.
    let first = first - UNIX_1899_12_30 - i64::from(year == 1900 && month <= 2);
    let serial = first + day as i64 - 1;
    if !(0..=MAX_SERIAL as i64).contains(&serial) {
        return None;
    }
    Some(serial as f64)
}

/// How many days a month has.
///
/// February 1900 has **29** days here, because the tree's calendar has
/// a leap day in it that never happened.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    if !(1..=12).contains(&month) {
        return 0;
    }
    if year == 1900 && month == 2 {
        return 29;
    }
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    }
}

/// Whether a year is a **leap year**.
///
/// The Gregorian rule, for every year: a year is a leap year when it
/// is divisible by four, and not by a hundred, unless it is divisible
/// by four hundred.
pub fn is_leap(year: i32) -> bool {
    let year = i64::from(year);
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// A date, from the way a date is **written**.
///
/// The tree's own shapes, and the tree's own order of trying them:
///
/// - `2026-10-26` — the year first, and the year has four digits;
/// - `10/26/2026` — the month first, and the day before the month is
///   **not** read: the tree reads the month first and the day second,
///   and `26/10/2026` is not a date to it;
/// - `4-Mar-2026` — the day, the month by name and the year.
///
/// The tree reads the year of two digits as `2000` and the years below
/// thirty, and as `1900` and the rest. The tree's shape for a date
/// with a month by name is `d-mmm-yy` and a **two** digit year; here
/// the shape is `d-mmm-yyyy` and the year is written in full.
///
/// The answer is the date and the **format code** the tree gives it,
/// because a date in a spreadsheet is a number and a format, and the
/// two go together.
pub fn parse(text: &str) -> Option<(Date, &'static str)> {
    let text = text.trim().trim_end_matches(',');
    let parts: Vec<&str> = text.split(['/', '-', '.']).map(|part| part.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let (first, second, third) = (parts[0], parts[1], parts[2]);
    let number = |text: &str| -> Option<i64> {
        let text = text.trim();
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        text.parse().ok()
    };
    // The year first: `2026-10-26`.
    if first.len() == 4
        && let (Some(year), Some(month), Some(day)) = (number(first), number(second), number(third))
    {
        let date = the_date(year, month, day)?;
        return Some((date, "yyyy-mm-dd"));
    }
    // The month first: `10/26/2026`.
    if let (Some(month), Some(day), Some(year)) = (number(first), number(second), number(third)) {
        let date = the_date(the_year(year, third.len()), month, day)?;
        return Some((date, "m/d/yyyy"));
    }
    // The month by name: `4-Mar-2026`.
    if let (Some(day), Some(month), Some(year)) = (number(first), month_from_name(second), number(third)) {
        let date = the_date(the_year(year, third.len()), month as i64, day)?;
        return Some((date, "d-mmm-yyyy"));
    }
    None
}

/// The year a date is written with, when the year has two digits.
fn the_year(year: i64, digits: usize) -> i64 {
    if digits <= 2 { if year < 30 { 2000 + year } else { 1900 + year } } else { year }
}

/// The month a name says, if it says one.
fn month_from_name(text: &str) -> Option<u32> {
    let text = text.to_ascii_lowercase();
    if text.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|month| month.to_ascii_lowercase().starts_with(&text))
        .map(|index| index as u32 + 1)
}

/// The date a year, a month and a day say — if they say a day that
/// happened.
fn the_date(year: i64, month: i64, day: i64) -> Option<Date> {
    if !(1900..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    if day < 1 || day > days_in_month(year as i32, month as u32) as i64 {
        return None;
    }
    let serial = to_serial(year as i32, month as u32, day as u32)?;
    from_serial(serial)
}

/// The days from the 1st of January 1970 to a date.
///
/// The Gregorian calendar, for every year: the calendar repeats every
/// four hundred years, and the count is exact for all of them.
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year);
    let month = i64::from(month);
    let day = i64::from(day);
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_of_year = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_of_year + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// A date, from the days since the 1st of January 1970.
///
/// The inverse of [`days_from_civil`], and exact for every day.
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = (days - era * 146_097) as u64;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_of_year = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_of_year + 2) / 5 + 1) as u32;
    let month = if month_of_year < 10 { month_of_year + 3 } else { month_of_year - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year as i32, month, day)
}
