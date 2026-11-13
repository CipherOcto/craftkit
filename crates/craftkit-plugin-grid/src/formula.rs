//! The **formula** of a grid: what a cell's text means.
//!
//! # What is here
//!
//! A small formula language, and it is small on purpose: the point of
//! the port is not to be Excel, it is to find out what a **spreadsheet**
//! asks of a kit.
//!
//! - numbers, text (`"like this"`), booleans (`TRUE`, `FALSE`);
//! - **cell references** (`B3`) and **ranges** (`A1:B4`);
//! - the arithmetic (`+ - * / ^`), the comparisons (`= <> < <= > >=`)
//!   and the parentheses;
//! - the functions: `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT` and
//!   `COUNTA`; the tree's `TEXT`, which writes a value with a format
//!   code; the tree's `SUBTOTAL`; a **choice** with `IF`; and a
//!   **counting** with `COUNTIF` and `SUMIF`.
//! - a **choice**: `IF` takes a condition and the two values it
//!   chooses between, and **only the branch that is taken is
//!   computed**.
//!
//! # The two things that matter
//!
//! - a formula's **precedents are areas**, not cells: `SUM(A1:B4)` reads
//!   sixteen cells and one range, and the graph of a spreadsheet is a
//!   graph of areas. That is what [`precedents`] returns.
//! - a formula's **value is derived**: it is computed from the cells it
//!   reads, every time, and it is never written down. A cell that holds
//!   a formula holds the formula and nothing else.
//!
//! # The cycle
//!
//! A formula that reads itself is refused at the point it is set, and
//! the refusal says which cell and which cell. A cycle found later, in
//! the middle of a recalculation, is a bad afternoon.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use craftkit_sdk::serde_json;

use crate::format::Format;

/// The error a reference that is gone is written as.
///
/// The tree's own spelling: `#REF!`.
const REF_ERROR: &str = "#REF!";

/// The most rows a sheet has.
///
/// The tree's bound: a sheet is a million rows tall, and an address
/// that is not in the sheet is not an address.
pub const MAX_ROWS: u64 = 1_048_576;

/// The most columns a sheet has.
///
/// The tree's bound: a sheet is sixteen thousand columns wide.
pub const MAX_COLS: u64 = 16_384;

/// A cell's address: a column and a row, both counted from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellRef {
    /// The column, from zero.
    pub col: u32,
    /// The row, from zero.
    pub row: u32,
}

impl CellRef {
    /// The address, as `A1`.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_text(self) -> String {
        let mut name = CellRef::column_text(self.col);
        let _ = write!(name, "{}", self.row + 1);
        name
    }

    /// The letters of a column, as `A` — and `AA` for the twenty-seventh.
    pub fn column_text(col: u32) -> String {
        let mut name = String::new();
        let mut col = col;
        loop {
            name.insert(0, (b'A' + (col % 26) as u8) as char);
            if col < 26 {
                break;
            }
            col = col / 26 - 1;
        }
        name
    }

    /// The **column** a name says, counted from zero: `A` is 0 and `AA`
    /// is 26.
    pub fn column_index(text: &str) -> Option<u32> {
        let mut column: u32 = 0;
        for character in text.chars() {
            if !character.is_ascii_alphabetic() {
                return None;
            }
            column = column * 26 + (character.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
        }
        if text.is_empty() { None } else { Some(column - 1) }
    }

    /// The address, from `A1`.
    ///
    /// An **anchored** address — `$A$1` — is not the address of a cell
    /// but the address of a **reference** in a formula, and it is
    /// refused here: a command that names a cell names a cell, and a
    /// `$` in it is not a cell.
    pub fn parse(text: &str) -> Option<CellRef> {
        let anchor = Anchor::parse(text)?;
        if anchor.row_abs || anchor.col_abs {
            return None;
        }
        Some(anchor.cell)
    }
}

/// An **address**, and whether it is **anchored**.
///
/// The `$` of a formula: `$A$1` is anchored in both directions and
/// does not move when the formula is copied; `A$1` is anchored in its
/// row; `$A1` is anchored in its column; `A1` is anchored in neither.
///
/// The tree keeps the anchoring in the address and it is **the** thing
/// that decides whether a copied formula moves, so it is kept here too:
/// an address that has forgotten its `$` is an address that cannot be
/// copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    /// The cell the address names.
    pub cell: CellRef,
    /// Whether the **row** is anchored — the `$` before the number.
    pub row_abs: bool,
    /// Whether the **column** is anchored — the `$` before the letters.
    pub col_abs: bool,
}

impl Anchor {
    /// The address **moved**: the parts that are not anchored move.
    ///
    /// A reference that would leave the sheet is gone, and `None` says
    /// so.
    pub fn shifted(&self, row_delta: i64, col_delta: i64) -> Option<Anchor> {
        let row = if self.row_abs { i64::from(self.cell.row) } else { i64::from(self.cell.row) + row_delta };
        let col = if self.col_abs { i64::from(self.cell.col) } else { i64::from(self.cell.col) + col_delta };
        if row < 0 || col < 0 || row as u64 >= MAX_ROWS || col as u64 >= MAX_COLS {
            return None;
        }
        Some(Anchor {
            cell: CellRef { col: col as u32, row: row as u32 },
            row_abs: self.row_abs,
            col_abs: self.col_abs,
        })
    }

    /// The anchor with **rows inserted** at `at`.
    ///
    /// A reference whose row is **anchored** — the `$` before the
    /// number — does not move, and a reference above the row the rows
    /// are inserted at does not move either. A reference at or below it
    /// moves down by the number of rows inserted.
    pub fn inserted(&self, at: u64, count: u64) -> Anchor {
        if self.row_abs {
            return *self;
        }
        let row = self.cell.row as u64 + 1;
        if row < at {
            return *self;
        }
        Anchor {
            cell: CellRef { col: self.cell.col, row: self.cell.row + count as u32 },
            row_abs: self.row_abs,
            col_abs: self.col_abs,
        }
    }

    /// The anchor with the rows **deleted** at `at`.
    ///
    /// A reference whose row is **anchored** does not move. A
    /// reference below the rows that are deleted moves up by the
    /// number of rows deleted, and a reference **in** them cannot be
    /// written: `None` says the reference names a row that is gone.
    pub fn deleted(&self, at: u64, count: u64) -> Option<Anchor> {
        if self.row_abs {
            return Some(*self);
        }
        let row = self.cell.row as u64 + 1;
        if row >= at && row < at + count {
            return None;
        }
        if row >= at + count {
            return Some(Anchor {
                cell: CellRef { col: self.cell.col, row: self.cell.row - count as u32 },
                row_abs: self.row_abs,
                col_abs: self.col_abs,
            });
        }
        Some(*self)
    }

    /// The anchor with **columns inserted** at `at`.
    ///
    /// A reference whose column is **anchored** — the `$` before the
    /// letters — does not move, and a reference to the left of the
    /// column the columns are inserted at does not move either. A
    /// reference at or to the right of it moves right by the number of
    /// columns inserted.
    pub fn inserted_column(&self, at: u64, count: u64) -> Anchor {
        if self.col_abs {
            return *self;
        }
        let col = self.cell.col as u64 + 1;
        if col < at {
            return *self;
        }
        Anchor {
            cell: CellRef { col: self.cell.col + count as u32, row: self.cell.row },
            row_abs: self.row_abs,
            col_abs: self.col_abs,
        }
    }

    /// The anchor with the columns **deleted** at `at`.
    ///
    /// A reference whose column is **anchored** does not move. A
    /// reference to the right of the columns that are deleted moves
    /// left by the number of columns deleted, and a reference **in**
    /// them cannot be written: `None` says the reference names a
    /// column that is gone.
    pub fn deleted_column(&self, at: u64, count: u64) -> Option<Anchor> {
        if self.col_abs {
            return Some(*self);
        }
        let col = self.cell.col as u64 + 1;
        if col >= at && col < at + count {
            return None;
        }
        if col >= at + count {
            return Some(Anchor {
                cell: CellRef { col: self.cell.col - count as u32, row: self.cell.row },
                row_abs: self.row_abs,
                col_abs: self.col_abs,
            });
        }
        Some(*self)
    }

    pub fn to_text(self) -> String {
        let mut text = String::new();
        if self.col_abs {
            text.push('$');
        }
        text.push_str(&CellRef::column_text(self.cell.col));
        if self.row_abs {
            text.push('$');
        }
        let _ = write!(text, "{}", self.cell.row + 1);
        text
    }

    /// The address, from `$A$1`.
    pub fn parse(text: &str) -> Option<Anchor> {
        let characters: Vec<char> = text.chars().collect();
        let mut at = 0usize;
        let mut col_abs = false;
        if characters.get(at) == Some(&'$') {
            col_abs = true;
            at += 1;
        }
        let mut letters = String::new();
        while let Some(character) = characters.get(at) {
            if character.is_ascii_alphabetic() {
                letters.push(*character);
                at += 1;
            } else {
                break;
            }
        }
        let mut row_abs = false;
        if characters.get(at) == Some(&'$') {
            row_abs = true;
            at += 1;
        }
        let mut digits = String::new();
        while let Some(character) = characters.get(at) {
            if character.is_ascii_digit() {
                digits.push(*character);
                at += 1;
            } else {
                break;
            }
        }
        if letters.is_empty() || digits.is_empty() || at != characters.len() {
            return None;
        }
        // The **column** of the address: `A` is 0, `Z` is 25 and `AA`
        // is 26 — the letters are a base of twenty-six, and the first
        // of them counts from one.
        let mut col = 0u32;
        for character in letters.chars() {
            let upper = character.to_ascii_uppercase();
            let value = (upper as u8 - b'A') as u32;
            col = col.checked_mul(26)?.checked_add(value + 1)?;
        }
        let col = col.checked_sub(1)?;
        let row = digits.parse::<u32>().ok()?.checked_sub(1)?;
        Some(Anchor { cell: CellRef { col, row }, row_abs, col_abs })
    }

    /// The address **moved** by `(dr, dc)` rows and columns.
    ///
    /// An anchored part does not move, and an address that would leave
    /// the sheet is **nothing**: the caller refuses, and the tree writes
    /// `#REF!` in its place.
    pub fn moved(self, dr: i64, dc: i64) -> Option<Anchor> {
        let row = if self.row_abs { self.cell.row as i64 } else { self.cell.row as i64 + dr };
        let col = if self.col_abs { self.cell.col as i64 } else { self.cell.col as i64 + dc };
        if row < 0 || col < 0 || row as u64 >= MAX_ROWS || col as u64 >= MAX_COLS {
            return None;
        }
        Some(Anchor {
            cell: CellRef { col: col as u32, row: row as u32 },
            row_abs: self.row_abs,
            col_abs: self.col_abs,
        })
    }
}

impl std::fmt::Display for CellRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.to_text())
    }
}

/// A **range** of cells: a corner and another corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    /// The first corner.
    pub start: CellRef,
    /// The last corner.
    pub end: CellRef,
}

impl Area {
    /// The area from one corner to another, whichever way round they are.
    pub fn new(first: CellRef, second: CellRef) -> Area {
        Area {
            start: CellRef { col: first.col.min(second.col), row: first.row.min(second.row) },
            end: CellRef { col: first.col.max(second.col), row: first.row.max(second.row) },
        }
    }

    /// Whether the area covers that cell.
    pub fn covers(&self, cell: CellRef) -> bool {
        cell.col >= self.start.col
            && cell.col <= self.end.col
            && cell.row >= self.start.row
            && cell.row <= self.end.row
    }

    /// How many cells it covers.
    pub fn count(&self) -> u64 {
        let columns = (self.end.col - self.start.col + 1) as u64;
        let rows = (self.end.row - self.start.row + 1) as u64;
        columns * rows
    }

    /// The area, as `A1:B4`.
    pub fn to_text(self) -> String {
        if self.start == self.end {
            return self.start.to_text();
        }
        format!("{}:{}", self.start.to_text(), self.end.to_text())
    }
}

/// What a cell holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Nothing.
    Empty,
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
    /// A boolean.
    Bool(bool),
    /// An error, and what it says.
    Error(String),
}

impl Value {
    /// The value as a number, if it is one.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(number) => Some(*number),
            Value::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// The value as JSON, for the document.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Empty => serde_json::Value::Null,
            Value::Number(number) => serde_json::json!(number),
            Value::Text(text) => serde_json::json!(text),
            Value::Bool(value) => serde_json::json!(value),
            Value::Error(message) => serde_json::json!({ "error": message }),
        }
    }

    /// The value from JSON, as the document holds it.
    pub fn from_json(value: &serde_json::Value) -> Value {
        match value {
            serde_json::Value::Null => Value::Empty,
            serde_json::Value::Bool(value) => Value::Bool(*value),
            serde_json::Value::Number(number) => Value::Number(number.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(text) => Value::Text(text.clone()),
            serde_json::Value::Object(map) => match map.get("error") {
                Some(serde_json::Value::String(message)) => Value::Error(message.clone()),
                _ => Value::Empty,
            },
            _ => Value::Empty,
        }
    }
}

/// A formula, parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
    /// A boolean.
    Bool(bool),
    /// One cell, in a sheet — or in the sheet the formula is in.
    Cell {
        /// The sheet the cell is in, when the formula names one.
        sheet: Option<String>,
        /// The address, and its anchoring.
        anchor: Anchor,
    },
    /// A range of cells, in a sheet — or in the sheet the formula is in.
    Range {
        /// The sheet the range is in, when the formula names one.
        sheet: Option<String>,
        /// The first corner, and its anchoring.
        start: Anchor,
        /// The last corner, and its anchoring.
        end: Anchor,
    },
    /// A **name**: a word that is not a cell and not a function, and
    /// that stands for a range of cells.
    Name(String),
    /// An **error**: a reference that is gone, or a value that is not
    /// one.
    ///
    /// The tree's errors are written in the formula itself — `#REF!`
    /// where a reference names a cell that is not there — and the
    /// engine knows them. Here they are written in the formula too,
    /// and the value of a cell that holds one is the error.
    Error(String),
    /// A function and its arguments.
    Call(String, Vec<Expr>),
    /// A negation.
    Negate(Box<Expr>),
    /// A binary operation.
    Binary(String, Box<Expr>, Box<Expr>),
}

/// A **reference**: a sheet, maybe, and an area of cells in it.
///
/// A formula's precedents are areas, and an area is in a sheet: the
/// cells `A1:B4` of the sheet the formula is in, or the cells
/// `A1:B4` of the sheet the formula names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The sheet, when the reference names one.
    pub sheet: Option<String>,
    /// The area of cells.
    pub area: Area,
}

impl Reference {
    /// The reference, as it is written: `Sheet1!A1:B4`.
    pub fn to_text(&self) -> String {
        match &self.sheet {
            Some(sheet) => format!("{sheet}!{}", self.area.to_text()),
            None => self.area.to_text(),
        }
    }
}

/// A **name**: what a name stands for.
///
/// A name is a range of cells in a sheet. It is not a formula and not
/// another name, so a name may not be a cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The sheet the range is in.
    pub sheet: String,
    /// The area of cells.
    pub area: Area,
}

/// A **criterion**: an operation and a value.
///
/// The value may hold the tree's `*` wildcard — `*abc*` is "contains
/// abc" and `abc*` is "begins with abc".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterion {
    /// The operation: `=`, `!=`, `<`, `<=`, `>` or `>=`.
    pub op: String,
    /// The value it compares with.
    pub value: String,
}

/// A **filter**: what a sheet's rows are judged by.
///
/// There are two kinds, and they are the same kind of thing at two
/// sizes:
///
/// - a **column** and a **criterion** — the rows whose cell in that
///   column does not match are hidden;
/// - **groups** of criteria — the columns of a group are judged
///   together, and the groups are judged one by one. A row is hidden
///   when it matches **none** of the groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    /// One column and one criterion.
    Column {
        /// The column it filters, counted from zero.
        column: u32,
        /// The criterion the column's cells are judged by.
        criterion: Criterion,
    },
    /// One column and a **list of values**: the tree's own list
    /// filter.
    ///
    /// The rows whose cell shows one of the values are kept, and the
    /// others are hidden. The comparison is of the **text** the cell
    /// shows and it is without case.
    Values {
        /// The column it filters, counted from zero.
        column: u32,
        /// The values that are kept.
        values: Vec<String>,
        /// Whether an empty cell is kept.
        blanks: bool,
    },
    /// Groups of criteria: the columns of a group are judged together.
    Groups(Vec<Vec<(u32, Criterion)>>),
    /// The tree's own **advanced filter**: a **list** of rows under a row
    /// of headings, and a **criteria range** of headings and criteria.
    Advanced {
        /// The list: the rows under the headings.
        list: Area,
        /// The criteria: the rows of criteria under their headings.
        criteria: Area,
        /// Whether only the first row of a set of the same values is
        /// kept.
        unique: bool,
    },
}

/// The **operations** a filter may have.
pub const FILTER_OPS: [&str; 6] = ["=", "!=", "<", "<=", ">", ">="];

impl Criterion {
    /// Whether the operation is one of the tree's own.
    pub fn op_is_known(op: &str) -> bool {
        FILTER_OPS.contains(&op)
    }

    /// Whether a cell's **value** matches the criterion.
    ///
    /// The comparison operations compare **numbers**: an empty cell is
    /// zero, as it is in the tree, and a cell that holds text is not a
    /// number and does not match. The equality operations compare the
    /// **text** of the cell, and the tree's `*` is a wildcard.
    pub fn matches(&self, value: &Value) -> bool {
        match self.op.as_str() {
            "=" => wildcard(&self.value, &display_text(value)),
            "!=" => !wildcard(&self.value, &display_text(value)),
            "<" | "<=" | ">" | ">=" => {
                let Some(criterion) = self.value.trim().parse::<f64>().ok() else {
                    return false;
                };
                let cell = match value {
                    Value::Empty => 0.0,
                    other => match other.as_number() {
                        Some(number) => number,
                        None => return false,
                    },
                };
                match self.op.as_str() {
                    "<" => cell < criterion,
                    "<=" => cell <= criterion,
                    ">" => cell > criterion,
                    _ => cell >= criterion,
                }
            }
            _ => false,
        }
    }
}

/// The **text** of a value, as a cell shows it.
pub fn display_text(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Error(message) => message.clone(),
    }
}

/// Whether a **text** matches a pattern with the tree's wildcards.
///
/// The wildcards are the tree's:
///
/// - `*` is any **run** of characters, and it may be nothing at all;
/// - `?` is any **one** character;
/// - `~` is the escape: `~*` is a star and `~?` is a question mark.
///
/// The comparison is without case, as the tree's is.
pub fn wildcard(pattern: &str, text: &str) -> bool {
    wildcard_with_case(pattern, text, false)
}

/// Whether a pattern matches the **beginning** of a text, and the text
/// may be longer than the pattern.
///
/// This is the tree's rule for a **search**: the pattern is found
/// somewhere in the text and not only when it is the whole of it.
pub fn wildcard_at(pattern: &str, text: &str) -> bool {
    matched(pattern, text, false, false)
}

/// Whether a **text** matches a pattern with the tree's wildcards, and
/// whether the **case** matters.
///
/// When the case matters the two are compared as they are written;
/// when it does not, both are folded.
pub fn wildcard_with_case(pattern: &str, text: &str, case: bool) -> bool {
    matched(pattern, text, case, true)
}

/// Whether a pattern matches a text, and whether the text must be
/// matched whole.
fn matched(pattern: &str, text: &str, case: bool, whole: bool) -> bool {
    let (pattern, text) = if case {
        (pattern.to_string(), text.to_string())
    } else {
        (pattern.to_lowercase(), text.to_lowercase())
    };
    let text: Vec<char> = text.chars().collect();
    let mut tokens: Vec<(char, bool)> = Vec::new();
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        if character == '~' {
            if let Some(next) = characters.next() {
                tokens.push((next, true));
            }
        } else {
            tokens.push((character, false));
        }
    }
    let (mut in_text, mut in_pattern) = (0usize, 0usize);
    // Where the last `*` was, and where the text was when it was
    // found: a `*` that matched too little takes one more character.
    let (mut star, mut mark) = (None, 0usize);
    while in_text < text.len() {
        if !whole && in_pattern == tokens.len() {
            // The pattern is done and the text is longer: the pattern
            // matches the beginning of it.
            return true;
        }
        if in_pattern < tokens.len()
            && (tokens[in_pattern] == ('?', false)
                || (tokens[in_pattern].0 == text[in_text] && tokens[in_pattern] != ('*', false)))
        {
            in_text += 1;
            in_pattern += 1;
        } else if in_pattern < tokens.len() && tokens[in_pattern] == ('*', false) {
            star = Some(in_pattern);
            mark = in_text;
            in_pattern += 1;
        } else if let Some(star_at) = star {
            in_pattern = star_at + 1;
            mark += 1;
            in_text = mark;
        } else {
            return false;
        }
    }
    while in_pattern < tokens.len() && tokens[in_pattern] == ('*', false) {
        in_pattern += 1;
    }
    in_pattern == tokens.len()
}

/// A formula that did not parse.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaError {
    /// What went wrong, in words.
    pub message: String,
}

/// Parse a formula.
///
/// The text may begin with `=`; it does not have to.
pub fn parse(text: &str) -> Result<Expr, FormulaError> {
    let text = text.trim();
    let text = text.strip_prefix('=').unwrap_or(text).trim();
    if text.is_empty() {
        return Err(FormulaError { message: "the formula is empty".to_string() });
    }
    let characters: Vec<char> = text.chars().collect();
    let mut parser = Parser { characters, at: 0 };
    let expression = parser.expression()?;
    parser.skip_spaces();
    if parser.at < parser.characters.len() {
        return Err(FormulaError {
            message: format!("there is something after the formula at character {}", parser.at + 1),
        });
    }
    Ok(expression)
}

/// The cells and ranges a formula reads.
pub fn precedents(expression: &Expr, names: &BTreeMap<String, Name>) -> Vec<Reference> {
    let mut found: Vec<Reference> = Vec::new();
    collect_precedents(expression, names, &mut found);
    found
}

fn collect_precedents(expression: &Expr, names: &BTreeMap<String, Name>, found: &mut Vec<Reference>) {
    match expression {
        Expr::Cell { sheet, anchor } => {
            found.push(Reference { sheet: sheet.clone(), area: Area::new(anchor.cell, anchor.cell) })
        }
        Expr::Range { sheet, start, end } => {
            found.push(Reference { sheet: sheet.clone(), area: Area::new(start.cell, end.cell) })
        }
        Expr::Name(name) => {
            // A name stands for a range, and the range is what the
            // formula reads.
            if let Some(name) = names.get(name) {
                found.push(Reference { sheet: Some(name.sheet.clone()), area: name.area });
            }
        }
        Expr::Call(_, arguments) => {
            for argument in arguments {
                collect_precedents(argument, names, found);
            }
        }
        Expr::Negate(inner) => collect_precedents(inner, names, found),
        Expr::Binary(_, left, right) => {
            collect_precedents(left, names, found);
            collect_precedents(right, names, found);
        }
        Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => {}
    }
}

/// The parser: a small recursive descent over the characters.
struct Parser {
    /// The characters of the formula.
    characters: Vec<char>,
    /// Where the parser is.
    at: usize,
}

impl Parser {
    /// Skip the spaces.
    fn skip_spaces(&mut self) {
        while self.at < self.characters.len() && self.characters[self.at].is_whitespace() {
            self.at += 1;
        }
    }

    /// The character at the parser's place, if there is one.
    fn peek(&self) -> Option<char> {
        self.characters.get(self.at).copied()
    }

    /// Take a character if it is that one.
    fn take(&mut self, character: char) -> bool {
        if self.peek() == Some(character) {
            self.at += 1;
            return true;
        }
        false
    }

    /// Take a word if it is that one, in any case.
    fn take_word(&mut self, word: &str) -> bool {
        let mut at = self.at;
        for expected in word.chars() {
            match self.characters.get(at) {
                Some(character) if character.eq_ignore_ascii_case(&expected) => at += 1,
                _ => return false,
            }
        }
        self.at = at;
        true
    }

    /// The error, with the place it happened.
    fn error(&self, message: &str) -> FormulaError {
        FormulaError { message: format!("{message} (at character {})", self.at + 1) }
    }

    /// An expression: a comparison.
    fn expression(&mut self) -> Result<Expr, FormulaError> {
        let left = self.additive()?;
        self.skip_spaces();
        for operator in ["<>", "<=", ">="] {
            if self.take_word(operator) {
                let right = self.additive()?;
                return Ok(Expr::Binary(operator.to_string(), Box::new(left), Box::new(right)));
            }
        }
        if self.take('=') {
            let right = self.additive()?;
            return Ok(Expr::Binary("=".to_string(), Box::new(left), Box::new(right)));
        }
        if self.take('<') {
            let right = self.additive()?;
            return Ok(Expr::Binary("<".to_string(), Box::new(left), Box::new(right)));
        }
        if self.take('>') {
            let right = self.additive()?;
            return Ok(Expr::Binary(">".to_string(), Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    /// A sum or a difference.
    fn additive(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.multiplicative()?;
        loop {
            self.skip_spaces();
            if self.take('+') {
                let right = self.multiplicative()?;
                left = Expr::Binary("+".to_string(), Box::new(left), Box::new(right));
                continue;
            }
            if self.take('-') {
                let right = self.multiplicative()?;
                left = Expr::Binary("-".to_string(), Box::new(left), Box::new(right));
                continue;
            }
            return Ok(left);
        }
    }

    /// A product or a quotient.
    fn multiplicative(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.power()?;
        loop {
            self.skip_spaces();
            if self.take('*') {
                let right = self.power()?;
                left = Expr::Binary("*".to_string(), Box::new(left), Box::new(right));
                continue;
            }
            if self.take('/') {
                let right = self.power()?;
                left = Expr::Binary("/".to_string(), Box::new(left), Box::new(right));
                continue;
            }
            return Ok(left);
        }
    }

    /// A power.
    fn power(&mut self) -> Result<Expr, FormulaError> {
        let base = self.unary()?;
        self.skip_spaces();
        if self.take('^') {
            let exponent = self.power()?;
            return Ok(Expr::Binary("^".to_string(), Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    /// A negation.
    fn unary(&mut self) -> Result<Expr, FormulaError> {
        self.skip_spaces();
        if self.take('-') {
            let inner = self.unary()?;
            return Ok(Expr::Negate(Box::new(inner)));
        }
        if self.take('+') {
            return self.unary();
        }
        self.primary()
    }

    /// A literal, a reference or a call.
    fn primary(&mut self) -> Result<Expr, FormulaError> {
        self.skip_spaces();
        let Some(character) = self.peek() else {
            return Err(self.error("the formula stops here"));
        };
        if character == '(' {
            self.at += 1;
            let inner = self.expression()?;
            self.skip_spaces();
            if !self.take(')') {
                return Err(self.error("the bracket is not closed"));
            }
            return Ok(inner);
        }
        // A **quoted sheet name**: `'My Sheet'!A1`. The quote is the
        // sheet's, and the name inside it may hold anything but another
        // quote.
        if character == '\'' {
            self.at += 1;
            let mut name = String::new();
            loop {
                match self.peek() {
                    None => return Err(self.error("the sheet's name is not closed")),
                    Some('\'') => {
                        self.at += 1;
                        break;
                    }
                    Some(character) => {
                        name.push(character);
                        self.at += 1;
                    }
                }
            }
            if name.is_empty() {
                return Err(self.error("the sheet has no name"));
            }
            if !self.take('!') {
                return Err(self.error("the sheet's name is not followed by a cell"));
            }
            return self.reference(Some(name));
        }
        if character == '"' {
            self.at += 1;
            let mut text = String::new();
            loop {
                match self.peek() {
                    None => return Err(self.error("the text is not closed")),
                    Some('"') => {
                        self.at += 1;
                        break;
                    }
                    Some(character) => {
                        text.push(character);
                        self.at += 1;
                    }
                }
            }
            return Ok(Expr::Text(text));
        }
        if character == '#' {
            return self.error_literal();
        }
        if character.is_ascii_digit() || (character == '.' && self.next_is_digit()) {
            return self.number();
        }
        if character.is_ascii_alphabetic() || character == '$' {
            return self.word();
        }
        Err(self.error("this is not something a formula can hold"))
    }

    /// An **error** written in the formula: `#REF!`.
    ///
    /// The error is written with an exclamation mark, and the tree's
    /// errors that are written that way are the ones a formula may
    /// hold.
    fn error_literal(&mut self) -> Result<Expr, FormulaError> {
        let start = self.at;
        while self.at < self.characters.len() && self.characters[self.at] != '!' {
            self.at += 1;
        }
        if !self.take('!') {
            return Err(self.error("an error is written with an exclamation mark"));
        }
        let text: String = self.characters[start..self.at].iter().collect();
        Ok(Expr::Error(text))
    }

    /// Whether the next character is a digit.
    fn next_is_digit(&self) -> bool {
        self.characters.get(self.at + 1).map(|character| character.is_ascii_digit()).unwrap_or(false)
    }

    /// A number.
    fn number(&mut self) -> Result<Expr, FormulaError> {
        let start = self.at;
        while self.at < self.characters.len()
            && (self.characters[self.at].is_ascii_digit() || self.characters[self.at] == '.')
        {
            self.at += 1;
        }
        let text: String = self.characters[start..self.at].iter().collect();
        match text.parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(Expr::Number(number)),
            _ => Err(self.error("that is not a number")),
        }
    }

    /// A word: a reference, a range, a boolean or a function.
    fn word(&mut self) -> Result<Expr, FormulaError> {
        let start = self.at;
        while self.at < self.characters.len()
            && (self.characters[self.at].is_ascii_alphanumeric()
                || self.characters[self.at] == '$'
                || self.characters[self.at] == '_')
        {
            self.at += 1;
        }
        let mut word: String = self.characters[start..self.at].iter().collect();
        if word.is_empty() {
            return Err(self.error("there is nothing there"));
        }
        // A **function's name** may hold dots: `ERROR.TYPE`,
        // `NORM.S.DIST` and `T.DIST.2T`. The dots are only taken when
        // a bracket follows, so that a name with a dot in it is still
        // a name.
        let mut at = self.at;
        let mut furthest = self.at;
        loop {
            if self.characters.get(at) != Some(&'.')
                || !self.characters.get(at + 1).is_some_and(|character| character.is_ascii_alphanumeric())
            {
                break;
            }
            let mut end = at + 1;
            while end < self.characters.len()
                && (self.characters[end].is_ascii_alphanumeric() || self.characters[end] == '_')
            {
                end += 1;
            }
            let mut look = end;
            while look < self.characters.len() && self.characters[look].is_whitespace() {
                look += 1;
            }
            if self.characters.get(look) == Some(&'(') {
                furthest = end;
                at = end;
                continue;
            }
            if self.characters.get(look) == Some(&'.') {
                at = look;
                continue;
            }
            break;
        }
        if furthest > self.at {
            let tail: String = self.characters[self.at..furthest].iter().collect();
            word.push_str(&tail);
            self.at = furthest;
        }
        if word.eq_ignore_ascii_case("TRUE") {
            return Ok(Expr::Bool(true));
        }
        if word.eq_ignore_ascii_case("FALSE") {
            return Ok(Expr::Bool(false));
        }
        // A **sheet**: `Sheet1!A1`. The word is the sheet's name and
        // the reference that follows is in it.
        if self.peek() == Some('!') {
            self.at += 1;
            return self.reference(Some(word));
        }
        // A function: a name and a bracket.
        if self.peek() == Some('(') {
            self.at += 1;
            let mut arguments = Vec::new();
            self.skip_spaces();
            if !self.take(')') {
                loop {
                    arguments.push(self.expression()?);
                    self.skip_spaces();
                    if self.take(',') {
                        continue;
                    }
                    if self.take(')') {
                        break;
                    }
                    return Err(self.error("the bracket is not closed"));
                }
            }
            return Ok(Expr::Call(word.to_ascii_uppercase(), arguments));
        }
        // A **name**: a word that is not a cell. The parser does not
        // know the names of the workbook — it is a parser and not a
        // book — so a name is a word here and is resolved when the
        // formula is worked out.
        if Anchor::parse(&word).is_none() {
            return Ok(Expr::Name(word));
        }
        self.reference_to(word, None)
    }

    /// A **reference**: the word that follows, in the sheet (maybe).
    fn reference(&mut self, sheet: Option<String>) -> Result<Expr, FormulaError> {
        let start = self.at;
        while self.at < self.characters.len()
            && (self.characters[self.at].is_ascii_alphanumeric() || self.characters[self.at] == '$')
        {
            self.at += 1;
        }
        let word: String = self.characters[start..self.at].iter().collect();
        if word.is_empty() {
            return Err(self.error("there is nothing there"));
        }
        self.reference_to(word, sheet)
    }

    /// A **reference**: a cell or a range, in the sheet (maybe).
    ///
    /// A range is in one sheet: `Sheet1!A1:Sheet1!B4` is the range
    /// `A1:B4` of `Sheet1`, and a range that names two sheets is not a
    /// range and is refused.
    fn reference_to(&mut self, word: String, sheet: Option<String>) -> Result<Expr, FormulaError> {
        // A range: `A1:B4`.
        let mut after = self.at;
        while after < self.characters.len() && self.characters[after].is_whitespace() {
            after += 1;
        }
        if self.characters.get(after) == Some(&':') {
            let mut end = after + 1;
            while end < self.characters.len()
                && (self.characters[end].is_ascii_alphanumeric() || self.characters[end] == '$')
            {
                end += 1;
            }
            let mut second: String = self.characters[after + 1..end].iter().collect();
            self.at = end;
            // The second corner may name its sheet: `Sheet1!A1:Sheet1!B4`.
            if self.peek() == Some('!') {
                self.at += 1;
                let other = second.clone();
                let start = self.at;
                while self.at < self.characters.len()
                    && (self.characters[self.at].is_ascii_alphanumeric() || self.characters[self.at] == '$')
                {
                    self.at += 1;
                }
                second = self.characters[start..self.at].iter().collect();
                if Some(&other) != sheet.as_ref() {
                    return Err(FormulaError {
                        message: format!(
                            "a range is in one sheet, and this one names `{other}` and `{other}`"
                        ),
                    });
                }
            }
            let first = Anchor::parse(&word)
                .ok_or_else(|| FormulaError { message: format!("`{word}` is not a cell") })?;
            let second = Anchor::parse(&second)
                .ok_or_else(|| FormulaError { message: format!("`{second}` is not a cell") })?;
            return Ok(Expr::Range { sheet, start: first, end: second });
        }
        match Anchor::parse(&word) {
            Some(anchor) => Ok(Expr::Cell { sheet, anchor }),
            None => Err(FormulaError { message: format!("`{word}` is not a cell or a function") }),
        }
    }
}

/// The cells of a sheet: what is in them, and what a formula reads.
#[derive(Debug, Clone, Default)]
pub struct Cells {
    /// The cells, by address.
    cells: BTreeMap<CellRef, Cell>,
}

/// How the text of a cell lies **across** it.
///
/// The tree's own names: general, left, center, right, fill, justify
/// and center-across.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignH {
    /// The text is as the numbers and the text of the cell say: the
    /// numbers to the right and the text to the left.
    General,
    /// The text is to the left.
    Left,
    /// The text is in the middle.
    Center,
    /// The text is to the right.
    Right,
    /// The text is stretched over the width of the cell.
    Fill,
    /// The text is spread over the width of the cell.
    Justify,
    /// The text is in the middle, over the cells it is centered in.
    CenterAcross,
}

impl AlignH {
    /// The **name** of the alignment.
    pub fn to_text(self) -> &'static str {
        match self {
            AlignH::General => "general",
            AlignH::Left => "left",
            AlignH::Center => "center",
            AlignH::Right => "right",
            AlignH::Fill => "fill",
            AlignH::Justify => "justify",
            AlignH::CenterAcross => "center-across",
        }
    }

    /// The alignment of a **name**, and nothing when the name is not
    /// one of them.
    pub fn parse(text: &str) -> Option<AlignH> {
        match text.trim().to_lowercase().as_str() {
            "general" => Some(AlignH::General),
            "left" => Some(AlignH::Left),
            "center" | "centre" => Some(AlignH::Center),
            "right" => Some(AlignH::Right),
            "fill" => Some(AlignH::Fill),
            "justify" => Some(AlignH::Justify),
            "center-across" | "center_across" | "centeracross" => Some(AlignH::CenterAcross),
            _ => None,
        }
    }
}

/// How the text of a cell lies **down** it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignV {
    /// The text is at the top of the cell.
    Top,
    /// The text is in the middle of the cell.
    Middle,
    /// The text is at the bottom of the cell.
    Bottom,
}

impl AlignV {
    /// The **name** of the alignment.
    pub fn to_text(self) -> &'static str {
        match self {
            AlignV::Top => "top",
            AlignV::Middle => "middle",
            AlignV::Bottom => "bottom",
        }
    }

    /// The alignment of a **name**, and nothing when the name is not
    /// one of them.
    pub fn parse(text: &str) -> Option<AlignV> {
        match text.trim().to_lowercase().as_str() {
            "top" => Some(AlignV::Top),
            "middle" | "center" | "centre" => Some(AlignV::Middle),
            "bottom" => Some(AlignV::Bottom),
            _ => None,
        }
    }
}

/// The **alignment** of a cell: how its text lies across it and how it
/// lies down it, and how far it is indented and whether it wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Align {
    /// How the text lies across the cell.
    pub horizontal: AlignH,
    /// How the text lies down the cell.
    pub vertical: AlignV,
    /// How far the text is indented, in steps of the width of a
    /// character.
    pub indent: u8,
    /// Whether the text wraps at the width of the cell.
    pub wrap: bool,
}

impl Align {
    /// The alignment a cell has when it names none: the general one,
    /// the text at the top, no indent and no wrap.
    pub fn plain() -> Align {
        Align { horizontal: AlignH::General, vertical: AlignV::Top, indent: 0, wrap: false }
    }

    /// The alignment, as the sheet keeps it.
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "horizontal": self.horizontal.to_text(),
            "vertical": self.vertical.to_text(),
            "indent": self.indent,
            "wrap": self.wrap,
        })
    }

    /// The alignment of a sheet's own record, and nothing when the
    /// record is not one this tree writes.
    pub fn from_json(value: &serde_json::Value) -> Option<Align> {
        let horizontal = value.get("horizontal").and_then(|horizontal| horizontal.as_str());
        let vertical = value.get("vertical").and_then(|vertical| vertical.as_str());
        let horizontal = match horizontal {
            Some(horizontal) => AlignH::parse(horizontal)?,
            None => AlignH::General,
        };
        let vertical = match vertical {
            Some(vertical) => AlignV::parse(vertical)?,
            None => AlignV::Top,
        };
        let indent = value.get("indent").and_then(|indent| indent.as_u64()).unwrap_or(0);
        let indent = u8::try_from(indent).ok()?;
        let wrap = value.get("wrap").and_then(|wrap| wrap.as_bool()).unwrap_or(false);
        Some(Align { horizontal, vertical, indent, wrap })
    }
}

/// The **underline** of a cell's text.
///
/// The tree's own names: none, single, double, single accounting and
/// double accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Underline {
    /// There is no underline.
    None,
    /// A single line under the text.
    Single,
    /// Two lines under the text.
    Double,
    /// A single line under the text, and the line is as long as the
    /// numbers of the cell are wide.
    SingleAccounting,
    /// Two lines under the text, and the lines are as long as the
    /// numbers of the cell are wide.
    DoubleAccounting,
}

impl Underline {
    /// The **name** of the underline.
    pub fn to_text(self) -> &'static str {
        match self {
            Underline::None => "none",
            Underline::Single => "single",
            Underline::Double => "double",
            Underline::SingleAccounting => "single-accounting",
            Underline::DoubleAccounting => "double-accounting",
        }
    }

    /// The underline of a **name**, and nothing when the name is not
    /// one of them.
    pub fn parse(text: &str) -> Option<Underline> {
        match text.trim().to_lowercase().replace('_', "-").as_str() {
            "none" => Some(Underline::None),
            "single" => Some(Underline::Single),
            "double" => Some(Underline::Double),
            "single-accounting" => Some(Underline::SingleAccounting),
            "double-accounting" => Some(Underline::DoubleAccounting),
            _ => None,
        }
    }
}

/// Where the text of a cell sits **above or below** its line.
///
/// The tree's own names: baseline, superscript and subscript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vert {
    /// On the line of the text.
    Baseline,
    /// Above the line of the text.
    Superscript,
    /// Below the line of the text.
    Subscript,
}

impl Vert {
    /// The **name** of the place.
    pub fn to_text(self) -> &'static str {
        match self {
            Vert::Baseline => "baseline",
            Vert::Superscript => "superscript",
            Vert::Subscript => "subscript",
        }
    }

    /// The place of a **name**, and nothing when the name is not one
    /// of them.
    pub fn parse(text: &str) -> Option<Vert> {
        match text.trim().to_lowercase().as_str() {
            "baseline" | "base" => Some(Vert::Baseline),
            "superscript" | "super" => Some(Vert::Superscript),
            "subscript" | "sub" => Some(Vert::Subscript),
            _ => None,
        }
    }
}

/// The **font** of a cell: the face of its text, the size of it, and
/// the marks that are put on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// The **name** of the face, when the cell names one.
    pub name: Option<String>,
    /// The **size** of the text, when the cell names one.
    pub size: Option<f64>,
    /// Whether the text is **bold**.
    pub bold: bool,
    /// Whether the text is **italic**.
    pub italic: bool,
    /// Whether the text is **struck through**.
    pub strike: bool,
    /// The **underline** of the text.
    pub underline: Underline,
    /// Where the text sits above or below its line.
    pub vert: Vert,
    /// The **colour** of the text, when it has one.
    pub colour: Option<String>,
}

impl Font {
    /// The font a cell has when it names none: the face of the sheet,
    /// the size of the sheet, and no marks at all.
    pub fn plain() -> Font {
        Font {
            name: None,
            size: None,
            bold: false,
            italic: false,
            strike: false,
            underline: Underline::None,
            vert: Vert::Baseline,
            colour: None,
        }
    }

    /// The font, as the sheet keeps it.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": match &self.name {
                Some(name) => serde_json::Value::String(name.clone()),
                None => serde_json::Value::Null,
            },
            "size": match self.size {
                Some(size) => serde_json::Value::from(size),
                None => serde_json::Value::Null,
            },
            "bold": self.bold,
            "italic": self.italic,
            "strike": self.strike,
            "underline": self.underline.to_text(),
            "vert": self.vert.to_text(),
            "colour": match &self.colour {
                Some(colour) => serde_json::Value::String(colour.clone()),
                None => serde_json::Value::Null,
            },
        })
    }

    /// The font of a sheet's own record, and nothing when the record is
    /// not one this tree writes.
    pub fn from_json(value: &serde_json::Value) -> Option<Font> {
        let name = value.get("name").and_then(|name| name.as_str()).map(|name| name.to_string());
        let size = value.get("size").and_then(|size| size.as_f64());
        let bold = value.get("bold").and_then(|bold| bold.as_bool()).unwrap_or(false);
        let italic = value.get("italic").and_then(|italic| italic.as_bool()).unwrap_or(false);
        let strike = value.get("strike").and_then(|strike| strike.as_bool()).unwrap_or(false);
        let underline = match value.get("underline").and_then(|underline| underline.as_str()) {
            Some(underline) => Underline::parse(underline)?,
            None => Underline::None,
        };
        let vert = match value.get("vert").and_then(|vert| vert.as_str()) {
            Some(vert) => Vert::parse(vert)?,
            None => Vert::Baseline,
        };
        let colour = value.get("colour").and_then(|colour| colour.as_str()).map(|colour| colour.to_string());
        Some(Font { name, size, bold, italic, strike, underline, vert, colour })
    }
}

/// A cell: a value, or a formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The value, when the cell holds one.
    pub value: Option<Value>,
    /// The formula's text, when the cell holds one.
    pub formula: Option<String>,
    /// The **number format** of the cell, when it has one.
    ///
    /// The tree's format is a style and a style is shared between
    /// cells; here the format is the cell's own, so a hundred cells
    /// with one format carry a hundred formats — unless the cell names
    /// a **style**, and then the style's format is the one it is
    /// written with.
    pub format: Option<String>,
    /// The **style** the cell names: the name of a style of the
    /// workbook, when it names one.
    pub style: Option<String>,
    /// The **alignment** of the cell, when it has one.
    ///
    /// The tree's alignment is a style and a style is shared between
    /// cells; here the alignment is the cell's own, as the format is.
    pub align: Option<Align>,
    /// The **font** of the cell, when it has one.
    ///
    /// The tree's font is a style and a style is shared between cells;
    /// here the font is the cell's own, as the format and the
    /// alignment are.
    pub font: Option<Font>,
    /// The **lock** of the cell, when it has one.
    ///
    /// The tree's lock is a flag of the style of the cell, and a cell
    /// is locked unless its style says otherwise. Here the lock is the
    /// cell's own, and a cell with no lock of its own follows the
    /// sheet.
    pub locked: Option<bool>,
    /// The **fill** of the cell: the colour behind its text, when it
    /// has one.
    ///
    /// The tree's fill is a style and a style is shared between cells;
    /// here the fill is the cell's own, as the format, the alignment
    /// and the font are.
    pub fill: Option<String>,
}

impl Cells {
    /// An empty sheet.
    pub fn new() -> Cells {
        Cells { cells: BTreeMap::new() }
    }

    /// The cell at that address.
    pub fn get(&self, cell: CellRef) -> Option<&Cell> {
        self.cells.get(&cell)
    }

    /// Put a value in a cell.
    ///
    /// The cell keeps its **format**: setting a value does not change
    /// how the value is written.
    pub fn set_value(&mut self, cell: CellRef, value: Value) {
        let format = self.cells.get(&cell).and_then(|entry| entry.format.clone());
        let style = self.cells.get(&cell).and_then(|entry| entry.style.clone());
        let align = self.cells.get(&cell).and_then(|entry| entry.align);
        let font = self.cells.get(&cell).and_then(|entry| entry.font.clone());
        let locked = self.cells.get(&cell).and_then(|entry| entry.locked);
        let fill = self.cells.get(&cell).and_then(|entry| entry.fill.clone());
        self.cells.insert(
            cell,
            Cell { value: Some(value), formula: None, format, style, align, font, locked, fill },
        );
    }

    /// Put a formula in a cell.
    ///
    /// The cell keeps its **format**, as it does when a value is put in
    /// it.
    pub fn set_formula(&mut self, cell: CellRef, formula: String) {
        let format = self.cells.get(&cell).and_then(|entry| entry.format.clone());
        let style = self.cells.get(&cell).and_then(|entry| entry.style.clone());
        let align = self.cells.get(&cell).and_then(|entry| entry.align);
        let font = self.cells.get(&cell).and_then(|entry| entry.font.clone());
        let locked = self.cells.get(&cell).and_then(|entry| entry.locked);
        let fill = self.cells.get(&cell).and_then(|entry| entry.fill.clone());
        self.cells.insert(
            cell,
            Cell { value: None, formula: Some(formula), format, style, align, font, locked, fill },
        );
    }

    /// The **number format** of a cell, if it has one.
    pub fn format(&self, cell: CellRef) -> Option<&str> {
        self.cells.get(&cell)?.format.as_deref()
    }

    /// Put a **number format** on a cell, keeping what the cell holds.
    ///
    /// A cell that is not there is made: a cell may be formatted before
    /// it has a value.
    pub fn set_format(&mut self, cell: CellRef, format: Option<String>) {
        match self.cells.get_mut(&cell) {
            Some(entry) => entry.format = format,
            None => {
                self.cells.insert(
                    cell,
                    Cell {
                        value: None,
                        formula: None,
                        style: None,
                        align: None,
                        font: None,
                        locked: None,
                        fill: None,
                        format,
                    },
                );
            }
        }
    }

    /// The **style** a cell names, if it names one.
    pub fn style(&self, cell: CellRef) -> Option<&str> {
        self.cells.get(&cell)?.style.as_deref()
    }

    /// Put a **style** on a cell, keeping what the cell holds.
    ///
    /// A cell that is not there is made: a cell may be styled before it
    /// has a value.
    pub fn set_style(&mut self, cell: CellRef, style: Option<String>) {
        match self.cells.get_mut(&cell) {
            Some(entry) => entry.style = style,
            None => {
                self.cells.insert(
                    cell,
                    Cell {
                        value: None,
                        formula: None,
                        format: None,
                        align: None,
                        font: None,
                        locked: None,
                        fill: None,
                        style,
                    },
                );
            }
        }
    }

    /// The **alignment** of a cell, if it has one.
    pub fn align(&self, cell: CellRef) -> Option<&Align> {
        self.cells.get(&cell)?.align.as_ref()
    }

    /// Put an **alignment** on a cell, keeping what the cell holds.
    ///
    /// A cell that is not there is made: a cell may be aligned before
    /// it has a value.
    pub fn set_align(&mut self, cell: CellRef, align: Option<Align>) {
        match self.cells.get_mut(&cell) {
            Some(entry) => entry.align = align,
            None => {
                self.cells.insert(
                    cell,
                    Cell {
                        value: None,
                        formula: None,
                        format: None,
                        style: None,
                        font: None,
                        locked: None,
                        fill: None,
                        align,
                    },
                );
            }
        }
    }

    /// The **font** of a cell, if it has one.
    pub fn font(&self, cell: CellRef) -> Option<&Font> {
        self.cells.get(&cell)?.font.as_ref()
    }

    /// Put a **font** on a cell, keeping what the cell holds.
    ///
    /// A cell that is not there is made: a cell may have a font before
    /// it has a value.
    pub fn set_font(&mut self, cell: CellRef, font: Option<Font>) {
        match self.cells.get_mut(&cell) {
            Some(entry) => entry.font = font,
            None => {
                self.cells.insert(
                    cell,
                    Cell {
                        value: None,
                        formula: None,
                        format: None,
                        style: None,
                        align: None,
                        locked: None,
                        fill: None,
                        font,
                    },
                );
            }
        }
    }

    /// The **lock** of a cell, if it has one.
    pub fn locked(&self, cell: CellRef) -> Option<bool> {
        self.cells.get(&cell)?.locked
    }

    /// Put a **lock** on a cell, keeping what the cell holds.
    ///
    /// A cell that is not there is made: a cell may be locked before
    /// it has a value.
    pub fn set_locked(&mut self, cell: CellRef, locked: Option<bool>) {
        match self.cells.get_mut(&cell) {
            Some(entry) => entry.locked = locked,
            None => {
                self.cells.insert(
                    cell,
                    Cell {
                        value: None,
                        formula: None,
                        format: None,
                        style: None,
                        align: None,
                        font: None,
                        fill: None,
                        locked,
                    },
                );
            }
        }
    }

    /// The **fill** of a cell, if it has one.
    pub fn fill(&self, cell: CellRef) -> Option<&str> {
        self.cells.get(&cell)?.fill.as_deref()
    }

    /// Put a **fill** on a cell, keeping what the cell holds.
    ///
    /// A cell that is not there is made: a cell may be filled before it
    /// has a value.
    pub fn set_fill(&mut self, cell: CellRef, fill: Option<String>) {
        match self.cells.get_mut(&cell) {
            Some(entry) => entry.fill = fill,
            None => {
                self.cells.insert(
                    cell,
                    Cell {
                        value: None,
                        formula: None,
                        format: None,
                        style: None,
                        align: None,
                        font: None,
                        locked: None,
                        fill,
                    },
                );
            }
        }
    }

    /// Empty a cell.
    pub fn clear(&mut self, cell: CellRef) -> bool {
        self.cells.remove(&cell).is_some()
    }

    /// Put a **cell** as it is, with all that it holds.
    pub fn set_entry(&mut self, cell: CellRef, entry: Cell) {
        self.cells.insert(cell, entry);
    }

    /// Take a **cell** away, and give back what it held.
    pub fn take(&mut self, cell: CellRef) -> Option<Cell> {
        self.cells.remove(&cell)
    }

    /// Empty the **contents** of a cell: its value and its formula.
    ///
    /// The cell keeps its **format** and its **style**: the contents
    /// are what the cell holds and the format is how it is shown.
    pub fn clear_contents(&mut self, cell: CellRef) -> bool {
        let Some(entry) = self.cells.get_mut(&cell) else { return false };
        let had = entry.value.is_some() || entry.formula.is_some();
        entry.value = None;
        entry.formula = None;
        had
    }

    /// Clear the **format** of a cell: its number format and its
    /// style.
    ///
    /// The cell keeps its **contents**: what it holds is not changed by
    /// how it is shown.
    pub fn clear_format(&mut self, cell: CellRef) -> bool {
        let Some(entry) = self.cells.get_mut(&cell) else { return false };
        let had = entry.format.is_some() || entry.style.is_some();
        entry.format = None;
        entry.style = None;
        had
    }

    /// Every cell, in order.
    pub fn iter(&self) -> impl Iterator<Item = (CellRef, &Cell)> {
        self.cells.iter().map(|(cell, value)| (*cell, value))
    }

    /// **Rename** the sheet in every formula that names it.
    ///
    /// A formula is **text**, so the text is parsed, the sheet is named
    /// again and the expression is printed. A formula that does not
    /// name the sheet is left exactly as the caller wrote it.
    ///
    /// The answer is how many formulas were rewritten.
    pub fn rename_sheet(&mut self, old: &str, new: &str) -> usize {
        let mut renamed = 0usize;
        for entry in self.cells.values_mut() {
            let Some(text) = &entry.formula else { continue };
            let Some(rewritten) = rename_in_formula(text, old, new) else { continue };
            entry.formula = Some(rewritten);
            renamed += 1;
        }
        renamed
    }

    /// How many cells there are.
    pub fn len(&self) -> usize {
        self.cells.len()
    }
}

/// The **sheets** of a workbook: the cells of each sheet, by name.
///
/// A formula may read another sheet, and the value of a cell is then
/// not a fact about the sheet but a fact about the **workbook**: the
/// value of a cell in `Sheet2` is the value of `Sheet1!A1` and the
/// value of `Sheet2!A1` at the same time.
///
/// The cycle is a cycle of the workbook too: a formula in one sheet
/// that reads a cell in another sheet that reads it back is a cycle,
/// and the workbook is where that is seen.
#[derive(Debug, Clone, Default)]
pub struct Book {
    /// The sheets, by name.
    sheets: BTreeMap<String, Cells>,
    /// The **hidden rows** of each sheet, by sheet name.
    hidden: BTreeMap<String, Vec<u64>>,
    /// The **filter** of each sheet, by sheet name.
    filters: BTreeMap<String, Filter>,
    /// The **names**, by name.
    names: BTreeMap<String, Name>,
    /// The **date system** of the workbook the sheets are in.
    system: crate::dates::System,
}

impl Book {
    /// A workbook in a **date system**.
    pub fn in_system(system: crate::dates::System) -> Book {
        Book {
            sheets: BTreeMap::new(),
            hidden: BTreeMap::new(),
            filters: BTreeMap::new(),
            names: BTreeMap::new(),
            system,
        }
    }

    /// The **date system** of the workbook.
    pub fn system(&self) -> crate::dates::System {
        self.system
    }

    /// Say which of a sheet's **rows** are hidden.
    pub fn hide_rows(&mut self, sheet: &str, rows: Vec<u64>) {
        self.hidden.insert(sheet.to_string(), rows);
    }

    /// Whether a **row** of a sheet is hidden.
    pub fn row_is_hidden(&self, sheet: &str, row: u64) -> bool {
        self.hidden.get(sheet).is_some_and(|rows| rows.contains(&row))
    }

    /// Set the **filter** of a sheet.
    pub fn set_filter(&mut self, sheet: &str, filter: Filter) {
        self.filters.insert(sheet.to_string(), filter);
    }

    /// The **rows** a sheet's filter hides, counted from one.
    ///
    /// The rows are **derived**: they are the rows whose cells do not
    /// match, and they are worked out from the cells each time they
    /// are asked for.
    pub fn filtered_rows(&self, sheet: &str) -> Vec<u64> {
        let Some(filter) = self.filters.get(sheet) else {
            return Vec::new();
        };
        let Some(cells) = self.sheets.get(sheet) else {
            return Vec::new();
        };
        // The **value** of a cell, or nothing when it is empty.
        let value_at = |column: u32, row: u32| -> Value {
            cells
                .get(CellRef { col: column, row })
                .and_then(|cell| cell.value.clone())
                .unwrap_or(Value::Empty)
        };
        // The **advanced filter**: the rows of its list that are kept,
        // worked out once.
        let advanced: Option<Vec<u32>> = match filter {
            Filter::Advanced { list, criteria, unique } => {
                crate::advancedfilter::kept_rows(cells, list, criteria, *unique).ok()
            }
            _ => None,
        };
        let mut rows: Vec<u64> = Vec::new();
        for (cell, _) in cells.iter() {
            let row = cell.row as u64 + 1;
            if rows.contains(&row) {
                continue;
            }
            // A row is hidden when it does not match: a column filter
            // judges one cell, and groups judge the groups one by one —
            // a row that matches **any** group is kept.
            let hidden = match filter {
                Filter::Column { column, criterion } => !criterion.matches(&value_at(*column, cell.row)),
                Filter::Values { column, values, blanks } => {
                    // The **text** the cell shows, and it is without
                    // case: the tree's own rule.
                    let text = display_text(&value_at(*column, cell.row));
                    let kept = if text.is_empty() {
                        *blanks
                    } else {
                        values.iter().any(|value| value.eq_ignore_ascii_case(&text))
                    };
                    !kept
                }
                Filter::Groups(groups) => !groups.iter().any(|group| {
                    group.iter().all(|(column, criterion)| criterion.matches(&value_at(*column, cell.row)))
                }),
                Filter::Advanced { list, .. } => {
                    // The tree's own advanced filter: the rows of the
                    // list that match are kept, and the others are
                    // hidden. The **headings** of the list and the rows
                    // outside it are not hidden: the tree's own rule.
                    let in_the_list = cell.row > list.start.row && cell.row <= list.end.row;
                    let kept = advanced.as_ref().map(|kept| kept.contains(&cell.row)).unwrap_or(false);
                    in_the_list && !kept
                }
            };
            if hidden {
                rows.push(row);
            }
        }
        rows.sort_unstable();
        rows
    }

    /// Put a **name** in the workbook.
    pub fn add_name(&mut self, name: &str, name_of: Name) {
        self.names.insert(name.to_string(), name_of);
    }

    /// The **names** of the workbook.
    pub fn names(&self) -> &BTreeMap<String, Name> {
        &self.names
    }

    /// Put a sheet in the workbook.
    pub fn add(&mut self, name: &str, cells: Cells) {
        self.sheets.insert(name.to_string(), cells);
    }

    /// Whether a cell's formula reads itself, directly or through
    /// others — in this sheet or in another.
    ///
    /// The cycle is a cycle of the **workbook**: a formula in one sheet
    /// that reads a cell in another sheet that reads it back is a
    /// cycle, and it is a cycle because the sheets are in one book.
    pub fn reads_itself(&self, sheet: &str, cell: CellRef) -> bool {
        self.reads_itself_guarded(sheet, cell, &mut Vec::new())
    }

    /// Whether a cell reads itself, with the cells being read on the way.
    fn reads_itself_guarded(&self, sheet: &str, cell: CellRef, being: &mut Vec<(String, CellRef)>) -> bool {
        let Some(cells) = self.sheets.get(sheet) else { return false };
        let Some(entry) = cells.get(cell) else { return false };
        let Some(formula) = &entry.formula else { return false };
        let Ok(expression) = parse(formula) else { return false };
        if being.contains(&(sheet.to_string(), cell)) {
            return true;
        }
        being.push((sheet.to_string(), cell));
        let mut found = false;
        for reference in precedents(&expression, &self.names) {
            let name = reference.sheet.as_deref().unwrap_or(sheet);
            for row in reference.area.start.row..=reference.area.end.row {
                for col in reference.area.start.col..=reference.area.end.col {
                    let other = CellRef { col, row };
                    if name == sheet && other == cell {
                        found = true;
                        break;
                    }
                    if self.reads_itself_guarded(name, other, being) {
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }
        being.pop();
        found
    }

    /// The sheet of that name, for writing.
    pub fn sheet_mut(&mut self, name: &str) -> Option<&mut Cells> {
        self.sheets.get_mut(name)
    }

    /// The **cells** of a sheet.
    pub fn sheet(&self, name: &str) -> Option<&Cells> {
        self.sheets.get(name)
    }

    /// The **sheets** of the book, by name.
    pub fn sheets(&self) -> impl Iterator<Item = (&String, &Cells)> {
        self.sheets.iter()
    }

    /// The **value** of a cell in a sheet.
    pub fn value_of(&self, sheet: &str, cell: CellRef) -> Value {
        self.value_of_guarded(sheet, cell, &mut Vec::new())
    }

    /// The value of a cell, with the cells being computed on the way.
    pub(crate) fn value_of_guarded(
        &self,
        sheet: &str,
        cell: CellRef,
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let Some(cells) = self.sheets.get(sheet) else {
            return Value::Error(format!("there is no sheet `{sheet}`"));
        };
        let Some(entry) = cells.get(cell) else { return Value::Empty };
        if let Some(value) = &entry.value {
            return value.clone();
        }
        let Some(formula) = &entry.formula else { return Value::Empty };
        if being.contains(&(sheet.to_string(), cell)) {
            return Value::Error(format!("{sheet}!{} reads itself", cell.to_text()));
        }
        let expression = match parse(formula) {
            Ok(expression) => expression,
            Err(error) => return Value::Error(error.message),
        };
        being.push((sheet.to_string(), cell));
        let value = self.evaluate(sheet, &expression, being);
        being.pop();
        value
    }

    /// Evaluate an expression in a sheet.
    pub fn evaluate(&self, sheet: &str, expression: &Expr, being: &mut Vec<(String, CellRef)>) -> Value {
        match expression {
            Expr::Number(number) => Value::Number(*number),
            Expr::Text(text) => Value::Text(text.clone()),
            Expr::Bool(value) => Value::Bool(*value),
            Expr::Error(message) => Value::Error(message.clone()),
            Expr::Cell { sheet: named, anchor } => {
                let name = named.as_deref().unwrap_or(sheet);
                self.value_of_guarded(name, anchor.cell, being)
            }
            Expr::Range { sheet: named, start, end } => {
                let name = named.as_deref().unwrap_or(sheet);
                if !self.sheets.contains_key(name) {
                    return Value::Error(format!("there is no sheet `{name}`"));
                }
                let area = Area::new(start.cell, end.cell);
                let mut values = Vec::new();
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        values.push(self.value_of_guarded(name, CellRef { col, row }, being));
                    }
                }
                Value::Text(format!("{} values", values.len()))
            }
            Expr::Name(name) => {
                let Some(name) = self.names.get(name) else {
                    return Value::Error(format!("there is no name `{name}`"));
                };
                let area = name.area;
                let mut values = Vec::new();
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        values.push(self.value_of_guarded(&name.sheet, CellRef { col, row }, being));
                    }
                }
                Value::Text(format!("{} values", values.len()))
            }
            Expr::Negate(inner) => match self.evaluate(sheet, inner, being) {
                Value::Error(message) => Value::Error(message),
                value => match value.as_number() {
                    Some(number) => Value::Number(-number),
                    None => Value::Error("#VALUE!".to_string()),
                },
            },
            Expr::Binary(operator, left, right) => {
                let left = self.evaluate(sheet, left, being);
                let right = self.evaluate(sheet, right, being);
                binary(operator, &left, &right)
            }
            Expr::Call(name, arguments) => self.call(sheet, name, arguments, being),
        }
    }

    /// Call a function.
    fn call(&self, sheet: &str, name: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        // An argument that is an **error** makes the call an error, and
        // the error is the one the argument says.
        for argument in arguments {
            if let Expr::Error(message) = argument {
                return Value::Error(message.clone());
            }
        }
        if name == "TEXT" {
            return self.text_call(sheet, arguments, being);
        }
        if name == "SUBTOTAL" {
            return self.subtotal(sheet, arguments, being);
        }
        if name == "IF" {
            return self.if_call(sheet, arguments, being);
        }
        if name == "COUNTIF" {
            return self.countif(sheet, arguments, being);
        }
        if name == "SUMIF" {
            return self.sumif(sheet, arguments, being);
        }
        if crate::aggregates::is_aggregate_function(name) {
            return self.aggregates(sheet, name, arguments, being);
        }
        if crate::texts::is_text_function(name) {
            return self.shaping(sheet, name, arguments, being);
        }
        if crate::lookups::is_lookup_function(name) {
            return self.looking(sheet, name, arguments, being);
        }
        if crate::datetimes::is_date_function(name) {
            return self.dates(sheet, name, arguments, being);
        }
        if crate::statistics::is_statistical_function(name) {
            return self.statistics(sheet, name, arguments, being);
        }
        if crate::arithmetic::is_arithmetic_function(name) {
            return self.arithmetic(sheet, name, arguments, being);
        }
        if crate::trigonometry::is_trigonometric_function(name) {
            return self.trigonometry(sheet, name, arguments, being);
        }
        if crate::finance::is_financial_function(name) {
            return self.finance(sheet, name, arguments, being);
        }
        if crate::depreciation::is_depreciation_function(name) {
            return self.depreciation(sheet, name, arguments, being);
        }
        if crate::securities::is_security_function(name) {
            return self.securities(sheet, name, arguments, being);
        }
        if crate::bonds::is_bond_function(name) {
            return self.bonds(sheet, name, arguments, being);
        }
        if crate::combinations::is_combination_function(name) {
            return self.combinations(sheet, name, arguments, being);
        }
        if crate::engineering::is_engineering_function(name) {
            return self.engineering(sheet, name, arguments, being);
        }
        if crate::information::is_information_function(name) {
            return self.information(sheet, name, arguments, being);
        }
        if crate::logic::is_logical_function(name) {
            return self.logic(sheet, name, arguments, being);
        }
        if crate::regression::is_regression_function(name) {
            return self.regression(sheet, name, arguments, being);
        }
        if crate::distributions::is_distribution_function(name) {
            return self.distributions(sheet, name, arguments, being);
        }
        if crate::cashflows::is_cash_flow_function(name) {
            return self.cash_flows(sheet, name, arguments, being);
        }
        if crate::words::is_word_function(name) {
            return self.words(sheet, name, arguments, being);
        }
        if crate::database::is_database_function(name) {
            return self.database(sheet, name, arguments, being);
        }
        if crate::hypotheses::is_hypothesis_function(name) {
            return self.hypotheses(sheet, name, arguments, being);
        }
        if crate::textvalue::is_text_value_function(name) {
            return self.text_value(sheet, name, arguments, being);
        }
        // The tree's **arrays**: a function that gives back a table,
        // and a formula here is a value.
        if crate::arrays::is_array_function(name) {
            return Value::Error("not supported yet: an array".to_string());
        }
        if crate::matrices::is_matrix_function(name) {
            return self.matrices(sheet, name, arguments, being);
        }
        if crate::complex::is_complex_function(name) {
            return self.complex(sheet, name, arguments, being);
        }
        if name == "CONVERT" {
            return self.convert(sheet, arguments, being);
        }
        // A range argument is the numbers of every cell it covers.
        let mut numbers: Vec<f64> = Vec::new();
        let mut plain: Vec<Value> = Vec::new();
        for argument in arguments {
            match argument {
                Expr::Range { sheet: named, start, end } => {
                    let name = named.as_deref().unwrap_or(sheet);
                    if !self.sheets.contains_key(name) {
                        return Value::Error(format!("there is no sheet `{name}`"));
                    }
                    let area = Area::new(start.cell, end.cell);
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let value = self.value_of_guarded(name, CellRef { col, row }, being);
                            if let Some(number) = value.as_number() {
                                numbers.push(number);
                            }
                        }
                    }
                }
                Expr::Name(name) => {
                    let Some(name) = self.names.get(name) else {
                        return Value::Error(format!("there is no name `{name}`"));
                    };
                    let area = name.area;
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let value = self.value_of_guarded(&name.sheet, CellRef { col, row }, being);
                            if let Some(number) = value.as_number() {
                                numbers.push(number);
                            }
                        }
                    }
                }
                other => {
                    let value = self.evaluate(sheet, other, being);
                    if let Some(number) = value.as_number() {
                        numbers.push(number);
                    }
                    plain.push(value);
                }
            }
        }
        match name {
            "SUM" => Value::Number(numbers.iter().sum()),
            "AVERAGE" => {
                if numbers.is_empty() {
                    Value::Error("#DIV/0!".to_string())
                } else {
                    Value::Number(numbers.iter().sum::<f64>() / numbers.len() as f64)
                }
            }
            "MIN" => numbers
                .iter()
                .copied()
                .fold(None, |smallest: Option<f64>, number| {
                    Some(smallest.map_or(number, |smallest| smallest.min(number)))
                })
                .map(Value::Number)
                .unwrap_or(Value::Empty),
            "MAX" => numbers
                .iter()
                .copied()
                .fold(None, |largest: Option<f64>, number| {
                    Some(largest.map_or(number, |largest| largest.max(number)))
                })
                .map(Value::Number)
                .unwrap_or(Value::Empty),
            "COUNT" => Value::Number(numbers.len() as f64),
            "COUNTA" => Value::Number((numbers.len() + plain.len()) as f64),
            other => Value::Error(format!("there is no function `{other}`")),
        }
    }

    /// The tree's `IF`: a **choice** between two values.    ///
    /// The tree's rule, in the tree's own words:
    ///
    /// - the **condition** is a value, and its truth is asked of it: an
    ///   empty value is false, a number is true when it is not zero, a
    ///   boolean is itself, and the text `TRUE` or `FALSE` is the
    ///   boolean it names. Any other text is an error, and so is an
    ///   error that is the condition.
    /// - **only the branch that is taken is computed.** The tree
    ///   computes the condition, and then the branch it names and
    ///   nothing else — so a formula like `IF(A1=0,"",1/A1)` does not
    ///   divide by zero when the condition is false. So does this one.
    /// - a formula of **one argument** is the truth of its condition:
    ///   `IF(A1>2)` is `TRUE` when the condition holds and `FALSE`
    ///   when it does not.
    /// - a formula of **two arguments** has no false branch, and the
    ///   false answer is `FALSE`.
    /// - a formula of **more than three** arguments is an error.
    fn if_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.is_empty() || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let condition = self.evaluate(sheet, &arguments[0], being);
        let truth = match truth_of(&condition) {
            Ok(truth) => truth,
            Err(error) => return Value::Error(error),
        };
        let branch = if truth { arguments.get(1) } else { arguments.get(2) };
        match branch {
            Some(expression) => self.evaluate(sheet, expression, being),
            None => Value::Bool(truth),
        }
    }

    /// The **area** a range argument stands for: the sheet it is in
    /// and the cells it covers.
    ///
    /// A range names a sheet or is in the sheet the formula is in, and
    /// a name stands for a range of cells.
    /// The **numbers** of an argument: the numbers of every cell of a
    /// range, and the number of a plain argument.
    ///
    /// The text of a range and of a plain argument does not count, and
    /// a plain argument that is an error is the error.
    pub(crate) fn numbers_of(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<Vec<f64>, String> {
        let Some(argument) = arguments.get(index) else { return Ok(Vec::new()) };
        match argument {
            Expr::Range { .. } | Expr::Name(_) => {
                let (name, area) = self.area_of(sheet, argument)?;
                let mut numbers = Vec::new();
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                        if let Some(number) = value.as_number() {
                            numbers.push(number);
                        }
                    }
                }
                Ok(numbers)
            }
            other => {
                let value = self.evaluate(sheet, other, being);
                if let Value::Error(message) = &value {
                    return Err(message.clone());
                }
                Ok(value.as_number().map(|number| vec![number]).unwrap_or_default())
            }
        }
    }

    /// The **number** an argument says.
    /// The **text** an argument says, if it says one.
    pub(crate) fn text_at(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<String, String> {
        let Some(argument) = arguments.get(index) else { return Err("#VALUE!".to_string()) };
        let value = self.evaluate(sheet, argument, being);
        crate::texts::value_text(&value)
    }

    /// The **values** an argument holds: the cells of a range, and the
    /// value of a plain argument.
    pub(crate) fn values_of_cells(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<Vec<Value>, String> {
        match argument {
            Expr::Range { .. } | Expr::Name(_) => {
                let (name, area) = self.area_of(sheet, argument)?;
                let mut values = Vec::new();
                for row in area.start.row..=area.end.row {
                    for col in area.start.col..=area.end.col {
                        values.push(self.value_of_guarded(&name, CellRef { col, row }, being));
                    }
                }
                Ok(values)
            }
            other => Ok(vec![self.evaluate(sheet, other, being)]),
        }
    }

    /// The **number** an argument says, if the argument is there.
    ///
    /// An argument that is not there is **none** and not a zero: a
    /// function with a default says what the default is.
    pub(crate) fn optional_number(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<Option<f64>, String> {
        match arguments.get(index) {
            Some(_) => Ok(Some(self.number_at(sheet, arguments, index, being)?)),
            None => Ok(None),
        }
    }

    pub(crate) fn number_at(
        &self,
        sheet: &str,
        arguments: &[Expr],
        index: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<f64, String> {
        let Some(argument) = arguments.get(index) else { return Err("#VALUE!".to_string()) };
        let value = self.evaluate(sheet, argument, being);
        if let Value::Error(message) = &value {
            return Err(message.clone());
        }
        value.as_number().ok_or_else(|| "#VALUE!".to_string())
    }

    pub(crate) fn area_of(&self, sheet: &str, argument: &Expr) -> Result<(String, Area), String> {
        match argument {
            Expr::Range { sheet: named, start, end } => {
                let name = named.clone().unwrap_or_else(|| sheet.to_string());
                if !self.sheets.contains_key(&name) {
                    return Err(format!("there is no sheet `{name}`"));
                }
                Ok((name, Area::new(start.cell, end.cell)))
            }
            Expr::Name(name) => {
                let Some(name) = self.names.get(name) else {
                    return Err(format!("there is no name `{name}`"));
                };
                Ok((name.sheet.clone(), name.area))
            }
            _ => Err("a counting function takes a range of cells".to_string()),
        }
    }

    /// The tree's `NUMBERVALUE`: a text read as a number, with the
    /// **separators** of the text's own.
    ///
    /// The tree's rule: the **decimal separator** and the **group
    /// separator** are the first character of the second and the third
    /// arguments — a point and a comma when they are not given — and
    /// they may not be the same character.
    ///
    /// The **spaces** in the text are not there, a text that is empty
    /// is zero, and the **percent signs** at the end of it make the
    /// number a hundredth of itself — one for each of them.
    fn numbervalue(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
        text: String,
    ) -> Value {
        let decimal = match arguments.get(1) {
            Some(expression) => {
                let value = self.evaluate(sheet, expression, being);
                match crate::texts::value_text(&value) {
                    Ok(text) => match text.chars().next() {
                        Some(character) => character,
                        None => return Value::Error("#VALUE!".to_string()),
                    },
                    Err(message) => return Value::Error(message),
                }
            }
            None => '.',
        };
        let group = match arguments.get(2) {
            Some(expression) => {
                let value = self.evaluate(sheet, expression, being);
                match crate::texts::value_text(&value) {
                    Ok(text) => match text.chars().next() {
                        Some(character) => character,
                        None => return Value::Error("#VALUE!".to_string()),
                    },
                    Err(message) => return Value::Error(message),
                }
            }
            None => ',',
        };
        if decimal == group {
            return Value::Error("#VALUE!".to_string());
        }
        let body: String = text.chars().filter(|character| !character.is_whitespace()).collect();
        if body.is_empty() {
            return Value::Number(0.0);
        }
        // The **percent signs** at the end of the text.
        let mut body = body;
        let mut percent = 0;
        while let Some(stripped) = body.strip_suffix('%') {
            body = stripped.to_string();
            percent += 1;
        }
        let mut seen_decimal = false;
        let mut out = String::new();
        for character in body.chars() {
            if character == decimal {
                if seen_decimal {
                    return Value::Error("#VALUE!".to_string());
                }
                seen_decimal = true;
                out.push('.');
            } else if character == group {
                if seen_decimal {
                    return Value::Error("#VALUE!".to_string());
                }
            } else if character == '.' || character == ',' {
                return Value::Error("#VALUE!".to_string());
            } else {
                out.push(character);
            }
        }
        let Some(number) = crate::textvalue::number_of(&out, self.system()) else {
            return Value::Error("#VALUE!".to_string());
        };
        let number = number / 100f64.powi(percent);
        if !number.is_finite() {
            return Value::Error("#NUM!".to_string());
        }
        Value::Number(number)
    }

    /// The tree's `COUNTIF`: the **count** of the cells that meet a
    /// condition.
    ///
    /// The condition is the tree's condition (the module `criteria`),
    /// and it is a **value**: a text criterion is a condition written
    /// out, and a number or a boolean is the value itself.
    ///
    /// A cell that holds an **error** is counted when the error is the
    /// one the condition names, and the condition that is an error does
    /// not stop the count: it is the condition *that the cell holds
    /// that error*.
    fn countif(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let (name, area) = match self.area_of(sheet, &arguments[0]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let condition = self.evaluate(sheet, &arguments[1], being);
        let condition = match crate::criteria::Condition::parse_argument(&condition) {
            Ok(condition) => condition,
            Err(message) => return Value::Error(message),
        };
        let mut count = 0.0;
        for row in area.start.row..=area.end.row {
            for col in area.start.col..=area.end.col {
                let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                if condition.matches(&value) {
                    count += 1.0;
                }
            }
        }
        Value::Number(count)
    }

    /// The tree's `SUMIF`: the **sum** of the cells that meet a
    /// condition.
    ///
    /// The third argument is the **sum range**: the cells that are
    /// added are the ones of the sum range that stand opposite the
    /// cells of the first range. The sum range is sized like the first
    /// range and is anchored at its own top left cell, as the tree's
    /// is, so a sum range that is smaller than the range it is summed
    /// over is read past its own end.
    ///
    /// A cell that holds an **error** and is added is that error: the
    /// sum is not a sum that skips it.
    fn sumif(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let (name, area) = match self.area_of(sheet, &arguments[0]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let condition = self.evaluate(sheet, &arguments[1], being);
        let condition = match crate::criteria::Condition::parse_argument(&condition) {
            Ok(condition) => condition,
            Err(message) => return Value::Error(message),
        };
        let sum_range = match arguments.get(2) {
            Some(argument) => match self.area_of(sheet, argument) {
                Ok(area) => Some(area),
                Err(message) => return Value::Error(message),
            },
            None => None,
        };
        let mut total = 0.0;
        for row in area.start.row..=area.end.row {
            for col in area.start.col..=area.end.col {
                let value = self.value_of_guarded(&name, CellRef { col, row }, being);
                if !condition.matches(&value) {
                    continue;
                }
                let target = match &sum_range {
                    Some((sum_name, sum_area)) => {
                        let cell = CellRef {
                            col: sum_area.start.col + (col - area.start.col),
                            row: sum_area.start.row + (row - area.start.row),
                        };
                        self.value_of_guarded(sum_name, cell, being)
                    }
                    None => value,
                };
                match target {
                    Value::Number(number) => total += number,
                    Value::Error(message) => return Value::Error(message),
                    _ => {}
                }
            }
        }
        Value::Number(total)
    }

    /// The tree's **database functions**: a table of records, a table
    /// of criteria and a field.
    ///
    /// The database and the criteria are ranges of cells, and the
    /// field is a label of the database's first row or a number that
    /// says which column it is.
    fn database(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let database = match self.table_of(sheet, &arguments[0], being) {
            Ok(table) => table,
            Err(message) => return Value::Error(message),
        };
        let criteria = match self.table_of(sheet, &arguments[2], being) {
            Ok(table) => table,
            Err(message) => return Value::Error(message),
        };
        let field = match arguments.get(1) {
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                if let Value::Error(message) = &value {
                    return Value::Error(message.clone());
                }
                Some(value)
            }
            None => None,
        };
        match crate::database::call(name, &database, &criteria, field.as_ref()) {
            Ok(value) => value,
            Err(message) => Value::Error(message),
        }
    }

    /// A **table** of values: the cells of a range, row by row.
    fn table_of(
        &self,
        sheet: &str,
        argument: &Expr,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<crate::database::Table, String> {
        let (name, area) = self.area_of(sheet, argument)?;
        let mut rows = Vec::new();
        for row in area.start.row..=area.end.row {
            let mut cells = Vec::new();
            for col in area.start.col..=area.end.col {
                cells.push(self.value_of_guarded(&name, CellRef { col, row }, being));
            }
            rows.push(cells);
        }
        Ok(rows)
    }

    /// The tree's **tests of a hypothesis**: a t-test, a chi-square
    /// test, an F-test and a z-test.
    fn hypotheses(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "T.TEST" | "TTEST" => {
                if arguments.len() != 4 {
                    return Value::Error("#VALUE!".to_string());
                }
                let (left, right) = match (
                    self.values_of_cells(sheet, &arguments[0], being),
                    self.values_of_cells(sheet, &arguments[1], being),
                ) {
                    (Ok(left), Ok(right)) => (left, right),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let (tails, kind) = match (
                    self.number_at(sheet, arguments, 2, being),
                    self.number_at(sheet, arguments, 3, being),
                ) {
                    (Ok(tails), Ok(kind)) => (tails, kind),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                match crate::hypotheses::t_test(&left, &right, tails, kind) {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                }
            }
            "CHISQ.TEST" | "CHITEST" => {
                if arguments.len() != 2 {
                    return Value::Error("#VALUE!".to_string());
                }
                let (left, right) = match (
                    self.values_of_cells(sheet, &arguments[0], being),
                    self.values_of_cells(sheet, &arguments[1], being),
                ) {
                    (Ok(left), Ok(right)) => (left, right),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                let (rows, columns) = self.shape_of(sheet, &arguments[0]);
                let (other_rows, other_columns) = self.shape_of(sheet, &arguments[1]);
                if rows != other_rows || columns != other_columns {
                    return Value::Error("#N/A".to_string());
                }
                match crate::hypotheses::chisq_test(&left, &right, rows, columns) {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                }
            }
            "F.TEST" | "FTEST" => {
                if arguments.len() != 2 {
                    return Value::Error("#VALUE!".to_string());
                }
                let (left, right) = match (
                    self.values_of_cells(sheet, &arguments[0], being),
                    self.values_of_cells(sheet, &arguments[1], being),
                ) {
                    (Ok(left), Ok(right)) => (left, right),
                    (Err(message), _) | (_, Err(message)) => return Value::Error(message),
                };
                match crate::hypotheses::f_test(&left, &right) {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                }
            }
            "CONFIDENCE" | "CONFIDENCE.NORM" | "CONFIDENCE.T" => {
                if arguments.len() != 3 {
                    return Value::Error("#VALUE!".to_string());
                }
                let (alpha, deviation, size) = match (
                    self.number_at(sheet, arguments, 0, being),
                    self.number_at(sheet, arguments, 1, being),
                    self.number_at(sheet, arguments, 2, being),
                ) {
                    (Ok(alpha), Ok(deviation), Ok(size)) => (alpha, deviation, size.trunc()),
                    (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => {
                        return Value::Error(message);
                    }
                };
                let value = if name == "CONFIDENCE.T" {
                    crate::hypotheses::confidence_t(alpha, deviation, size)
                } else {
                    crate::hypotheses::confidence_norm(alpha, deviation, size)
                };
                match value {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                }
            }
            _ => {
                // Z.TEST: the set, the number, and the deviation.
                if arguments.len() < 2 || arguments.len() > 3 {
                    return Value::Error("#VALUE!".to_string());
                }
                let values = match self.values_of_cells(sheet, &arguments[0], being) {
                    Ok(values) => values,
                    Err(message) => return Value::Error(message),
                };
                let number = match self.number_at(sheet, arguments, 1, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let deviation = match self.optional_number(sheet, arguments, 2, being) {
                    Ok(deviation) => deviation,
                    Err(message) => return Value::Error(message),
                };
                match crate::hypotheses::z_test(&values, number, deviation) {
                    Ok(value) => Value::Number(value),
                    Err(message) => Value::Error(message),
                }
            }
        }
    }

    /// The **shape** of a range: its rows and its columns.
    fn shape_of(&self, sheet: &str, argument: &Expr) -> (usize, usize) {
        match self.area_of(sheet, argument) {
            Ok((_, area)) => (
                usize::try_from(area.end.row - area.start.row + 1).unwrap_or(1),
                usize::try_from(area.end.col - area.start.col + 1).unwrap_or(1),
            ),
            Err(_) => (1, 1),
        }
    }

    /// The tree's **value of a text**: a text that holds a number, a
    /// date or a time.
    fn text_value(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let Some(argument) = arguments.first() else {
            return Value::Error("#VALUE!".to_string());
        };
        let value = self.evaluate(sheet, argument, being);
        if name == "VALUE" {
            return match value {
                Value::Number(number) => Value::Number(number),
                // A blank cell is nothing and nothing is a zero.
                Value::Empty => Value::Number(0.0),
                Value::Error(message) => Value::Error(message),
                Value::Text(text) => match crate::textvalue::number_of(&text, self.system()) {
                    Some(number) => Value::Number(number),
                    None => Value::Error("#VALUE!".to_string()),
                },
                Value::Bool(_) => Value::Error("#VALUE!".to_string()),
            };
        }
        let text = match crate::texts::value_text(&value) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        if name == "NUMBERVALUE" {
            return self.numbervalue(sheet, arguments, being, text);
        }
        let number = if name == "DATEVALUE" {
            crate::textvalue::date_of(&text, self.system())
        } else {
            crate::textvalue::time_of(&text, self.system())
        };
        match number {
            Some(number) => Value::Number(number),
            None => Value::Error("#VALUE!".to_string()),
        }
    }

    /// The tree's **complex numbers**: a pair of numbers, written as
    /// text and read back from text.
    fn complex(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        if name == "COMPLEX" {
            if arguments.len() < 2 || arguments.len() > 3 {
                return Value::Error("#VALUE!".to_string());
            }
            let (real, imaginary) = match (
                self.number_at(sheet, arguments, 0, being),
                self.number_at(sheet, arguments, 1, being),
            ) {
                (Ok(real), Ok(imaginary)) => (real, imaginary),
                (Err(message), _) | (_, Err(message)) => return Value::Error(message),
            };
            let suffix = match arguments.get(2) {
                Some(argument) => {
                    let value = self.evaluate(sheet, argument, being);
                    if let Value::Error(message) = &value {
                        return Value::Error(message.clone());
                    }
                    match crate::texts::value_text(&value) {
                        Ok(text) => text,
                        Err(message) => return Value::Error(message),
                    }
                }
                None => "i".to_string(),
            };
            let suffix = match suffix.as_str() {
                "i" | "" => 'i',
                "j" => 'j',
                _ => return Value::Error("#VALUE!".to_string()),
            };
            return match crate::complex::write(crate::complex::Cx::new(real, imaginary), suffix) {
                Ok(value) => value,
                Err(message) => Value::Error(message),
            };
        }
        let mut values: Vec<Vec<Value>> = Vec::new();
        for argument in arguments {
            match self.values_of_cells(sheet, argument, being) {
                Ok(cells) => values.push(cells),
                Err(message) => return Value::Error(message),
            }
        }
        match crate::complex::call(name, &values) {
            Ok(value) => value,
            Err(message) => Value::Error(message),
        }
    }
}

/// The **truth** of a value: whether it holds or not.
///
/// The tree's rule: an empty value is false, a number is true when it
/// is not zero, a boolean is itself, and the text `TRUE` or `FALSE` is
/// the boolean it names. Any other text is an error, and so is an
/// error that is asked for its truth.
pub(crate) fn truth_of(value: &Value) -> Result<bool, String> {
    match value {
        Value::Empty => Ok(false),
        Value::Number(number) => Ok(*number != 0.0),
        Value::Bool(flag) => Ok(*flag),
        Value::Text(text) => {
            if text.eq_ignore_ascii_case("TRUE") {
                Ok(true)
            } else if text.eq_ignore_ascii_case("FALSE") {
                Ok(false)
            } else {
                Err("#VALUE!".to_string())
            }
        }
        Value::Error(message) => Err(message.clone()),
    }
}

impl Book {
    /// The tree's `TEXT`: a value written with a **format code**.
    ///
    /// The code is the second argument and it is **evaluated**, so it
    /// may be a cell that holds a code. A code that this tree does not
    /// have is an error: the tree's `TEXT` takes the tree's whole
    /// format language, and this one takes the sixteen codes it has.
    /// The **subtotal** of a range: the tree's `SUBTOTAL`.
    ///
    /// The tree's rule, in the tree's own words:
    ///
    /// - the **function number** is one of the tree's eleven, and the
    ///   hundred-and-more form (`101` to `111`) is the same function
    ///   with the **hidden rows left out** as well;
    /// - a cell that holds a **nested subtotal** is left out, always —
    ///   in the hundred-and-more form too;
    /// - a row a **filter** hides is left out, whatever the function
    ///   number. This tree has no filter, so there is no such row.
    ///
    /// And the tree's **eleven** functions are all here: `AVERAGE`,
    /// `COUNT`, `COUNTA`, `MAX`, `MIN`, `PRODUCT`, `STDEV`, `STDEVP`,
    /// `SUM`, `VAR` and `VARP`. A function number that is not one of
    /// them is an error that says so.
    fn subtotal(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 {
            return Value::Error("a subtotal is a function number and a range".to_string());
        }
        let code = match &arguments[0] {
            Expr::Number(number) => *number,
            other => match self.evaluate(sheet, other, being).as_number() {
                Some(number) => number,
                None => return Value::Error("a subtotal's function number is a number".to_string()),
            },
        };
        if !code.is_finite() || code.fract() != 0.0 {
            return Value::Error(format!("a subtotal's function number is a whole number: {code}"));
        }
        let code = code as i64;
        if !(1..=11).contains(&code) && !(101..=111).contains(&code) {
            return Value::Error(format!(
                "a subtotal's function number is 1 to 11 or 101 to 111, and {code} is not"
            ));
        }
        let skip_hidden = code > 100;
        let function = match code % 100 {
            1 => "AVERAGE",
            2 => "COUNT",
            3 => "COUNTA",
            4 => "MAX",
            5 => "MIN",
            6 => "PRODUCT",
            7 => "STDEV",
            8 => "STDEVP",
            9 => "SUM",
            10 => "VAR",
            11 => "VARP",
            other => {
                return Value::Error(format!(
                    "the function number {other} is not one this tree has: the numbers are 1 (AVERAGE), 2 (COUNT), 3 (COUNTA), 4 (MAX), 5 (MIN), 6 (PRODUCT), 7 (STDEV), 8 (STDEVP), 9 (SUM), 10 (VAR) and 11 (VARP)"
                ));
            }
        };
        // The **cells** of the ranges: a row a filter hides, a hidden
        // row and a nested subtotal are left out.
        let mut filtered: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        let mut numbers: Vec<f64> = Vec::new();
        let mut plain: Vec<Value> = Vec::new();
        for argument in &arguments[1..] {
            match argument {
                Expr::Range { sheet: named, start, end } => {
                    let name = named.as_deref().unwrap_or(sheet);
                    if !self.sheets.contains_key(name) {
                        return Value::Error(format!("there is no sheet `{name}`"));
                    }
                    let area = Area::new(start.cell, end.cell);
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let cell = CellRef { col, row };
                            let rows =
                                filtered.entry(name.to_string()).or_insert_with(|| self.filtered_rows(name));
                            if self.left_out(name, cell, skip_hidden, rows) {
                                continue;
                            }
                            let value = self.value_of_guarded(name, cell, being);
                            if let Some(number) = value.as_number() {
                                numbers.push(number);
                            } else if !matches!(value, Value::Empty) {
                                plain.push(value);
                            }
                        }
                    }
                }
                Expr::Name(name) => {
                    let Some(name) = self.names.get(name) else {
                        return Value::Error(format!("there is no name `{name}`"));
                    };
                    let area = name.area;
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let cell = CellRef { col, row };
                            let rows = filtered
                                .entry(name.sheet.clone())
                                .or_insert_with(|| self.filtered_rows(&name.sheet));
                            if self.left_out(&name.sheet, cell, skip_hidden, rows) {
                                continue;
                            }
                            let value = self.value_of_guarded(&name.sheet, cell, being);
                            if let Some(number) = value.as_number() {
                                numbers.push(number);
                            } else if !matches!(value, Value::Empty) {
                                plain.push(value);
                            }
                        }
                    }
                }
                other => {
                    let value = self.evaluate(sheet, other, being);
                    if let Some(number) = value.as_number() {
                        numbers.push(number);
                    } else if !matches!(value, Value::Empty) {
                        plain.push(value);
                    }
                }
            }
        }
        // The **aggregate**.
        match function {
            "SUM" => Value::Number(numbers.iter().sum()),
            "AVERAGE" => {
                if numbers.is_empty() {
                    Value::Error("#DIV/0!".to_string())
                } else {
                    Value::Number(numbers.iter().sum::<f64>() / numbers.len() as f64)
                }
            }
            "MIN" => numbers
                .iter()
                .copied()
                .fold(None, |smallest: Option<f64>, number| {
                    Some(smallest.map_or(number, |smallest| smallest.min(number)))
                })
                .map(Value::Number)
                .unwrap_or(Value::Empty),
            "MAX" => numbers
                .iter()
                .copied()
                .fold(None, |largest: Option<f64>, number| {
                    Some(largest.map_or(number, |largest| largest.max(number)))
                })
                .map(Value::Number)
                .unwrap_or(Value::Empty),
            "COUNT" => Value::Number(numbers.len() as f64),
            "COUNTA" => Value::Number((numbers.len() + plain.len()) as f64),
            "PRODUCT" => {
                // The tree's `PRODUCT` of nothing is nothing.
                if numbers.is_empty() { Value::Number(0.0) } else { Value::Number(numbers.iter().product()) }
            }
            "STDEV" => match crate::statistics::variance(&numbers, true) {
                Ok(variance) => Value::Number(variance.sqrt()),
                Err(message) => Value::Error(message),
            },
            "STDEVP" => match crate::statistics::variance(&numbers, false) {
                Ok(variance) => Value::Number(variance.sqrt()),
                Err(message) => Value::Error(message),
            },
            "VAR" => match crate::statistics::variance(&numbers, true) {
                Ok(variance) => Value::Number(variance),
                Err(message) => Value::Error(message),
            },
            "VARP" => match crate::statistics::variance(&numbers, false) {
                Ok(variance) => Value::Number(variance),
                Err(message) => Value::Error(message),
            },
            _ => Value::Error(format!("there is no function `{function}`")),
        }
    }

    /// Whether a cell is **left out** of a subtotal: a row a **filter**
    /// hides, a row hidden **by hand** and the subtotal is the
    /// hundred-and-more form, or a cell that holds a nested subtotal.
    fn left_out(&self, sheet: &str, cell: CellRef, skip_hidden: bool, filtered: &[u64]) -> bool {
        // The rows of a sheet are counted from one and a cell's row is
        // counted from nothing.
        let row = cell.row as u64 + 1;
        // The rows a filter hides are left out whatever the function
        // number: that is the tree's rule.
        if filtered.contains(&row) {
            return true;
        }
        if skip_hidden && self.row_is_hidden(sheet, row) {
            return true;
        }
        let Some(cells) = self.sheets.get(sheet) else { return false };
        let Some(entry) = cells.get(cell) else { return false };
        let Some(formula) = entry.formula.as_deref() else { return false };
        formula.to_ascii_uppercase().contains("SUBTOTAL(")
    }

    fn text_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() != 2 {
            return Value::Error("#VALUE!".to_string());
        }
        let value = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &value {
            return Value::Error(message.clone());
        }
        let code = self.evaluate(sheet, &arguments[1], being);
        let Value::Text(code) = code else {
            return Value::Error("#VALUE!".to_string());
        };
        match Format::parse(&code) {
            Some(format) => Value::Text(format.text(&value, self.system)),
            None => Value::Error(format!("there is no format `{code}`")),
        }
    }
}

/// A binary operation on two values.
fn binary(operator: &str, left: &Value, right: &Value) -> Value {
    if let Value::Error(message) = left {
        return Value::Error(message.clone());
    }
    if let Value::Error(message) = right {
        return Value::Error(message.clone());
    }
    if operator == "=" || operator == "<>" {
        let same = match (left, right) {
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            _ => match (left.as_number(), right.as_number()) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            },
        };
        let result = if operator == "=" { same } else { !same };
        return Value::Bool(result);
    }
    if operator == "<" || operator == "<=" || operator == ">" || operator == ">=" {
        let (Some(a), Some(b)) = (left.as_number(), right.as_number()) else {
            return Value::Error("#VALUE!".to_string());
        };
        let result = match operator {
            "<" => a < b,
            "<=" => a <= b,
            ">" => a > b,
            _ => a >= b,
        };
        return Value::Bool(result);
    }
    let (Some(a), Some(b)) = (left.as_number(), right.as_number()) else {
        return Value::Error("#VALUE!".to_string());
    };
    let result = match operator {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => {
            if b == 0.0 {
                return Value::Error("#DIV/0!".to_string());
            }
            a / b
        }
        "^" => a.powf(b),
        _ => return Value::Error(format!("there is no operation `{operator}`")),
    };
    if !result.is_finite() {
        return Value::Error("#NUM!".to_string());
    }
    Value::Number(result)
}

// ---------------------------------------------------------------------------
// The printer: a formula written out again.
// ---------------------------------------------------------------------------

/// The precedence of a **comparison**.
const COMPARISON: u8 = 1;
/// The precedence of a **sum** or a **difference**.
const ADDITIVE: u8 = 2;
/// The precedence of a **product** or a **quotient**.
const MULTIPLICATIVE: u8 = 3;
/// The precedence of a **power**.
const POWER: u8 = 4;
/// The precedence of a **negation**.
const UNARY: u8 = 5;

impl Expr {
    /// The expression with the **sheet** renamed in it.
    ///
    /// The tree's `adjust::rename_sheet`: every reference that names
    /// the sheet is given the new name. The tree's comparison
    /// **ignores case**, and so does this one.
    ///
    /// A name that is **sheet-qualified** — `Old!MyName` — is renamed
    /// too, and the printer adds the quotes when the new name needs
    /// them.
    pub fn renamed_sheet(&self, old: &str, new: &str) -> Expr {
        match self {
            Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => self.clone(),
            Expr::Cell { sheet, anchor } => Expr::Cell {
                sheet: sheet.as_ref().map(|sheet| {
                    if sheet.eq_ignore_ascii_case(old) { new.to_string() } else { sheet.clone() }
                }),
                anchor: *anchor,
            },
            Expr::Range { sheet, start, end } => Expr::Range {
                sheet: sheet.as_ref().map(|sheet| {
                    if sheet.eq_ignore_ascii_case(old) { new.to_string() } else { sheet.clone() }
                }),
                start: *start,
                end: *end,
            },
            Expr::Name(name) => Expr::Name(name.clone()),
            Expr::Call(name, arguments) => Expr::Call(
                name.clone(),
                arguments.iter().map(|argument| argument.renamed_sheet(old, new)).collect(),
            ),
            Expr::Negate(inner) => Expr::Negate(Box::new(inner.renamed_sheet(old, new))),
            Expr::Binary(operator, left, right) => Expr::Binary(
                operator.clone(),
                Box::new(left.renamed_sheet(old, new)),
                Box::new(right.renamed_sheet(old, new)),
            ),
        }
    }

    /// The expression with its **relative references moved**.
    ///
    /// A reference that is **anchored** — the `$` before the letters or
    /// before the number — does not move, and a reference that is not
    /// anchored moves with the cell that holds it. A reference that
    /// would leave the sheet is gone, and the expression is gone with
    /// it: `None` says that the formula cannot be moved.
    /// The expression with its **relative references moved**.
    ///
    /// A reference that is **anchored** — the `$` before the letters or
    /// before the number — does not move, and a reference that is not
    /// anchored moves with the cell that holds it. A reference that
    /// would leave the sheet becomes the **error** `#REF!`, which is
    /// what the tree writes there.
    pub fn shifted(&self, row_delta: i64, col_delta: i64) -> Expr {
        match self {
            Expr::Cell { sheet, anchor } => match anchor.shifted(row_delta, col_delta) {
                Some(anchor) => Expr::Cell { sheet: sheet.clone(), anchor },
                None => Expr::Error(REF_ERROR.to_string()),
            },
            Expr::Range { sheet, start, end } => {
                match (start.shifted(row_delta, col_delta), end.shifted(row_delta, col_delta)) {
                    (Some(start), Some(end)) => Expr::Range { sheet: sheet.clone(), start, end },
                    _ => Expr::Error(REF_ERROR.to_string()),
                }
            }
            Expr::Name(name) => Expr::Name(name.clone()),
            Expr::Call(name, arguments) => Expr::Call(
                name.clone(),
                arguments.iter().map(|argument| argument.shifted(row_delta, col_delta)).collect(),
            ),
            Expr::Negate(inner) => Expr::Negate(Box::new(inner.shifted(row_delta, col_delta))),
            Expr::Binary(operator, left, right) => Expr::Binary(
                operator.clone(),
                Box::new(left.shifted(row_delta, col_delta)),
                Box::new(right.shifted(row_delta, col_delta)),
            ),
            Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => self.clone(),
        }
    }

    /// The expression with **rows inserted** at `at`.
    ///
    /// A reference that names **another sheet** does not move: the
    /// rows of that sheet are not the rows that were inserted. A
    /// reference that names this sheet — or names none — moves when it
    /// is at or below the row the rows are inserted at, and when its
    /// row is not anchored.
    pub fn inserted(&self, sheet: &str, at: u64, count: u64) -> Expr {
        let names_this_sheet = |named: &Option<String>| match named {
            None => true,
            Some(name) => name.eq_ignore_ascii_case(sheet),
        };
        match self {
            Expr::Cell { sheet: named, anchor } => {
                if names_this_sheet(named) {
                    Expr::Cell { sheet: named.clone(), anchor: anchor.inserted(at, count) }
                } else {
                    self.clone()
                }
            }
            Expr::Range { sheet: named, start, end } => {
                if names_this_sheet(named) {
                    Expr::Range {
                        sheet: named.clone(),
                        start: start.inserted(at, count),
                        end: end.inserted(at, count),
                    }
                } else {
                    self.clone()
                }
            }
            Expr::Name(name) => Expr::Name(name.clone()),
            Expr::Call(name, arguments) => Expr::Call(
                name.clone(),
                arguments.iter().map(|argument| argument.inserted(sheet, at, count)).collect(),
            ),
            Expr::Negate(inner) => Expr::Negate(Box::new(inner.inserted(sheet, at, count))),
            Expr::Binary(operator, left, right) => Expr::Binary(
                operator.clone(),
                Box::new(left.inserted(sheet, at, count)),
                Box::new(right.inserted(sheet, at, count)),
            ),
            Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => self.clone(),
        }
    }

    /// The expression with the rows **deleted** at `at`.
    ///
    /// A reference that names **another sheet** does not move: the
    /// rows of that sheet are not the rows that are deleted. A
    /// reference that names this sheet — or names none — moves up when
    /// it is below the rows that are deleted, and `None` says that a
    /// reference names a row that is gone.
    /// The expression with the rows **deleted** at `at`.
    ///
    /// A reference that names **another sheet** does not move: the
    /// rows of that sheet are not the rows that are deleted. A
    /// reference that names this sheet — or names none — moves up when
    /// it is below the rows that are deleted, and a reference that
    /// names a row that is gone becomes the **error** `#REF!`, which
    /// is what the tree writes there.
    pub fn deleted(&self, sheet: &str, at: u64, count: u64) -> Expr {
        let names_this_sheet = |named: &Option<String>| match named {
            None => true,
            Some(name) => name.eq_ignore_ascii_case(sheet),
        };
        match self {
            Expr::Cell { sheet: named, anchor } => {
                if !names_this_sheet(named) {
                    return self.clone();
                }
                match anchor.deleted(at, count) {
                    Some(anchor) => Expr::Cell { sheet: named.clone(), anchor },
                    None => Expr::Error(REF_ERROR.to_string()),
                }
            }
            Expr::Range { sheet: named, start, end } => {
                if !names_this_sheet(named) {
                    return self.clone();
                }
                match (start.deleted(at, count), end.deleted(at, count)) {
                    (Some(start), Some(end)) => Expr::Range { sheet: named.clone(), start, end },
                    _ => Expr::Error(REF_ERROR.to_string()),
                }
            }
            Expr::Name(name) => Expr::Name(name.clone()),
            Expr::Call(name, arguments) => Expr::Call(
                name.clone(),
                arguments.iter().map(|argument| argument.deleted(sheet, at, count)).collect(),
            ),
            Expr::Negate(inner) => Expr::Negate(Box::new(inner.deleted(sheet, at, count))),
            Expr::Binary(operator, left, right) => Expr::Binary(
                operator.clone(),
                Box::new(left.deleted(sheet, at, count)),
                Box::new(right.deleted(sheet, at, count)),
            ),
            Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => self.clone(),
        }
    }

    /// The expression with **columns inserted** at `at`.
    ///
    /// A reference that names **another sheet** does not move: the
    /// columns of that sheet are not the columns that were inserted. A
    /// reference that names this sheet — or names none — moves when it
    /// is at or to the right of the column the columns are inserted at,
    /// and when its column is not anchored.
    pub fn inserted_column(&self, sheet: &str, at: u64, count: u64) -> Expr {
        let names_this_sheet = |named: &Option<String>| match named {
            None => true,
            Some(name) => name.eq_ignore_ascii_case(sheet),
        };
        match self {
            Expr::Cell { sheet: named, anchor } => {
                if names_this_sheet(named) {
                    Expr::Cell { sheet: named.clone(), anchor: anchor.inserted_column(at, count) }
                } else {
                    self.clone()
                }
            }
            Expr::Range { sheet: named, start, end } => {
                if names_this_sheet(named) {
                    Expr::Range {
                        sheet: named.clone(),
                        start: start.inserted_column(at, count),
                        end: end.inserted_column(at, count),
                    }
                } else {
                    self.clone()
                }
            }
            Expr::Name(name) => Expr::Name(name.clone()),
            Expr::Call(name, arguments) => Expr::Call(
                name.clone(),
                arguments.iter().map(|argument| argument.inserted_column(sheet, at, count)).collect(),
            ),
            Expr::Negate(inner) => Expr::Negate(Box::new(inner.inserted_column(sheet, at, count))),
            Expr::Binary(operator, left, right) => Expr::Binary(
                operator.clone(),
                Box::new(left.inserted_column(sheet, at, count)),
                Box::new(right.inserted_column(sheet, at, count)),
            ),
            Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => self.clone(),
        }
    }

    /// The expression with the columns **deleted** at `at`.
    ///
    /// A reference that names **another sheet** does not move: the
    /// columns of that sheet are not the columns that are deleted. A
    /// reference that names this sheet — or names none — moves left
    /// when it is to the right of the columns that are deleted, and
    /// `None` says that a reference names a column that is gone.
    /// The expression with the columns **deleted** at `at`.
    ///
    /// A reference that names **another sheet** does not move: the
    /// columns of that sheet are not the columns that are deleted. A
    /// reference that names this sheet — or names none — moves left
    /// when it is to the right of the columns that are deleted, and a
    /// reference that names a column that is gone becomes the
    /// **error** `#REF!`, which is what the tree writes there.
    pub fn deleted_column(&self, sheet: &str, at: u64, count: u64) -> Expr {
        let names_this_sheet = |named: &Option<String>| match named {
            None => true,
            Some(name) => name.eq_ignore_ascii_case(sheet),
        };
        match self {
            Expr::Cell { sheet: named, anchor } => {
                if !names_this_sheet(named) {
                    return self.clone();
                }
                match anchor.deleted_column(at, count) {
                    Some(anchor) => Expr::Cell { sheet: named.clone(), anchor },
                    None => Expr::Error(REF_ERROR.to_string()),
                }
            }
            Expr::Range { sheet: named, start, end } => {
                if !names_this_sheet(named) {
                    return self.clone();
                }
                match (start.deleted_column(at, count), end.deleted_column(at, count)) {
                    (Some(start), Some(end)) => Expr::Range { sheet: named.clone(), start, end },
                    _ => Expr::Error(REF_ERROR.to_string()),
                }
            }
            Expr::Name(name) => Expr::Name(name.clone()),
            Expr::Call(name, arguments) => Expr::Call(
                name.clone(),
                arguments.iter().map(|argument| argument.deleted_column(sheet, at, count)).collect(),
            ),
            Expr::Negate(inner) => Expr::Negate(Box::new(inner.deleted_column(sheet, at, count))),
            Expr::Binary(operator, left, right) => Expr::Binary(
                operator.clone(),
                Box::new(left.deleted_column(sheet, at, count)),
                Box::new(right.deleted_column(sheet, at, count)),
            ),
            Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => self.clone(),
        }
    }

    /// The formula's **text**: the expression written out.
    ///
    /// The tree's printer writes an expression back out, and a copied
    /// formula is written out again. Here the text of a formula is what
    /// a cell holds, so the printer is not a nicety: it is the way a
    /// formula is written down twice.
    pub fn to_text(&self) -> String {
        expression_text(self, 0)
    }
}

/// The formula's **text** with the sheet renamed in it, if it names it.
///
/// The formula is **text**, so the text is parsed, the sheet is named
/// again and the expression is printed. A formula that does not name
/// the sheet is left exactly as the caller wrote it.
pub fn rename_in_formula(text: &str, old: &str, new: &str) -> Option<String> {
    let expression = parse(text).ok()?;
    let renamed = expression.renamed_sheet(old, new);
    if renamed == expression {
        return None;
    }
    let printed = renamed.to_text();
    // The `=` is the caller's, and it is kept: a formula that was
    // written with one keeps it, and one that was not does not get
    // one.
    if text.trim_start().starts_with('=') { Some(format!("={printed}")) } else { Some(printed) }
}

/// A formula's **text** with its relative references moved.
///
/// The text is parsed, the references are moved and the expression is
/// printed. A reference that would leave the sheet becomes the error
/// `#REF!`. A formula that does not parse is `None`: a formula that
/// cannot be read is not moved.
pub fn shift_in_formula(text: &str, row_delta: i64, col_delta: i64) -> Option<String> {
    let expression = parse(text).ok()?;
    let shifted = expression.shifted(row_delta, col_delta);
    let printed = shifted.to_text();
    Some(if text.trim_start().starts_with('=') { format!("={printed}") } else { printed })
}

/// The formula's **text** with **rows inserted** at `at`.
///
/// The text is parsed, the rows are inserted and the expression is
/// printed. A formula that does not parse is `None`: a formula that
/// cannot be read is not changed.
pub fn insert_in_formula(text: &str, sheet: &str, at: u64, count: u64) -> Option<String> {
    let expression = parse(text).ok()?;
    let inserted = expression.inserted(sheet, at, count);
    let printed = inserted.to_text();
    if text.trim_start().starts_with('=') { Some(format!("={printed}")) } else { Some(printed) }
}

/// The formula's **text** with the rows **deleted** at `at`.
///
/// The text is parsed, the rows are deleted and the expression is
/// printed. A reference that names a row that is gone becomes the
/// error `#REF!`. A formula that does not parse is `None`: a formula
/// that cannot be read is left as it was.
pub fn delete_in_formula(text: &str, sheet: &str, at: u64, count: u64) -> Option<String> {
    let expression = parse(text).ok()?;
    let deleted = expression.deleted(sheet, at, count);
    let printed = deleted.to_text();
    Some(if text.trim_start().starts_with('=') { format!("={printed}") } else { printed })
}

/// The formula's **text** with **columns inserted** at `at`.
///
/// The text is parsed, the columns are inserted and the expression is
/// printed. A formula that does not parse is `None`: a formula that
/// cannot be read is not changed.
pub fn insert_column_in_formula(text: &str, sheet: &str, at: u64, count: u64) -> Option<String> {
    let expression = parse(text).ok()?;
    let inserted = expression.inserted_column(sheet, at, count);
    let printed = inserted.to_text();
    if text.trim_start().starts_with('=') { Some(format!("={printed}")) } else { Some(printed) }
}

/// The formula's **text** with the columns **deleted** at `at`.
///
/// The text is parsed, the columns are deleted and the expression is
/// printed. A reference that names a column that is gone becomes the
/// error `#REF!`. A formula that does not parse is `None`: a formula
/// that cannot be read is left as it was.
pub fn delete_column_in_formula(text: &str, sheet: &str, at: u64, count: u64) -> Option<String> {
    let expression = parse(text).ok()?;
    let deleted = expression.deleted_column(sheet, at, count);
    let printed = deleted.to_text();
    Some(if text.trim_start().starts_with('=') { format!("={printed}") } else { printed })
}

/// The precedence of an expression.
fn precedence(expression: &Expr) -> u8 {
    match expression {
        Expr::Binary(operator, _, _) => match operator.as_str() {
            "=" | "<>" | "<" | "<=" | ">" | ">=" => COMPARISON,
            "+" | "-" => ADDITIVE,
            "*" | "/" => MULTIPLICATIVE,
            _ => POWER,
        },
        Expr::Negate(_) => UNARY,
        _ => UNARY + 1,
    }
}

/// An expression, written out, with brackets where they are needed.
fn expression_text(expression: &Expr, min_precedence: u8) -> String {
    let own = precedence(expression);
    let text = match expression {
        Expr::Number(number) => number_text(*number),
        Expr::Text(text) => format!("\"{}\"", text.replace('"', "\"\"")),
        Expr::Bool(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Expr::Error(message) => message.clone(),
        Expr::Name(name) => name.clone(),
        Expr::Cell { sheet, anchor } => format!("{}{}", sheet_prefix(sheet), anchor.to_text()),
        Expr::Range { sheet, start, end } => {
            format!("{}{}:{}", sheet_prefix(sheet), start.to_text(), end.to_text())
        }
        Expr::Call(name, arguments) => {
            let arguments: Vec<String> =
                arguments.iter().map(|argument| expression_text(argument, 0)).collect();
            format!("{name}({})", arguments.join(", "))
        }
        Expr::Negate(inner) => format!("-{}", expression_text(inner, UNARY)),
        Expr::Binary(operator, left, right) => {
            // A power binds to the right and everything else to the left.
            let (left_minimum, right_minimum) =
                if operator == "^" { (POWER + 1, POWER) } else { (own, own + 1) };
            format!(
                "{} {} {}",
                expression_text(left, left_minimum),
                operator,
                expression_text(right, right_minimum)
            )
        }
    };
    if own < min_precedence { format!("({text})") } else { text }
}

/// A number, as a formula writes it.
fn number_text(number: f64) -> String {
    if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

/// The name of a sheet, and its `!` — quoted when the name needs it.
fn sheet_prefix(sheet: &Option<String>) -> String {
    let Some(name) = sheet else { return String::new() };
    let plain = !name.is_empty()
        && name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !name.chars().next().is_some_and(|character| character.is_ascii_digit());
    if plain { format!("{name}!") } else { format!("'{}'!", name.replace('\'', "''")) }
}

// ---------------------------------------------------------------------------
// The shift: a formula copied, and its references moved.
// ---------------------------------------------------------------------------

/// Move a formula's **relative** references by `(dr, dc)`.
///
/// The tree's `shift_relative`: the relative part of every reference
/// moves and the anchored part stays where it is, so `$A$1` is the same
/// cell in every cell a formula is copied to, and `A1` is the cell to
/// the right of the one it was written in.
///
/// A reference that would leave the sheet becomes the **error**
/// `#REF!`, which is what the tree writes there.
pub fn shift(expression: &Expr, dr: i64, dc: i64) -> Expr {
    match expression {
        Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Error(_) => expression.clone(),
        // A **name** does not move: it stands for a range and not for
        // a place, so there is nothing relative in it.
        Expr::Name(_) => expression.clone(),
        Expr::Cell { sheet, anchor } => match anchor.moved(dr, dc) {
            Some(anchor) => Expr::Cell { sheet: sheet.clone(), anchor },
            None => Expr::Error(REF_ERROR.to_string()),
        },
        Expr::Range { sheet, start, end } => match (start.moved(dr, dc), end.moved(dr, dc)) {
            (Some(start), Some(end)) => Expr::Range { sheet: sheet.clone(), start, end },
            _ => Expr::Error(REF_ERROR.to_string()),
        },
        Expr::Call(name, arguments) => {
            let mut moved = Vec::new();
            for argument in arguments {
                moved.push(shift(argument, dr, dc));
            }
            Expr::Call(name.clone(), moved)
        }
        Expr::Negate(inner) => Expr::Negate(Box::new(shift(inner, dr, dc))),
        Expr::Binary(operator, left, right) => {
            Expr::Binary(operator.clone(), Box::new(shift(left, dr, dc)), Box::new(shift(right, dr, dc)))
        }
    }
}
