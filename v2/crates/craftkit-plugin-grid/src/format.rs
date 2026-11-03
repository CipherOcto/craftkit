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
//!   hundred;
//! - the **currencies**: a number with a symbol in quotes, such as
//!   `"$"#,##0.00` or `#,##0.00" kr"`;
//! - the **dates**: `yyyy-mm-dd`, `dd/mm/yyyy`, `m/d/yyyy`,
//!   `d-mmm-yyyy`, `mmm`, `mmmm`, `ddd`, `dddd` and `yyyy`.
//! - the **times**: `h`, `hh`, `h:mm`, `hh:mm`, `h:mm:ss`, `hh:mm:ss`,
//!   `h AM/PM`, `h:mm AM/PM`, `h:mm:ss AM/PM`, `hh:mm:ss AM/PM`,
//!   `[h]:mm`, `[h]:mm:ss` and `[mm]:ss`.
//!
//! The **number** formats are a **grammar** and not a list: the
//! thousands separated by a comma, a whole part of `0` and `#`
//! placeholders with at least one `0` in it, a point and up to fifteen
//! decimals, and a percent sign at the end. So `0`, `0.00`, `#,##0`,
//! `0.0000` and `0%` are all codes this tree has.
//!
//! The **date** formats are a **list** of nine, because the tree's
//! date formats are a language and this one has nine shapes of it.
//!
//! A code that is neither a number format of that shape nor one of the
//! nine dates is **refused**: a format that is quietly ignored is a
//! format the user believes in and the program does not have.
//!
//! A **date** is a number with one of those on it: the number says
//! how many days have passed and the format says that the number is a
//! date and how it is written (the module `dates`).
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
//! - **The tree's scientific formats are not here**, and neither are
//!   its **sections** — a format with a different code for negative
//!   numbers is not here. The tree's **currency** is here: a number
//!   with a symbol on it, and the symbol is a literal in quotes. The
//!   tree's **times** are here: a time is a fraction of a day and a
//!   format, and the tree's clock shapes and its elapsed shapes are
//!   both here.
//! - **The tree's date formats are a language, and here they are a
//!   list**: the nine shapes above are the ones this tree has, and a
//!   date format that is not one of them is refused.

use crate::dates::{self, Date};
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
    /// A **date**: a number that is written as a calendar date.
    Date {
        /// The shape the date is written in.
        style: DateStyle,
    },
    /// A **time**: a number that is written as a time of day.
    Time {
        /// The shape the time is written in.
        style: TimeStyle,
    },
    /// A **currency**: a number with a **symbol** on it.
    Currency {
        /// The symbol: the tree writes it in quotes.
        symbol: String,
        /// Whether the symbol is in **front** of the number or behind
        /// it.
        before: bool,
        /// How many decimals are written.
        decimals: usize,
        /// Whether the thousands are separated by a comma.
        thousands: bool,
    },
}

/// The **time formats** this tree knows.
///
/// The tree's format is a whole language, and a time in it is a code
/// with an `h` and a `m` and a `s` in it and anything else the tree
/// likes. Here a time format is one of a **list**, and the list is
/// this one — the clock's shapes and the tree's own way of writing
/// each of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeStyle {
    /// `h` — the hour, on a clock of twenty-four.
    Hour,
    /// `hh` — the hour, with two digits.
    Hour2,
    /// `h:mm` — the hour and the minute.
    HourMinute,
    /// `hh:mm` — and the hour with two digits.
    Hour2Minute,
    /// `h:mm:ss` — and the second.
    HourMinuteSecond,
    /// `hh:mm:ss` — and the hour with two digits.
    Hour2MinuteSecond,
    /// `h AM/PM` — the hour on a clock of twelve.
    Hour12,
    /// `h:mm AM/PM` — and the minute.
    Hour12Minute,
    /// `h:mm:ss AM/PM` — and the second.
    Hour12MinuteSecond,
    /// `hh:mm:ss AM/PM` — and the hour with two digits.
    Hour2MinuteSecond12,
    /// `[h]:mm` — the **elapsed** hours and minutes.
    ElapsedHourMinute,
    /// `[h]:mm:ss` — and the seconds.
    ElapsedHourMinuteSecond,
    /// `[mm]:ss` — the elapsed minutes and seconds.
    ElapsedMinuteSecond,
}

impl TimeStyle {
    /// Every time format, and the code that says it.
    pub const ALL: [(TimeStyle, &'static str); 13] = [
        (TimeStyle::Hour, "h"),
        (TimeStyle::Hour2, "hh"),
        (TimeStyle::HourMinute, "h:mm"),
        (TimeStyle::Hour2Minute, "hh:mm"),
        (TimeStyle::HourMinuteSecond, "h:mm:ss"),
        (TimeStyle::Hour2MinuteSecond, "hh:mm:ss"),
        (TimeStyle::Hour12, "h AM/PM"),
        (TimeStyle::Hour12Minute, "h:mm AM/PM"),
        (TimeStyle::Hour12MinuteSecond, "h:mm:ss AM/PM"),
        (TimeStyle::Hour2MinuteSecond12, "hh:mm:ss AM/PM"),
        (TimeStyle::ElapsedHourMinute, "[h]:mm"),
        (TimeStyle::ElapsedHourMinuteSecond, "[h]:mm:ss"),
        (TimeStyle::ElapsedMinuteSecond, "[mm]:ss"),
    ];

    /// The style a code says, if it says one this tree knows.
    pub fn parse(code: &str) -> Option<TimeStyle> {
        let code = code.trim();
        TimeStyle::ALL.iter().find(|(_, written)| written.eq_ignore_ascii_case(code)).map(|(style, _)| *style)
    }

    /// The code the style is written with.
    pub fn code(self) -> &'static str {
        TimeStyle::ALL.iter().find(|(style, _)| *style == self).map(|(_, code)| *code).unwrap_or("h:mm")
    }

    /// The **text** a number is written with, as a time.
    ///
    /// The number is a serial: its whole part is the day and its rest
    /// is the time of day. The **clock** shapes look at the time of day
    /// and the **elapsed** shapes look at the whole number — an elapsed
    /// hour is an hour of the whole number, and it does not stop at
    /// twenty-four.
    pub fn text(self, serial: f64) -> String {
        let Some((day, ms)) = dates::day_and_ms(serial) else {
            return String::new();
        };
        let hour = ms / 3_600_000;
        let minute = ms / 60_000 % 60;
        let second = ms / 1000 % 60;
        let twelve = |hour: i64| {
            let hour = hour % 12;
            if hour == 0 { 12 } else { hour }
        };
        let half = |hour: i64| if hour >= 12 { "PM" } else { "AM" };
        let total = day.saturating_mul(86_400_000).saturating_add(ms);
        match self {
            TimeStyle::Hour => format!("{hour}"),
            TimeStyle::Hour2 => format!("{hour:02}"),
            TimeStyle::HourMinute => format!("{hour}:{minute:02}"),
            TimeStyle::Hour2Minute => format!("{hour:02}:{minute:02}"),
            TimeStyle::HourMinuteSecond => format!("{hour}:{minute:02}:{second:02}"),
            TimeStyle::Hour2MinuteSecond => format!("{hour:02}:{minute:02}:{second:02}"),
            TimeStyle::Hour12 => format!("{} {}", twelve(hour), half(hour)),
            TimeStyle::Hour12Minute => format!("{}:{:02} {}", twelve(hour), minute, half(hour)),
            TimeStyle::Hour12MinuteSecond => {
                format!("{}:{:02}:{:02} {}", twelve(hour), minute, second, half(hour))
            }
            TimeStyle::Hour2MinuteSecond12 => {
                format!("{:02}:{:02}:{:02} {}", twelve(hour), minute, second, half(hour))
            }
            TimeStyle::ElapsedHourMinute => {
                format!("{}:{:02}", total / 3_600_000, total / 60_000 % 60)
            }
            TimeStyle::ElapsedHourMinuteSecond => {
                format!("{}:{:02}:{:02}", total / 3_600_000, total / 60_000 % 60, total / 1000 % 60)
            }
            TimeStyle::ElapsedMinuteSecond => format!("{}:{:02}", total / 60_000, total / 1000 % 60),
        }
    }
}

/// The **date formats** this tree knows.
///
/// The tree's format is a whole language, and a date in it is a code
/// with a `y` and a `d` and a `m` in it and anything else the tree
/// likes. Here a date format is one of a **list**, and the list is
/// this one — nine shapes, and the tree's own way of writing each of
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateStyle {
    /// `yyyy-mm-dd` — the year, the month and the day, and the year
    /// first.
    Iso,
    /// `dd/mm/yyyy` — the day first.
    DayFirst,
    /// `m/d/yyyy` — the month first, and the day and the month
    /// without a zero in front of them.
    MonthFirst,
    /// `d-mmm-yyyy` — the day, the month by name, the year.
    DayMonthName,
    /// `mmm` — the month by name, in three letters.
    MonthShort,
    /// `mmmm` — the month by name, in full.
    MonthFull,
    /// `ddd` — the day of the week, in three letters.
    WeekdayShort,
    /// `dddd` — the day of the week, in full.
    WeekdayFull,
    /// `yyyy` — the year, and nothing else.
    Year,
}

impl DateStyle {
    /// Every date format, and the code that says it.
    pub const ALL: [(DateStyle, &'static str); 9] = [
        (DateStyle::Iso, "yyyy-mm-dd"),
        (DateStyle::DayFirst, "dd/mm/yyyy"),
        (DateStyle::MonthFirst, "m/d/yyyy"),
        (DateStyle::DayMonthName, "d-mmm-yyyy"),
        (DateStyle::MonthShort, "mmm"),
        (DateStyle::MonthFull, "mmmm"),
        (DateStyle::WeekdayShort, "ddd"),
        (DateStyle::WeekdayFull, "dddd"),
        (DateStyle::Year, "yyyy"),
    ];

    /// The style a code says, if it says one this tree knows.
    pub fn parse(code: &str) -> Option<DateStyle> {
        let code = code.trim();
        DateStyle::ALL.iter().find(|(_, written)| written.eq_ignore_ascii_case(code)).map(|(style, _)| *style)
    }

    /// The code the style is written with.
    pub fn code(self) -> &'static str {
        DateStyle::ALL.iter().find(|(style, _)| *style == self).map(|(_, code)| *code).unwrap_or("yyyy-mm-dd")
    }

    /// The **text** a date is written with.
    pub fn text(self, date: Date) -> String {
        match self {
            DateStyle::Iso => format!("{:04}-{:02}-{:02}", date.year, date.month, date.day),
            DateStyle::DayFirst => format!("{:02}/{:02}/{:04}", date.day, date.month, date.year),
            DateStyle::MonthFirst => {
                format!("{}/{}/{:04}", date.month, date.day, date.year)
            }
            DateStyle::DayMonthName => {
                let month: String = dates::MONTHS[month_index(date)].chars().take(3).collect();
                format!("{}-{}-{:04}", date.day, month, date.year)
            }
            DateStyle::MonthShort => dates::MONTHS[month_index(date)].chars().take(3).collect(),
            DateStyle::MonthFull => dates::MONTHS[month_index(date)].to_string(),
            DateStyle::WeekdayShort => dates::WEEKDAYS[weekday_index(date)].chars().take(3).collect(),
            DateStyle::WeekdayFull => dates::WEEKDAYS[weekday_index(date)].to_string(),
            DateStyle::Year => format!("{:04}", date.year),
        }
    }
}

/// The index of a month in the list of months.
fn month_index(date: Date) -> usize {
    (date.month.clamp(1, 12) - 1) as usize
}

/// The index of a day of the week in the list of days.
fn weekday_index(date: Date) -> usize {
    (date.weekday % 7) as usize
}

impl Format {
    /// The format a code says, if it says one this tree knows.
    pub fn parse(code: &str) -> Option<Format> {
        let code = code.trim();
        if code.is_empty() || code.eq_ignore_ascii_case("general") {
            return Some(Format::General);
        }
        // A **date** is a shape of its own, and it is asked for before
        // the number: `yyyy` is not a number with two decimals. A
        // **time** is a shape of its own too.
        if let Some(style) = DateStyle::parse(code) {
            return Some(Format::Date { style });
        }
        if let Some(style) = TimeStyle::parse(code) {
            return Some(Format::Time { style });
        }
        // A **currency** is a number with a **symbol**: the symbol is a
        // literal in quotes, and the tree writes it that way.
        for (symbol, before, rest) in symbols(code) {
            if let Some((decimals, thousands)) = number_shape(&rest) {
                return Some(Format::Currency { symbol, before, decimals, thousands });
            }
        }
        let (percent, rest) = match code.strip_suffix('%') {
            Some(rest) => (true, rest.to_string()),
            None => (false, code.to_string()),
        };
        let (decimals, thousands) = number_shape(&rest)?;
        Some(Format::Decimals { decimals, thousands, percent })
    }

    /// The **text** a value is written with.
    pub fn text(&self, value: &Value, system: dates::System) -> String {
        match value {
            Value::Empty => String::new(),
            Value::Number(number) => match self {
                Format::General => general(*number),
                Format::Decimals { decimals, thousands, percent } => {
                    fixed(*number, *decimals, *thousands, *percent)
                }
                // A date is a **number** with a date format on it, and
                // a number that is not a date is written as a number.
                Format::Date { style } => match dates::from_serial(*number, system) {
                    Some(date) => style.text(date),
                    None => general(*number),
                },
                // A time is a **number** with a time format on it, and
                // the number that is not a date is a time of the day.
                Format::Time { style } => style.text(*number),
                // A currency is a **number** with a symbol on it.
                Format::Currency { symbol, before, decimals, thousands } => {
                    currency(*number, symbol, *before, *decimals, *thousands)
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

/// The **symbols** of a currency, and the number format that is left.
///
/// The tree writes the symbol in **quotes**, because a symbol is a
/// literal and not a placeholder: `"$"#,##0.00`. The symbol may be in
/// front of the number or behind it, and a code that has both readings
/// is read the way that leaves a number behind.
fn symbols(code: &str) -> Vec<(String, bool, String)> {
    let code = code.trim();
    let mut out: Vec<(String, bool, String)> = Vec::new();
    if let Some(rest) = code.strip_prefix('"')
        && let Some((symbol, rest)) = rest.split_once('"')
    {
        out.push((symbol.to_string(), true, rest.trim().to_string()));
    }
    if let Some(rest) = code.strip_suffix('"')
        && let Some((rest, symbol)) = rest.rsplit_once('"')
    {
        out.push((symbol.to_string(), false, rest.trim().to_string()));
    }
    out
}

/// The **shape** of a number format: how many decimals it writes and
/// whether the thousands are separated.
///
/// The whole part is a placeholder: `0` always shows a digit and `#`
/// leaves it out when it is zero.
fn number_shape(code: &str) -> Option<(usize, bool)> {
    let (thousands, rest) = match code.contains(',') {
        true => (true, code.replace(',', "")),
        false => (false, code.to_string()),
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
    if whole.is_empty() || !whole.chars().all(|digit| digit == '#' || digit == '0') || !whole.contains('0') {
        return None;
    }
    Some((decimals, thousands))
}

/// A number with a **symbol** in front of it.
///
/// The sign is in front of the symbol, as the tree writes it.
fn currency(number: f64, symbol: &str, before: bool, decimals: usize, thousands: bool) -> String {
    if !number.is_finite() {
        return general(number);
    }
    let sign = if number < 0.0 { "-" } else { "" };
    let written = fixed(number.abs(), decimals, thousands, false);
    if before { format!("{sign}{symbol}{written}") } else { format!("{sign}{written}{symbol}") }
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
