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
//!
//! # The days between two dates
//!
//! Two functions ask how far apart two dates are:
//!
//! - `DAYS360(start, end, [method])` — the **days** under the year of
//!   three hundred and sixty days;
//! - `DATEDIF(start, end, unit)` — the **time** between them, in the
//!   unit that is asked for.
//!
//! The **method** of `DAYS360` is the tree's: the American one, which
//! is the default, and the European one.
//!
//! The **units** of `DATEDIF` are `D` — the days, `M` — the whole
//! months, `Y` — the whole years, `YM` — the whole months that are
//! left of the year, `MD` — the days that are left of the month, and
//! `YD` — the days that are left of the year.

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
            | "WEEKNUM"
            | "ISOWEEKNUM"
            | "DAYS360"
            | "DATEDIF"
            | "YEARFRAC"
            | "NETWORKDAYS"
            | "WORKDAY"
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
            "WEEKNUM" | "ISOWEEKNUM" => self.week_call(sheet, name, arguments, being),
            "DAYS360" => self.days360_call(sheet, arguments, being),
            "DATEDIF" => self.datedif_call(sheet, arguments, being),
            "YEARFRAC" => self.yearfrac_call(sheet, arguments, being),
            "NETWORKDAYS" => self.networkdays_call(sheet, arguments, being),
            "WORKDAY" => self.workday_call(sheet, arguments, being),
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

    /// The tree's **week** of a day: the week of the year it is in.
    fn week_call(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
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
        if name == "ISOWEEKNUM" {
            return match crate::dates::iso_week_number(serial, self.system()) {
                Some(week) => Value::Number(week),
                None => Value::Error("#NUM!".to_string()),
            };
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
        match crate::dates::week_number(serial, kind, self.system()) {
            Some(week) => Value::Number(week),
            None => Value::Error("#NUM!".to_string()),
        }
    }

    /// The tree's `DAYS360`: the **days** between two dates under the
    /// year of three hundred and sixty days.
    ///
    /// The **method** is the tree's: the American one, which is the
    /// default, and the European one. The American one treats a start
    /// date of the thirty-first of a month, or the last day of
    /// February, as the thirtieth, and an end date of the thirty-first
    /// as the thirtieth when the start date is the thirtieth or after
    /// it; the European one treats any day after the thirtieth as the
    /// thirtieth.
    fn days360_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let first = match self.day_argument(sheet, &arguments[0], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let second = match self.day_argument(sheet, &arguments[1], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let european = match arguments.get(2) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                match value {
                    Value::Bool(flag) => flag,
                    Value::Empty => false,
                    other => match other.as_number() {
                        Some(number) if number.is_finite() => number != 0.0,
                        _ => return Value::Error("#VALUE!".to_string()),
                    },
                }
            }
            None => false,
        };
        let Some((year1, month1, day1)) = crate::daycounts::ymd(self.system(), first) else {
            return Value::Error("#NUM!".to_string());
        };
        let Some((year2, month2, day2)) = crate::daycounts::ymd(self.system(), second) else {
            return Value::Error("#NUM!".to_string());
        };
        let (mut start, mut end) = (i64::from(day1), i64::from(day2));
        if european {
            start = start.min(30);
            end = end.min(30);
        } else {
            let last_of_february =
                |year: i32, month: u32, day: u32| month == 2 && day >= crate::dates::days_in_month(year, 2);
            if start == 31 || last_of_february(year1, month1, day1) {
                start = 30;
            }
            if end == 31 && start >= 30 {
                end = 30;
            }
        }
        Value::Number(
            f64::from(year2 - year1) * 360.0
                + f64::from(month2 as i32 - month1 as i32) * 30.0
                + (end - start) as f64,
        )
    }

    /// The tree's `DATEDIF`: the **time** between two dates, in the
    /// unit that is asked for.
    ///
    /// The units are `D` — the days, `M` — the whole months, `Y` — the
    /// whole years, `YM` — the whole months that are left of the year,
    /// `MD` — the days that are left of the month, and `YD` — the days
    /// that are left of the year.
    ///
    /// The tree's rule: a start date **after** the end date is an
    /// error, and a unit that is not one of the six is an error.
    fn datedif_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let start = match self.day_argument(sheet, &arguments[0], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let end = match self.day_argument(sheet, &arguments[1], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let unit = self.evaluate(sheet, &arguments[2], being);
        let Value::Text(unit) = unit else {
            return Value::Error("#VALUE!".to_string());
        };
        let unit = unit.to_ascii_uppercase();
        if start > end {
            return Value::Error("#NUM!".to_string());
        }
        let Some((year1, month1, day1)) = crate::daycounts::ymd(self.system(), start) else {
            return Value::Error("#NUM!".to_string());
        };
        let Some((year2, month2, day2)) = crate::daycounts::ymd(self.system(), end) else {
            return Value::Error("#NUM!".to_string());
        };
        // The **whole months** between the two dates: a month is whole
        // when the day of the end is not before the day of the start.
        let months =
            i64::from(year2 - year1) * 12 + i64::from(month2) - i64::from(month1) - i64::from(day2 < day1);
        let result = match unit.as_str() {
            "D" => (end - start) as i64,
            "M" => months,
            "Y" => months / 12,
            "YM" => months % 12,
            "MD" => {
                if day2 >= day1 {
                    i64::from(day2 - day1)
                } else {
                    // The days of the month before the end date.
                    let (year, month) = if month2 == 1 { (year2 - 1, 12) } else { (year2, month2 - 1) };
                    i64::from(crate::dates::days_in_month(year, month)) - i64::from(day1) + i64::from(day2)
                }
            }
            "YD" => {
                // The days of the year that are left: the year is
                // shortened by one when the month and the day of the
                // start are after the month and the day of the end.
                let later = (month1, day1) > (month2, day2);
                let year = if later { year2 - 1 } else { year2 };
                let day = day1.min(crate::dates::days_in_month(year, month1).max(1));
                match crate::dates::to_serial(year, month1, day, self.system()) {
                    Some(shifted) => (end - shifted) as i64,
                    None => return Value::Error("#NUM!".to_string()),
                }
            }
            _ => return Value::Error("#NUM!".to_string()),
        };
        Value::Number(result as f64)
    }

    /// The **serial** of a date argument.
    ///
    /// The tree's rule: the argument is a number, and a serial outside
    /// the calendar is an error.
    fn day_argument(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<f64, String> {
        let value = self.evaluate(sheet, argument, being);
        let Some(number) = value.as_number() else {
            return Err("#VALUE!".to_string());
        };
        if !number.is_finite() {
            return Err("#NUM!".to_string());
        }
        let serial = number.floor();
        if !(0.0..=self.system().max_serial() as f64).contains(&serial) {
            return Err("#NUM!".to_string());
        }
        Ok(serial)
    }

    /// The tree's `YEARFRAC`: the **fraction of a year** between two
    /// dates, under a basis.
    ///
    /// The bases are the tree's:
    ///
    /// - zero — the 30/360;
    /// - one — the actual days over the actual days of the year;
    /// - two — the actual days over a year of three hundred and sixty
    ///   days;
    /// - three — the actual days over a year of three hundred and
    ///   sixty-five days;
    /// - four — the European 30/360.
    ///
    /// The tree's rule: the two dates are put in order, and a basis
    /// that is not one of the five is an error.
    fn yearfrac_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let first = match self.day_argument(sheet, &arguments[0], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let second = match self.day_argument(sheet, &arguments[1], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let basis = match arguments.get(2) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                let Some(number) = value.as_number() else {
                    return Value::Error("#VALUE!".to_string());
                };
                if !number.is_finite() {
                    return Value::Error("#NUM!".to_string());
                }
                number.trunc() as i64
            }
            None => 0,
        };
        if !(0..=4).contains(&basis) {
            return Value::Error("#NUM!".to_string());
        }
        let (start, end) = if first <= second { (first, second) } else { (second, first) };
        let days = end - start;
        match basis {
            // The **30/360** of the tree's financial basis zero — not
            // the one the tree's spreadsheet `DAYS360` uses.
            0 => match crate::daycounts::days360(self.system(), start, end, true) {
                Some(count) => Value::Number(count / 360.0),
                None => Value::Error("#NUM!".to_string()),
            },
            // The **European 30/360**.
            4 => match crate::daycounts::days360(self.system(), start, end, false) {
                Some(count) => Value::Number(count / 360.0),
                None => Value::Error("#NUM!".to_string()),
            },
            // The **actual days** over the actual days of the year.
            1 => {
                let Some((year1, month1, day1)) = crate::daycounts::ymd(self.system(), start) else {
                    return Value::Error("#NUM!".to_string());
                };
                let Some((year2, month2, day2)) = crate::daycounts::ymd(self.system(), end) else {
                    return Value::Error("#NUM!".to_string());
                };
                // The two dates are **within a year** when they are of
                // the same year, or when they are of two years that
                // touch and the start is not before the end.
                let within = year1 == year2
                    || (year2 == year1 + 1 && (month1 > month2 || (month1 == month2 && day1 >= day2)));
                if within {
                    // The **length of the year**: a year with a leap
                    // day in it is three hundred and sixty-six days.
                    let leap = (year1 == year2 && crate::dates::is_leap(year1))
                        || feb29_between(self.system(), start, end)
                        || (month2 == 2 && day2 == 29);
                    Value::Number(days / if leap { 366.0 } else { 365.0 })
                } else {
                    // The **average** of the years the two dates are
                    // between.
                    let years = f64::from(year2 - year1 + 1);
                    let Some(from) = crate::dates::to_serial(year1, 1, 1, self.system()) else {
                        return Value::Error("#NUM!".to_string());
                    };
                    let Some(to) = crate::dates::to_serial(year2 + 1, 1, 1, self.system()) else {
                        return Value::Error("#NUM!".to_string());
                    };
                    Value::Number(days / ((to - from) / years))
                }
            }
            // The actual days over a year of three hundred and sixty
            // days, and over a year of three hundred and sixty-five.
            2 => Value::Number(days / 360.0),
            3 => Value::Number(days / 365.0),
            _ => Value::Error("#NUM!".to_string()),
        }
    }

    /// The tree's `NETWORKDAYS`: the **working days** between two
    /// dates.
    ///
    /// The Saturday and the Sunday are not working days, and the two
    /// dates are **counted both ways**: the first day and the last
    /// day are working days when they are one. A start date after the
    /// end date gives the count with the other sign.
    ///
    /// The tree's `NETWORKDAYS` takes a list of **holidays**, and a
    /// list is an array, and this tree has no arrays.
    fn networkdays_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        if arguments.len() == 3 {
            return Value::Error("not supported yet: an array".to_string());
        }
        let start = match self.day_argument(sheet, &arguments[0], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let end = match self.day_argument(sheet, &arguments[1], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let (low, high, sign) = if start <= end { (start, end, 1) } else { (end, start, -1) };
        let Some(mut count) = crate::dates::weekdays_between(low, high, self.system()) else {
            return Value::Error("#NUM!".to_string());
        };
        // The **first** day is a working day too, and the count the
        // tree's helper gives does not hold it.
        match crate::dates::is_weekday(low, self.system()) {
            Some(true) => count += 1,
            Some(false) => {}
            None => return Value::Error("#NUM!".to_string()),
        }
        Value::Number((count * sign) as f64)
    }

    /// The tree's `WORKDAY`: the date that is **n working days** after
    /// a date.
    ///
    /// The start date is not one of the days that are counted: the
    /// first working day after it is the first step. A start date that
    /// is a Saturday or a Sunday moves to the next working day first.
    ///
    /// The tree's `WORKDAY` takes a list of **holidays**, and a list
    /// is an array, and this tree has no arrays.
    fn workday_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        if arguments.len() == 3 {
            return Value::Error("not supported yet: an array".to_string());
        }
        let start = match self.day_argument(sheet, &arguments[0], being) {
            Ok(serial) => serial,
            Err(message) => return Value::Error(message),
        };
        let value = self.evaluate(sheet, &arguments[1], being);
        let Some(number) = value.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if !number.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        let days = number.trunc();
        // The tree's bound: ten million days is a year of twenty-seven
        // thousand, and the tree refuses more.
        if days.abs() > 10_000_000.0 {
            return Value::Error("#NUM!".to_string());
        }
        match crate::dates::add_weekdays(start, days as i64, self.system()) {
            Some(serial) => Value::Number(serial),
            None => Value::Error("#NUM!".to_string()),
        }
    }
}

/// Whether a **leap day** lies between two dates.
fn feb29_between(system: crate::dates::System, start: f64, end: f64) -> bool {
    let Some((first_year, _, _)) = crate::daycounts::ymd(system, start) else { return false };
    let Some((last_year, _, _)) = crate::daycounts::ymd(system, end) else { return false };
    for year in first_year..=last_year {
        if !crate::dates::is_leap(year) {
            continue;
        }
        let Some(serial) = crate::dates::to_serial(year, 2, 29, system) else { continue };
        if serial >= start && serial <= end {
            return true;
        }
    }
    false
}
