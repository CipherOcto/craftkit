//! The **finding** of a value in a range.
//!
//! Three functions look for something in a range of cells:
//!
//! - `MATCH(needle, range)` — the **place** of the needle in the
//!   range, counted from one;
//! - `VLOOKUP(needle, table, column)` — the value of the **column**
//!   of the row whose first column holds the needle;
//! - `INDEX(range, row, [column])` — the value at a **place**.
//!
//! # The search
//!
//! The search is the tree's:
//!
//! - two numbers are equal when they are the same number, and two
//!   texts are equal when they are the same text **without case**;
//! - a **text** needle may hold the tree's wildcards, and then it
//!   matches the texts it matches — `B*` matches `banana`;
//! - a needle of one kind does not match a value of another kind, and
//!   an empty value matches an empty value;
//! - the **first** match is the match.
//!
//! # The search that is not here
//!
//! The tree has an **approximate** search: the largest value that is
//! not greater than the needle, found by a bisection over the whole
//! range. That search answers what the tree's own application answers
//! on data that is not sorted, and it is not a rule that can be
//! written down in a sentence. So it is **not here**: a lookup that
//! asks for it is refused with a message that says so.
//!
//! And `MATCH` and `VLOOKUP` differ from the tree in their **default**:
//! the tree's default is the approximate search, and here the default
//! is the exact one.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **lookup functions**.
pub fn is_lookup_function(name: &str) -> bool {
    matches!(name, "MATCH" | "VLOOKUP" | "INDEX")
}

/// The message of the search that is **not here**.
const APPROXIMATE: &str = "not supported yet: an approximate search";

/// Whether a **needle** and a **value** are the same.
///
/// The tree's rule: the same number, the same boolean, the same error,
/// two empty values, and two texts that are the same text — without
/// case, and with the wildcards when the needle holds them.
pub fn lookup_equal(needle: &Value, value: &Value) -> bool {
    match (needle, value) {
        (Value::Text(pattern), Value::Text(text)) => crate::formula::wildcard(pattern, text),
        (Value::Number(one), Value::Number(other)) => one == other,
        (Value::Bool(one), Value::Bool(other)) => one == other,
        (Value::Error(one), Value::Error(other)) => one == other,
        (Value::Empty, Value::Empty) => true,
        _ => false,
    }
}

impl Book {
    /// The **lookup functions**.
    pub(crate) fn looking(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "MATCH" => self.match_call(sheet, arguments, being),
            "VLOOKUP" => self.vlookup_call(sheet, arguments, being),
            "INDEX" => self.index_call(sheet, arguments, being),
            _ => Value::Error(format!("there is no function `{name}`")),
        }
    }

    /// The **values** of a range, in the order a reader reads them.
    fn values_of(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<(String, crate::formula::Area, Vec<Value>), String> {
        let (name, area) = self.area_of(sheet, argument)?;
        let mut values = Vec::new();
        for row in area.start.row..=area.end.row {
            for col in area.start.col..=area.end.col {
                values.push(self.value_of_guarded(&name, CellRef { col, row }, being));
            }
        }
        Ok((name, area, values))
    }

    /// The tree's `MATCH`: the **place** of a value in a range.
    fn match_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        // The **match type**: the exact search, or the approximate one
        // that is not here.
        if let Some(argument) = arguments.get(2) {
            let value = self.evaluate(sheet, argument, being);
            let kind = value.as_number().unwrap_or(1.0);
            if kind != 0.0 {
                return Value::Error(APPROXIMATE.to_string());
            }
        }
        let (_name, _area, values) = match self.values_of(sheet, &arguments[1], being) {
            Ok(values) => values,
            Err(message) => return Value::Error(message),
        };
        match values.iter().position(|value| lookup_equal(&needle, value)) {
            Some(place) => Value::Number((place + 1) as f64),
            None => Value::Error("#N/A".to_string()),
        }
    }

    /// The tree's `VLOOKUP`: the value of a **column** of the row
    /// whose first column holds the needle.
    fn vlookup_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 3 || arguments.len() > 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        // The **column** that is wanted: counted from one.
        let column = self.evaluate(sheet, &arguments[2], being);
        let Some(column) = column.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if column.is_nan() || column < 1.0 {
            return Value::Error("#VALUE!".to_string());
        }
        let column = column.trunc() as usize - 1;
        // The **range lookup**: the exact search, or the approximate one
        // that is not here. The tree's default is the approximate one.
        match arguments.get(3) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                match crate::formula::truth_of(&value) {
                    Ok(true) => return Value::Error(APPROXIMATE.to_string()),
                    Ok(false) => {}
                    Err(message) => return Value::Error(message),
                }
            }
            None => return Value::Error(APPROXIMATE.to_string()),
        }
        let (name, area, values) = match self.values_of(sheet, &arguments[1], being) {
            Ok(values) => values,
            Err(message) => return Value::Error(message),
        };
        let columns = (area.end.col - area.start.col + 1) as usize;
        let rows = (area.end.row - area.start.row + 1) as usize;
        if column >= columns {
            return Value::Error("#REF!".to_string());
        }
        // The **keys** are the first column of the table.
        for row in 0..rows {
            let key = &values[row * columns];
            if !lookup_equal(&needle, key) {
                continue;
            }
            let cell = CellRef { col: area.start.col + column as u32, row: area.start.row + row as u32 };
            return self.value_of_guarded(&name, cell, being);
        }
        Value::Error("#N/A".to_string())
    }

    /// The tree's `INDEX`: the value at a **place**.
    fn index_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let (name, area, _values) = match self.values_of(sheet, &arguments[0], being) {
            Ok(values) => values,
            Err(message) => return Value::Error(message),
        };
        let row = match arguments.get(1) {
            Some(argument) => self.evaluate(sheet, argument, being).as_number().unwrap_or(f64::NAN),
            None => 0.0,
        };
        let column = match arguments.get(2) {
            Some(argument) => self.evaluate(sheet, argument, being).as_number().unwrap_or(f64::NAN),
            None => 0.0,
        };
        if row.is_nan() || column.is_nan() || row < 0.0 || column < 0.0 {
            return Value::Error("#VALUE!".to_string());
        }
        let columns = (area.end.col - area.start.col + 1) as usize;
        let rows = (area.end.row - area.start.row + 1) as usize;
        let mut row = row.trunc() as usize;
        let mut column = column.trunc() as usize;
        // A range of **one row** with only one index: the index is the
        // column.
        if rows == 1 && arguments.len() == 2 && columns > 1 {
            column = row;
            row = 1;
        }
        // A zero is the whole row or the whole column, and that is an
        // **array** — and this tree has no arrays.
        if row == 0 || column == 0 {
            return Value::Error("not supported yet: an array".to_string());
        }
        if row > rows || column > columns {
            return Value::Error("#REF!".to_string());
        }
        let cell =
            CellRef { col: area.start.col + (column - 1) as u32, row: area.start.row + (row - 1) as u32 };
        self.value_of_guarded(&name, cell, being)
    }
}
