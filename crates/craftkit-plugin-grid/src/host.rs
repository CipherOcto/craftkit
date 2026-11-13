//! The **host's** names: the functions the tree's program carries and
//! its registry does not.
//!
//! The tree has a **registry** (§895) of four hundred and ninety
//! functions, and the tree's *host* — the program that runs the
//! formulas — carries a second list beside it: `ROW`, `COLUMN`,
//! `AREAS`, `ISREF`, `ISFORMULA`, `FORMULATEXT`, `SHEET` and `SHEETS`
//! are the host's, and the registry does not hold them.
//!
//! These are here for the same reason the registry is: a formula may
//! ask where it is, and what the cell it names holds, and the answer
//! is a **value**.
//!
//! # The tree's rules
//!
//! - **`ROW`** and **`COLUMN`** with no argument are the row and the
//!   column of the cell the formula is in. With a reference, they are
//!   the row and the column of its **first** cell — and when the
//!   reference covers more than one row (or column) the tree gives
//!   back a **column** (or a row) of numbers, which is a table, and a
//!   table is refused here as it is everywhere (§885).
//! - **`AREAS`** is the number of areas in a reference. A reference
//!   here is one area, so the answer is one.
//! - **`ISREF`** says whether a thing is a reference at all.
//! - **`ISFORMULA`** says whether a cell holds a formula, and
//!   **`FORMULATEXT`** writes the formula of a cell — with its `=`.
//! - **`SHEET`** is the number of a sheet, and **`SHEETS`** is the
//!   number of sheets.

use std::collections::BTreeMap;

use crate::formula::{Book, CellRef, Expr, Value};

/// The **names** of the host's functions.
pub const NAMES: &[&str] =
    &["ROW", "COLUMN", "AREAS", "ISREF", "ISFORMULA", "FORMULATEXT", "SHEET", "SHEETS", "CELL", "AGGREGATE"];

/// The host's names that are **answered with a reason**, and the
/// reason each one is.
///
/// Three kinds, and the reasons are not the same:
///
/// - a name that gives back a **table** — `BYROW`, `BYCOL`,
///   `MAKEARRAY`, `MAP` and `SCAN` — is a table, and a formula here is
///   a value; (`GROUPBY` and `PIVOTBY` are tables too, and they are
///   refused with the twenty-eight of §885);
/// - a name that needs a **lambda** — `LAMBDA`, `LET` and
///   `ISOMITTED` — is a function, and a formula here has no
///   functions in it;
/// - a name that gives back a **reference** — `OFFSET`, `INDIRECT` and
///   `TRIMRANGE` — is a place, and a place that is worked out is not a
///   value.
pub const REFUSED: &[(&str, &str)] = &[
    ("BYROW", "not supported yet: an array"),
    ("BYCOL", "not supported yet: an array"),
    ("MAKEARRAY", "not supported yet: an array"),
    ("MAP", "not supported yet: an array"),
    ("SCAN", "not supported yet: an array"),
    ("REDUCE", "not supported yet: a lambda"),
    ("LAMBDA", "not supported yet: a lambda"),
    ("LET", "not supported yet: a lambda"),
    ("ISOMITTED", "not supported yet: a lambda"),
    ("OFFSET", "not supported yet: a reference that is worked out"),
    ("INDIRECT", "not supported yet: a reference that is worked out"),
    ("TRIMRANGE", "not supported yet: a reference that is worked out"),
];

/// The reason a name is refused, when it is one of the host's names
/// that are refused.
pub fn reason_for(name: &str) -> Option<&'static str> {
    REFUSED.iter().find(|(function, _)| *function == name).map(|(_, reason)| *reason)
}

impl Book {
    /// The **host's** functions.
    pub(crate) fn host(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        // A name that is **refused** is refused before it is
        // answered: the reason it is not here comes first.
        if let Some(reason) = reason_for(name) {
            return Value::Error(reason.to_string());
        }
        match name {
            "ROW" | "COLUMN" => self.row_or_column(sheet, name, arguments, being),
            "AREAS" => self.areas(arguments),
            "ISREF" => self.is_ref(arguments),
            "ISFORMULA" => self.is_formula(sheet, arguments),
            "FORMULATEXT" => self.formula_text(sheet, arguments),
            "CELL" => self.cell_info(sheet, arguments, being),
            "AGGREGATE" => self.aggregate(sheet, arguments, being),
            _ => self.sheet_or_sheets(sheet, name, arguments, being),
        }
    }

    /// The tree's **`ROW`** and **`COLUMN`**.
    ///
    /// With no argument, the row (or the column) of the cell the
    /// formula is in. With a reference, the row (or the column) of its
    /// first cell — and a reference that covers more than one row (or
    /// one column) is a **table** in the tree, and is refused here.
    fn row_or_column(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut [(String, CellRef)],
    ) -> Value {
        let is_row = name == "ROW";
        if arguments.is_empty() {
            let Some((_, cell)) = being.last() else {
                return Value::Error("#N/A".to_string());
            };
            return Value::Number(if is_row { f64::from(cell.row) + 1.0 } else { f64::from(cell.col) + 1.0 });
        }
        if arguments.len() > 1 {
            return Value::Error("#VALUE!".to_string());
        }
        if !matches!(arguments.first(), Some(Expr::Cell { .. } | Expr::Range { .. } | Expr::Name(_))) {
            return Value::Error("#VALUE!".to_string());
        }
        let (_, area) = match self.place_of(sheet, &arguments[0]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        if is_row {
            if area.start.row != area.end.row {
                return Value::Error("not supported yet: an array".to_string());
            }
            Value::Number(f64::from(area.start.row) + 1.0)
        } else {
            if area.start.col != area.end.col {
                return Value::Error("not supported yet: an array".to_string());
            }
            Value::Number(f64::from(area.start.col) + 1.0)
        }
    }

    /// The tree's **`AREAS`**: how many areas a reference has.
    ///
    /// A reference here is one area — a formula in this tree names one
    /// place — and so the answer is one. A thing that is not a
    /// reference is an error.
    fn areas(&self, arguments: &[Expr]) -> Value {
        if arguments.len() != 1 {
            return Value::Error("#VALUE!".to_string());
        }
        match arguments.first() {
            Some(Expr::Cell { .. } | Expr::Range { .. }) => Value::Number(1.0),
            Some(Expr::Name(name)) if self.has_name(name) => Value::Number(1.0),
            Some(Expr::Name(name)) => Value::Error(format!("there is no name `{name}`")),
            _ => Value::Error("#VALUE!".to_string()),
        }
    }

    /// The tree's **`ISREF`**: whether a thing is a reference.
    ///
    /// A name that is not in the book is not a reference: the tree
    /// looks the name up and finds nothing there.
    fn is_ref(&self, arguments: &[Expr]) -> Value {
        if arguments.len() != 1 {
            return Value::Error("#VALUE!".to_string());
        }
        match arguments.first() {
            Some(Expr::Cell { .. } | Expr::Range { .. }) => Value::Bool(true),
            Some(Expr::Name(name)) => Value::Bool(self.has_name(name)),
            _ => Value::Bool(false),
        }
    }

    /// The tree's **`ISFORMULA`**: whether a cell holds a formula.
    ///
    /// The tree's rule: the argument must be a **reference**, and a
    /// thing that is not one is `#N/A`.
    fn is_formula(&self, sheet: &str, arguments: &[Expr]) -> Value {
        match self.cell_of(sheet, arguments) {
            Ok(cell) => Value::Bool(cell.formula.is_some()),
            Err(value) => value,
        }
    }

    /// The tree's **`FORMULATEXT`**: the formula of a cell, with its
    /// `=`.
    ///
    /// A cell that holds no formula is `#N/A`, as the tree says.
    fn formula_text(&self, sheet: &str, arguments: &[Expr]) -> Value {
        match self.cell_of(sheet, arguments) {
            Ok(cell) => match cell.formula {
                Some(text) => {
                    let text = text.trim();
                    if text.starts_with('=') {
                        Value::Text(text.to_string())
                    } else {
                        Value::Text(format!("={text}"))
                    }
                }
                None => Value::Error("#N/A".to_string()),
            },
            Err(value) => value,
        }
    }

    /// The **place** an argument names: the sheet it is on and the
    /// area it covers.
    ///
    /// A **cell** is a place of one cell, a **range** is a place of
    /// many, and a **name** is the place the name stands for.
    fn place_of(&self, sheet: &str, argument: &Expr) -> Result<(String, crate::formula::Area), String> {
        match argument {
            Expr::Cell { sheet: named, anchor } => {
                let name = named.clone().unwrap_or_else(|| sheet.to_string());
                if !self.has_sheet(&name) {
                    return Err(format!("there is no sheet `{name}`"));
                }
                let cell = anchor.cell;
                Ok((name, crate::formula::Area::new(cell, cell)))
            }
            other => self.area_of(sheet, other),
        }
    }

    /// The **cell** an argument names, and the error when it names
    /// none.
    fn cell_of(&self, sheet: &str, arguments: &[Expr]) -> Result<crate::formula::Cell, Value> {
        if arguments.len() != 1 {
            return Err(Value::Error("#N/A".to_string()));
        }
        if !matches!(arguments.first(), Some(Expr::Cell { .. } | Expr::Range { .. } | Expr::Name(_))) {
            return Err(Value::Error("#N/A".to_string()));
        }
        let (name, area) = match self.place_of(sheet, &arguments[0]) {
            Ok(area) => area,
            Err(message) => return Err(Value::Error(message)),
        };
        match self.cell_at(&name, area.start) {
            Some(cell) => Ok(cell.clone()),
            None => Err(Value::Error("#N/A".to_string())),
        }
    }

    /// The tree's **`CELL`**: what the tree asks about a cell.
    ///
    /// The tree's question is the first argument — `address`, `row`,
    /// `col`, `contents`, `type`, `width`, `filename`, `sheetname`,
    /// `protect`, `format`, `color`, `parentheses` or `prefix` — and
    /// the cell is the second, or the cell the formula is in.
    ///
    /// The tree's `width` asks for the width of a column, and a cell
    /// here has no width, and so the tree's `width` has no answer
    /// here: it is refused with the reason it is not here.
    fn cell_info(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        let Some(first) = arguments.first() else {
            return Value::Error("#VALUE!".to_string());
        };
        let kind = match self.evaluate(sheet, first, being) {
            Value::Text(text) => text.to_ascii_lowercase(),
            Value::Error(message) => return Value::Error(message),
            _ => return Value::Error("#VALUE!".to_string()),
        };
        // The **cell**: the one the second argument names, or the one
        // the formula is in.
        let (name, cell) = match arguments.get(1) {
            None => match being.last() {
                Some((sheet, cell)) => (sheet.clone(), *cell),
                None => return Value::Error("#REF!".to_string()),
            },
            Some(argument) => {
                if !matches!(argument, Expr::Cell { .. } | Expr::Range { .. } | Expr::Name(_)) {
                    return Value::Error("#VALUE!".to_string());
                }
                match self.place_of(sheet, argument) {
                    Ok((name, area)) => (name, area.start),
                    Err(message) => return Value::Error(message),
                }
            }
        };
        let entry = self.cell_at(&name, cell);
        match kind.as_str() {
            "address" => Value::Text(format!("${}${}", CellRef::column_text(cell.col), cell.row + 1)),
            "row" => Value::Number(f64::from(cell.row) + 1.0),
            "col" => Value::Number(f64::from(cell.col) + 1.0),
            "contents" => match entry {
                Some(entry) => match entry.value.clone() {
                    Some(value) => value,
                    None => self.value_of_guarded(&name, cell, being),
                },
                None => Value::Empty,
            },
            "type" => {
                let value = match entry {
                    Some(entry) => entry.value.clone(),
                    None => None,
                };
                Value::Text(match value {
                    None => "b".to_string(),
                    Some(Value::Text(_)) => "l".to_string(),
                    Some(_) => "v".to_string(),
                })
            }
            "sheetname" => Value::Text(name),
            "protect" => {
                // The tree's cell is locked by its style, and a cell
                // that says nothing is locked.
                let locked = entry.and_then(|entry| entry.locked).unwrap_or(true);
                Value::Number(if locked { 1.0 } else { 0.0 })
            }
            "format" => {
                let code = entry.and_then(|entry| entry.format.clone()).unwrap_or_default();
                Value::Text(crate::format::cell_format_code(&code))
            }
            "filename" => Value::Text(String::new()),
            "color" | "parentheses" => Value::Number(0.0),
            "prefix" => Value::Text(String::new()),
            "width" => Value::Error("not supported yet: the width of a column".to_string()),
            _ => Value::Error("#VALUE!".to_string()),
        }
    }

    /// The tree's **`AGGREGATE`**: the tree's `SUBTOTAL` with
    /// nineteen functions and four options.
    ///
    /// The tree's rule: the first argument is the number of the
    /// function, the second is the **options** — which say what is
    /// left out (the rows a filter hides, the rows that are hidden by
    /// hand, the errors, and the subtotals inside) — and the rest are
    /// the ranges. The functions from fourteen on take a **k** after
    /// the range.
    fn aggregate(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let code = match self.number_of(sheet, &arguments[0], being) {
            Some(number) => number.trunc() as i64,
            None => return Value::Error("#VALUE!".to_string()),
        };
        let options = match self.number_of(sheet, &arguments[1], being) {
            Some(number) => number.trunc() as i64,
            None => return Value::Error("#VALUE!".to_string()),
        };
        if !(1..=19).contains(&code) {
            return Value::Error(format!("the function number {code} is not one this tree has"));
        }
        if !(0..=7).contains(&options) {
            return Value::Error(format!("the option number {options} is not one this tree has"));
        }
        // The tree's **options**: what is left out.
        let skip_hidden = matches!(options, 1 | 3 | 5 | 7);
        let skip_errors = matches!(options, 2 | 3 | 6 | 7);
        let skip_nested = options <= 3;
        // The **k** of the functions from fourteen on.
        let k = if (14..=19).contains(&code) {
            match arguments.get(3) {
                Some(argument) => match self.number_of(sheet, argument, being) {
                    Some(number) => number,
                    None => return Value::Error("#VALUE!".to_string()),
                },
                None => return Value::Error("#VALUE!".to_string()),
            }
        } else {
            0.0
        };
        // The **values**: the numbers of the ranges, and the values
        // that are not numbers and not empty. The rows the options
        // leave out are left out.
        let ranges = if (14..=19).contains(&code) { &arguments[2..3] } else { &arguments[2..] };
        let mut numbers: Vec<f64> = Vec::new();
        let mut plain: Vec<Value> = Vec::new();
        let mut filtered: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for argument in ranges {
            match argument {
                Expr::Range { sheet: named, start, end } => {
                    let name = named.as_deref().unwrap_or(sheet);
                    if !self.has_sheet(name) {
                        return Value::Error(format!("there is no sheet `{name}`"));
                    }
                    let area = crate::formula::Area::new(start.cell, end.cell);
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let cell = CellRef { col, row };
                            let rows =
                                filtered.entry(name.to_string()).or_insert_with(|| self.filtered_rows(name));
                            if self.left_out(name, cell, false, skip_hidden, skip_nested, rows) {
                                continue;
                            }
                            let value = self.value_of_guarded(name, cell, being);
                            match value {
                                Value::Error(message) => {
                                    if skip_errors {
                                        continue;
                                    }
                                    return Value::Error(message);
                                }
                                value => {
                                    if let Some(number) = value.as_number() {
                                        numbers.push(number);
                                    } else if !matches!(value, Value::Empty) {
                                        plain.push(value);
                                    }
                                }
                            }
                        }
                    }
                }
                Expr::Name(_) => {
                    let (name, area) = match self.area_of(sheet, argument) {
                        Ok(area) => area,
                        Err(message) => return Value::Error(message),
                    };
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let cell = CellRef { col, row };
                            let rows =
                                filtered.entry(name.clone()).or_insert_with(|| self.filtered_rows(&name));
                            if self.left_out(&name, cell, false, skip_hidden, skip_nested, rows) {
                                continue;
                            }
                            let value = self.value_of_guarded(&name, cell, being);
                            match value {
                                Value::Error(message) => {
                                    if skip_errors {
                                        continue;
                                    }
                                    return Value::Error(message);
                                }
                                value => {
                                    if let Some(number) = value.as_number() {
                                        numbers.push(number);
                                    } else if !matches!(value, Value::Empty) {
                                        plain.push(value);
                                    }
                                }
                            }
                        }
                    }
                }
                other => {
                    let value = self.evaluate(sheet, other, being);
                    match value {
                        Value::Error(message) => {
                            if skip_errors {
                                continue;
                            }
                            return Value::Error(message);
                        }
                        value => {
                            if let Some(number) = value.as_number() {
                                numbers.push(number);
                            } else if !matches!(value, Value::Empty) {
                                plain.push(value);
                            }
                        }
                    }
                }
            }
        }
        match code {
            1 => {
                if numbers.is_empty() {
                    Value::Error("#DIV/0!".to_string())
                } else {
                    Value::Number(numbers.iter().sum::<f64>() / numbers.len() as f64)
                }
            }
            2 => Value::Number(numbers.len() as f64),
            3 => Value::Number((numbers.len() + plain.len()) as f64),
            4 => Value::Number(numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
            5 => Value::Number(numbers.iter().copied().fold(f64::INFINITY, f64::min)),
            6 => {
                if numbers.is_empty() {
                    Value::Number(0.0)
                } else {
                    Value::Number(numbers.iter().product())
                }
            }
            7 => match crate::statistics::variance(&numbers, true) {
                Ok(variance) => Value::Number(variance.sqrt()),
                Err(message) => Value::Error(message),
            },
            8 => match crate::statistics::variance(&numbers, false) {
                Ok(variance) => Value::Number(variance.sqrt()),
                Err(message) => Value::Error(message),
            },
            9 => Value::Number(numbers.iter().sum()),
            10 => match crate::statistics::variance(&numbers, true) {
                Ok(variance) => Value::Number(variance),
                Err(message) => Value::Error(message),
            },
            11 => match crate::statistics::variance(&numbers, false) {
                Ok(variance) => Value::Number(variance),
                Err(message) => Value::Error(message),
            },
            12 => crate::statistics::median_of(numbers),
            13 => match crate::statistics::mode_of(&numbers) {
                Some(number) => Value::Number(number),
                None => Value::Error("#N/A".to_string()),
            },
            14 => crate::statistics::kth_of(numbers, k, true),
            15 => crate::statistics::kth_of(numbers, k, false),
            16 => crate::statistics::percentile_of(numbers, k, false),
            17 => {
                let q = k.trunc();
                if !(0.0..=4.0).contains(&q) {
                    return Value::Error("#NUM!".to_string());
                }
                crate::statistics::percentile_of(numbers, q / 4.0, false)
            }
            18 => crate::statistics::percentile_of(numbers, k, true),
            _ => {
                let q = k.trunc();
                if !(1.0..=3.0).contains(&q) {
                    return Value::Error("#NUM!".to_string());
                }
                crate::statistics::percentile_of(numbers, q / 4.0, true)
            }
        }
    }

    /// The **number** an argument is, when it is one.
    fn number_of(&self, sheet: &str, argument: &Expr, being: &mut Vec<(String, CellRef)>) -> Option<f64> {
        match argument {
            Expr::Number(number) => Some(*number),
            other => self.evaluate(sheet, other, being).as_number(),
        }
    }

    /// The tree's **`SHEET`** and **`SHEETS`**.
    ///
    /// `SHEET` is the number of a sheet — the sheet of the cell the
    /// formula is in, the sheet a reference names, or the sheet a
    /// name of a sheet is — and `SHEETS` is the number of sheets in the
    /// book, or the number of sheets a reference covers.
    fn sheet_or_sheets(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let is_sheet = name == "SHEET";
        if arguments.is_empty() {
            if !is_sheet {
                return Value::Number(self.sheet_count() as f64);
            }
            let Some((sheet, _)) = being.last() else {
                return Value::Error("#N/A".to_string());
            };
            return match self.sheet_number(sheet) {
                Some(number) => Value::Number(number as f64),
                None => Value::Error("#N/A".to_string()),
            };
        }
        if arguments.len() > 1 {
            return Value::Error("#N/A".to_string());
        }
        match arguments.first() {
            Some(Expr::Cell { .. } | Expr::Range { .. } | Expr::Name(_)) => {
                let (sheet, _) = match self.place_of(sheet, &arguments[0]) {
                    Ok(area) => area,
                    Err(message) => return Value::Error(message),
                };
                if !is_sheet {
                    // A reference here covers one sheet.
                    return Value::Number(1.0);
                }
                match self.sheet_number(&sheet) {
                    Some(number) => Value::Number(number as f64),
                    None => Value::Error("#N/A".to_string()),
                }
            }
            Some(other) if is_sheet => {
                // A **name of a sheet**: the text of the argument is
                // the name, and a name that is not a sheet is `#N/A`.
                let value = self.evaluate(sheet, other, being);
                match value {
                    Value::Text(text) => match self.sheet_number(&text) {
                        Some(number) => Value::Number(number as f64),
                        None => Value::Error("#N/A".to_string()),
                    },
                    Value::Error(message) => Value::Error(message),
                    _ => Value::Error("#N/A".to_string()),
                }
            }
            _ => Value::Error("#N/A".to_string()),
        }
    }
}
