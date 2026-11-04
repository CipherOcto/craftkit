//! The **parts** of a date.
//!
//! A formula may make a date and may take a date apart:
//!
//! - `DATE(year, month, day)` — the **serial** of the date, in the
//!   date system of the workbook;
//! - `YEAR(serial)`, `MONTH(serial)` and `DAY(serial)` — the parts of
//!   the date the serial is.
//!
//! # The rollover
//!
//! The tree's `DATE` **rolls over**: the month may be outside the year
//! and the day outside the month, and both roll — the 13th month is
//! the January of the next year, and the 32nd day of January is the
//! 1st of February. And a year **below 1900** is a year after 1900:
//! the year 24 is the year 1924.
//!
//! So does this one.
//!
//! # The serial
//!
//! A serial is a number: the days since the beginning of the
//! calendar of the workbook. The 1st of January 1900 is the serial 1
//! in the 1900 system, and the 1st of January 1904 is the serial 0 in
//! the 1904 system. The **fictitious** 29th of February 1900 is the
//! serial 60, because the tree's calendar has a leap day in it that
//! never happened.
//!
//! A serial that is outside the calendar is an error, and so is a
//! date that the calendar does not have.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **date functions**.
pub fn is_date_function(name: &str) -> bool {
    matches!(
        name,
        "DATE"
            | "YEAR"
            | "MONTH"
            | "DAY"
            | "TIME"
            | "HOUR"
            | "MINUTE"
            | "SECOND"
            | "WEEKDAY"
            | "EDATE"
            | "EOMONTH"
            | "DAYS"
    )
}

impl Book {
    /// The **date functions**.
    pub(crate) fn dates(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "DATE" => self.date_call(sheet, arguments, being),
            "YEAR" | "MONTH" | "DAY" => self.part_call(sheet, name, arguments, being),
            "TIME" => self.time_call(sheet, arguments, being),
            "HOUR" | "MINUTE" | "SECOND" => self.clock_call(sheet, name, arguments, being),
            "WEEKDAY" => self.weekday_call(sheet, arguments, being),
            "EDATE" | "EOMONTH" => self.month_call(sheet, name, arguments, being),
            "DAYS" => self.days_call(sheet, arguments, being),
            _ => Value::Error(format!("there is no function `{name}`")),
        }
    }

    /// The tree's `DATE`: the **serial** of a date.
    fn date_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let mut numbers: Vec<i64> = Vec::new();
        for argument in arguments {
            let value = self.evaluate(sheet, argument, being);
            let Some(number) = value.as_number() else {
                return Value::Error("#VALUE!".to_string());
            };
            if !number.is_finite() {
                return Value::Error("#NUM!".to_string());
            }
            numbers.push(number.trunc() as i64);
        }
        let (year, month, day) = (numbers[0], numbers[1], numbers[2]);
        // The tree's bounds: a year is below ten thousand, and the
        // month and the day are not absurd.
        if !(0..10000).contains(&year) || month.abs() > 1_000_000 || day.abs() > 100_000_000 {
            return Value::Error("#NUM!".to_string());
        }
        match crate::dates::to_serial_rolled(year, month, day, self.system()) {
            Some(serial) => Value::Number(serial),
            None => Value::Error("#NUM!".to_string()),
        }
    }

    /// The tree's `YEAR`, `MONTH` and `DAY`: the **parts** of a date.
    fn part_call(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if arguments.len() != 1 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        let Some(serial) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if !serial.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        let Some(date) = crate::dates::from_serial(serial.floor(), self.system()) else {
            return Value::Error("#NUM!".to_string());
        };
        let part = match name {
            "YEAR" => date.year as f64,
            "MONTH" => date.month as f64,
            _ => date.day as f64,
        };
        Value::Number(part)
    }

    /// The tree's `TIME`: the **fraction of a day** a time is.
    ///
    /// The tree's rule: the three numbers are cut to whole ones, and
    /// a number over thirty-two thousand seven hundred and sixty-seven
    /// is an error, and the time may not be before the midnight — and
    /// a time that is more than a day is a day and a bit.
    fn time_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let mut numbers: Vec<f64> = Vec::new();
        for argument in arguments {
            let value = self.evaluate(sheet, argument, being);
            let Some(number) = value.as_number() else {
                return Value::Error("#VALUE!".to_string());
            };
            if !number.is_finite() {
                return Value::Error("#NUM!".to_string());
            }
            numbers.push(number.trunc());
        }
        if numbers.iter().any(|number| *number > 32_767.0) {
            return Value::Error("#NUM!".to_string());
        }
        let total = numbers[0] * 3600.0 + numbers[1] * 60.0 + numbers[2];
        if total < 0.0 {
            return Value::Error("#NUM!".to_string());
        }
        Value::Number(total.rem_euclid(86_400.0) / 86_400.0)
    }

    /// The tree's `HOUR`, `MINUTE` and `SECOND`: the parts of a time.
    fn clock_call(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if arguments.len() != 1 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        let Some(serial) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if !serial.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        let Some((_day, milliseconds)) = crate::dates::day_and_ms(serial) else {
            return Value::Error("#NUM!".to_string());
        };
        let part = match name {
            "HOUR" => milliseconds / 3_600_000,
            "MINUTE" => (milliseconds / 60_000) % 60,
            _ => (milliseconds / 1_000) % 60,
        };
        Value::Number(part as f64)
    }

    /// The tree's `WEEKDAY`: the day of the week of a date, as a
    /// number.
    ///
    /// The tree's rule: the number is one of the tree's own, and it
    /// says where the week begins — one is a week that begins on the
    /// Sunday, two is a week that begins on the Monday, three is a
    /// week that begins on the Monday and counts from zero, and the
    /// eleven to seventeen are the weeks that begin on a day and count
    /// from one.
    fn weekday_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.is_empty() || arguments.len() > 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        let Some(serial) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if !serial.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        let kind = match arguments.get(1) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                match value.as_number() {
                    Some(number) if number.is_finite() => number.trunc() as i64,
                    _ => return Value::Error("#NUM!".to_string()),
                }
            }
            None => 1,
        };
        let Some(date) = crate::dates::from_serial(serial.floor(), self.system()) else {
            return Value::Error("#NUM!".to_string());
        };
        // The day of the week, from zero, and zero is the Sunday.
        let weekday = i64::from(date.weekday);
        let result = match kind {
            1 | 17 => weekday + 1,
            2 | 11 => (weekday + 6) % 7 + 1,
            3 => (weekday + 6) % 7,
            12..=16 => (weekday + 7 - (kind - 10)) % 7 + 1,
            _ => return Value::Error("#NUM!".to_string()),
        };
        Value::Number(result as f64)
    }

    /// The tree's `EDATE` and `EOMONTH`: a date a number of months
    /// away.
    ///
    /// The day of the month is **clamped** to the length of the month
    /// it lands in — the 31st of January plus a month is the 28th of
    /// February — and the months are always counted from the
    /// **original** date, so a series does not drift.
    ///
    /// And the time of the day is **dropped**: the tree's `EDATE` is a
    /// date and not a moment.
    fn month_call(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if arguments.len() != 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        let Some(serial) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        let Some(date) = crate::dates::from_serial(serial.floor(), self.system()) else {
            return Value::Error("#NUM!".to_string());
        };
        let value = self.evaluate(sheet, &arguments[1], being);
        let Some(months) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if !months.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        let months = months.trunc() as i64;
        let total = i64::from(date.year) * 12 + i64::from(date.month) - 1 + months;
        let year = total.div_euclid(12);
        let month = total.rem_euclid(12) + 1;
        if !(1900..=9999).contains(&year) {
            return Value::Error("#NUM!".to_string());
        }
        let month = u32::try_from(month).unwrap_or(1);
        let year = i32::try_from(year).unwrap_or(1900);
        let day = match name {
            "EOMONTH" => crate::dates::days_in_month(year, month),
            _ => {
                u32::try_from(i64::from(date.day)).unwrap_or(1).min(crate::dates::days_in_month(year, month))
            }
        };
        match crate::dates::to_serial_rolled(i64::from(year), i64::from(month), i64::from(day), self.system())
        {
            Some(serial) => Value::Number(serial),
            None => Value::Error("#NUM!".to_string()),
        }
    }

    /// The tree's `DAYS`: the number of days between two dates.
    fn days_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let mut serials: Vec<f64> = Vec::new();
        for argument in arguments {
            let value = self.evaluate(sheet, argument, being);
            let Some(serial) = value.as_number() else {
                return Value::Error("#VALUE!".to_string());
            };
            if !serial.is_finite() {
                return Value::Error("#NUM!".to_string());
            }
            serials.push(serial);
        }
        Value::Number(serials[0].floor() - serials[1].floor())
    }
}
