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
    matches!(name, "DATE" | "YEAR" | "MONTH" | "DAY")
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
}
