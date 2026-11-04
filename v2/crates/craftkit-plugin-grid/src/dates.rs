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
//! The tree's **1904 system** is here too: it is the system of the old
//! Macintosh Excel, and it counts from the 1st of January 1904, so it
//! has no day in it that never happened. Which system a workbook uses
//! is a fact of the **workbook** — as it is in the tree — and every
//! date in every sheet of it is read in that calendar.

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

/// The days from the 1st of January 1904 to the 1st of January 1970.
///
/// The tree's own constant, and it is what makes the 1904 serial and
/// the Unix day count one number apart.
const UNIX_1904_01_01: i64 = -24_107;

/// The **date system** of a workbook: the calendar its serials are
/// counted in.
///
/// The tree's two systems, and they are the tree's own:
///
/// - the **1900** system counts from the 31st of December 1899, so the
///   serial 1 is the 1st of January 1900 — and it has a day in it
///   that never happened, the 29th of February of 1900, which is the
///   serial 60;
/// - the **1904** system counts from the 1st of January 1904, so the
///   serial 0 is that day, and there is no day in it that never
///   happened.
///
/// The two are **1462 days** apart: the same date is the serial 1462
/// higher in the 1900 system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum System {
    /// The 1900 system, with the day in it that never happened.
    #[default]
    Y1900,
    /// The 1904 system, without one.
    Y1904,
}

impl System {
    /// The system a name says, if it says one this tree knows.
    pub fn parse(name: &str) -> Option<System> {
        match name.trim() {
            "1900" => Some(System::Y1900),
            "1904" => Some(System::Y1904),
            _ => None,
        }
    }

    /// The name of the system.
    pub fn name(self) -> &'static str {
        match self {
            System::Y1900 => "1900",
            System::Y1904 => "1904",
        }
    }

    /// The **largest** serial the system has.
    pub fn max_serial(self) -> i64 {
        match self {
            System::Y1900 => MAX_SERIAL as i64,
            System::Y1904 => MAX_SERIAL as i64 - 1462,
        }
    }
}

/// A **time of day**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    /// The hour, from zero to twenty-three.
    pub hour: u32,
    /// The minute.
    pub minute: u32,
    /// The second.
    pub second: u32,
}

impl Time {
    /// The time, as `13:45:00`.
    pub fn to_text(self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// The **fraction of a day** a time is.
///
/// The tree's own rule: a time is a part of a day and not a thing of
/// its own, so a cell that holds a time holds a number.
pub fn time_fraction(hour: u32, minute: u32, second: u32) -> f64 {
    (f64::from(hour) * 3600.0 + f64::from(minute) * 60.0 + f64::from(second)) / 86_400.0
}

/// The **day and the milliseconds** of a serial.
///
/// The tree's own split: the day is the whole part and the milliseconds
/// are the rest, rounded to the nearest **second** — and a second that
/// rounds to the next one carries into the day.
pub fn day_and_ms(serial: f64) -> Option<(i64, i64)> {
    if !serial.is_finite() {
        return None;
    }
    let mut day = serial.floor() as i64;
    let mut ms = ((serial - serial.floor()) * 86_400.000).round() as i64 * 1000;
    if ms >= 86_400_000 {
        ms -= 86_400_000;
        day += 1;
    }
    Some((day, ms))
}

/// A time, from the way a time is **written**.
///
/// The tree's own shapes, and the tree's own order of trying them:
///
/// - `13:45` — the hour and the minute;
/// - `13:45:30` — and the second;
/// - `1:45 pm` — on a clock of twelve, and the afternoon is the half
///   of the day after the noon.
///
/// The answer is the time and the **format code** the tree gives it,
/// because a time in a spreadsheet is a number and a format, and the
/// two go together.
pub fn parse_time(text: &str) -> Option<(Time, &'static str)> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    let (body, afternoon) = if let Some(body) = lower.strip_suffix("am").or_else(|| lower.strip_suffix('a')) {
        (body.trim().to_string(), Some(false))
    } else if let Some(body) = lower.strip_suffix("pm").or_else(|| lower.strip_suffix('p')) {
        (body.trim().to_string(), Some(true))
    } else {
        (lower.clone(), None)
    };
    let parts: Vec<&str> = body.split(':').map(|part| part.trim()).collect();
    // A lone number is not a time, unless it says which half of the
    // day it is in.
    if parts.len() == 1 && afternoon.is_none() {
        return None;
    }
    if parts.len() > 3
        || parts.iter().any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    let number = |text: &str| -> Option<u32> { text.parse::<u32>().ok() };
    let mut hour = number(parts.first()?)?;
    let minute = match parts.get(1) {
        Some(part) => number(part)?,
        None => 0,
    };
    let second = match parts.get(2) {
        Some(part) => number(part)?,
        None => 0,
    };
    if minute > 59 || second > 59 {
        return None;
    }
    match afternoon {
        Some(false) | Some(true) => {
            // On a clock of twelve the noon is the twelve, and the
            // midnight is the zero.
            if !(1..=12).contains(&hour) {
                return None;
            }
            if hour == 12 {
                hour = 0;
            }
            if afternoon == Some(true) {
                hour += 12;
            }
        }
        None => {
            if hour > 23 {
                return None;
            }
        }
    }
    let time = Time { hour, minute, second };
    let code = match (parts.len() == 3, afternoon.is_some()) {
        (true, true) => "h:mm:ss AM/PM",
        (true, false) => "h:mm:ss",
        (false, true) => "h:mm AM/PM",
        (false, false) => "h:mm",
    };
    Some((time, code))
}

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

/// The date of a **serial**, in the system it is counted in.
///
/// In the 1900 system the serial 60 is the fictitious 29th of February
/// 1900, and the serial 0 is the day before the first of January 1900
/// — which the tree writes as the zeroth of January, because a day of
/// a month cannot be zero and the tree does not pretend otherwise.
///
/// In the 1904 system the serial 0 is the 1st of January 1904 and
/// every day is a day that happened.
pub fn from_serial(serial: f64, system: System) -> Option<Date> {
    let max = system.max_serial();
    if !serial.is_finite() || !(0.0..max as f64 + 1.0).contains(&serial) {
        return None;
    }
    let day = serial.floor() as i64;
    let (year, month, dom, weekday) = match system {
        System::Y1900 => {
            // The tree's weekdays follow the **fictitious** calendar:
            // the serial 1 is a Sunday, and every date after it is a
            // day later than the day it really is.
            let weekday = ((day + 6) % 7) as u32;
            if day == 60 {
                (1900, 2, 29, weekday)
            } else if day == 0 {
                (1900, 1, 0, weekday)
            } else {
                let real = if day < 60 { day + 1 } else { day };
                let (year, month, dom) = civil_from_days(real + UNIX_1899_12_30);
                (year, month, dom, weekday)
            }
        }
        System::Y1904 => {
            let unix = day + UNIX_1904_01_01;
            let (year, month, dom) = civil_from_days(unix);
            (year, month, dom, ((unix + 4).rem_euclid(7)) as u32)
        }
    };
    Some(Date { year, month, day: dom, weekday })
}

/// The **serial** of a date, in the system it is counted in.
///
/// A date that did not happen is refused: the 30th of February is not
/// a date, and the tree's `DATE()` rolls it over, but a cell that
/// holds a date holds a date that was.
///
/// And a date before the system's own calendar is refused: the 1904
/// system begins at the 1st of January 1904, and the 1900 system at
/// the 1st of January 1900.
pub fn to_serial(year: i32, month: u32, day: u32, system: System) -> Option<f64> {
    let first_year = match system {
        System::Y1900 => 1900,
        System::Y1904 => 1904,
    };
    if !(first_year..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    if day < 1 || day > days_in_month(year, month) {
        return None;
    }
    let first = days_from_civil(year, month, 1);
    let first = match system {
        // Up to February 1900 the serial is one lower than the day
        // count from the 30th of December 1899: the fictitious 29th of
        // February comes before the 1st of March.
        System::Y1900 => first - UNIX_1899_12_30 - i64::from(year == 1900 && month <= 2),
        System::Y1904 => first - UNIX_1904_01_01,
    };
    let serial = first + day as i64 - 1;
    if !(0..=system.max_serial()).contains(&serial) {
        return None;
    }
    Some(serial as f64)
}

/// The **serial** of a date that is written with a **rollover**.
///
/// The tree's `DATE`: the month may be outside the year and the day
/// outside the month, and both roll over — the 13th month is the
/// January of the next year, and the 32nd day of January is the 1st
/// of February. And a year below 1900 is a year **after** 1900: the
/// year 24 is the year 1924.
///
/// A date that is outside the calendar of the system is refused.
pub fn to_serial_rolled(year: i64, month: i64, day: i64, system: System) -> Option<f64> {
    let year = if (0..1900).contains(&year) { year + 1900 } else { year };
    let months = month.checked_sub(1)?;
    let year = year.checked_add(months.div_euclid(12))?;
    let month = months.rem_euclid(12) + 1;
    let first_year = match system {
        System::Y1900 => 1900,
        System::Y1904 => 1904,
    };
    if !(first_year..=9999).contains(&year) {
        return None;
    }
    let month = u32::try_from(month).ok()?;
    let year = i32::try_from(year).ok()?;
    let first = days_from_civil(year, month, 1);
    let first = match system {
        System::Y1900 => first - UNIX_1899_12_30 - i64::from(year == 1900 && month <= 2),
        System::Y1904 => first - UNIX_1904_01_01,
    };
    let serial = first.checked_add(day)?.checked_sub(1)?;
    if !(0..=system.max_serial()).contains(&serial) {
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
pub fn parse(text: &str, system: System) -> Option<(Date, &'static str)> {
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
        let date = the_date(year, month, day, system)?;
        return Some((date, "yyyy-mm-dd"));
    }
    // The month first: `10/26/2026`.
    if let (Some(month), Some(day), Some(year)) = (number(first), number(second), number(third)) {
        let date = the_date(the_year(year, third.len()), month, day, system)?;
        return Some((date, "m/d/yyyy"));
    }
    // The month by name: `4-Mar-2026`.
    if let (Some(day), Some(month), Some(year)) = (number(first), month_from_name(second), number(third)) {
        let date = the_date(the_year(year, third.len()), month as i64, day, system)?;
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
fn the_date(year: i64, month: i64, day: i64, system: System) -> Option<Date> {
    let first_year = match system {
        System::Y1900 => 1900,
        System::Y1904 => 1904,
    };
    if !(first_year..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    if day < 1 || day > days_in_month(year as i32, month as u32) as i64 {
        return None;
    }
    let serial = to_serial(year as i32, month as u32, day as u32, system)?;
    from_serial(serial, system)
}

/// Whether a serial falls on **Monday to Friday**.
///
/// The Saturday is the sixth day and the Sunday is the zeroth, as the
/// tree has them.
pub fn is_weekday(serial: f64, system: System) -> Option<bool> {
    from_serial(serial, system).map(|date| date.weekday != 0 && date.weekday != 6)
}

/// A serial moved by `n` **weekdays**.
///
/// The Saturday and the Sunday are skipped, and a start that is on one
/// of them moves to the next weekday for the first step. From a
/// weekday every seven days hold five weekdays.
pub fn add_weekdays(serial: f64, n: i64, system: System) -> Option<f64> {
    if n == 0 {
        return Some(serial);
    }
    let step = if n > 0 { 1.0 } else { -1.0 };
    let mut day = serial;
    let next_weekday = |day: &mut f64| -> Option<()> {
        loop {
            *day += step;
            match is_weekday(*day, system) {
                Some(true) => return Some(()),
                Some(false) => {}
                None => return None,
            }
        }
    };
    next_weekday(&mut day)?;
    let rest = n.unsigned_abs() - 1;
    day += step * ((rest / 5) * 7) as f64;
    for _ in 0..rest % 5 {
        next_weekday(&mut day)?;
    }
    from_serial(day, system)?;
    Some(day)
}

/// How many weekdays lie after `a` up to and including `b`.
///
/// The count is negative when `b` comes first.
pub fn weekdays_between(a: f64, b: f64, system: System) -> Option<i64> {
    let (low, high, sign) = if b >= a { (a.floor(), b.floor(), 1) } else { (b.floor(), a.floor(), -1) };
    let days = (high - low) as i64;
    let mut count = days / 7 * 5;
    let mut day = low + (days / 7 * 7) as f64;
    for _ in 0..days % 7 {
        day += 1.0;
        if is_weekday(day, system)? {
            count += 1;
        }
    }
    Some(count * sign)
}

/// A serial moved by `months` **calendar months**.
///
/// The day of the month is kept and **clamped** to the length of the
/// month it lands in — the 31st of January plus a month is the 28th of
/// February — and the months are always counted from the **original**
/// date, so a series does not drift: the 31st of January, the 28th of
/// February, the 31st of March, the 30th of April.
pub fn add_months(serial: f64, months: i64, system: System) -> Option<f64> {
    let date = from_serial(serial, system)?;
    let total =
        i64::from(date.year).checked_mul(12)?.checked_add(i64::from(date.month) - 1)?.checked_add(months)?;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) + 1;
    let year = i32::try_from(year).ok()?;
    let last = days_in_month(year, month as u32);
    let day = i64::from(date.day).min(i64::from(last));
    let base = to_serial(year, month as u32, day as u32, system)?;
    Some(base + serial - serial.floor())
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
