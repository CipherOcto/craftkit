//! The **finding** of a value in a range.
//!
//! Five functions look for something in a range of cells:
//!
//! - `MATCH(needle, range)` — the **place** of the needle in the
//!   range, counted from one;
//! - `VLOOKUP(needle, table, column)` — the value of the **column**
//!   of the row whose first column holds the needle;
//! - `HLOOKUP(needle, table, row)` — the value of the **row** of the
//!   column whose first row holds the needle;
//! - `LOOKUP(needle, vector)` — the value of the **vector** at the
//!   place of the needle;
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
//! # The approximate search
//!
//! A lookup that does not find the needle exactly may look for the
//! **largest value that is not greater than the needle** — or, the
//! other way, for the **smallest value that is not less than it**.
//! That is a rule that can be written down in a sentence, and it is
//! the rule of this tree:
//!
//! - the **exact** match is the answer, and it is the first one;
//! - when there is none, the answer is the largest value that is not
//!   greater than the needle — or the smallest value that is not less
//!   than it, when the search is the other way;
//! - a value of another kind is not ordered against the needle, and so
//!   is not a candidate;
//! - and of the values that are the best, the **first** is the answer.
//!
//! The **needle** of a lookup is a value, and an empty one is the
//! number zero — as it is for `XLOOKUP`.
//!
//! The four functions that look for something have the search, and it
//! is what they do when they are not told to be exact:
//!
//! - `MATCH(needle, range, [match_type])` — the match type `0` is the
//!   exact search, `1` is the approximate one, and `-1` is the
//!   approximate one the other way; the default is `1`;
//! - `VLOOKUP(needle, table, column, [range_lookup])` — the range
//!   lookup `FALSE` is the exact search and `TRUE` is the approximate
//!   one; the default is `TRUE`;
//! - `HLOOKUP(needle, table, row, [range_lookup])` — the same;
//! - `LOOKUP(needle, vector, [result])` — the approximate search, and
//!   nothing else.
//!
//! # The tree's other two
//!
//! The tree has two more lookups, and they are a pair:
//!
//! - `XLOOKUP(needle, lookup_array, return_array, [if_not_found],
//!   [match_mode], [search_mode])` — the value of the **return array**
//!   at the place of the needle;
//! - `XMATCH(needle, lookup_array, [match_mode], [search_mode])` — the
//!   **place** of the needle, counted from one.
//!
//! The **match mode** is how a value may meet the needle:
//!
//! - `0` — the **same** value, without the wildcards (the default);
//! - `-1` — the **largest** value below the needle;
//! - `1` — the **smallest** value above the needle;
//! - `2` — the same text, with the **wildcards** in it.
//!
//! And the **search mode** is which way the search goes: `1` is from
//! the first to the last, and `-1` is from the last to the first.
//!
//! The **binary search** — the search mode `2` and the `-2` — is over
//! **every** kind of value, in the tree's order: the numbers, then the
//! text, then the booleans, then the errors, and the empty cells
//! last. The answer is the value that order puts next to the needle:
//! the exact one when there is one, and otherwise the largest value
//! that is not greater than the needle, or the smallest value that is
//! not less than it, as the match mode says. The wildcards are not
//! used in it: the search is the exact one.

use crate::formula::{Book, CellRef, Expr, Value};
pub const NAMES: &[&str] =
    &["MATCH", "VLOOKUP", "HLOOKUP", "LOOKUP", "INDEX", "ROWS", "COLUMNS", "XLOOKUP", "XMATCH", "ADDRESS"];

/// The **match mode**: how a value may meet the needle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchMode {
    /// The **same** value — the exact search, without the wildcards.
    Exact,
    /// The **largest** value below the needle.
    Smaller,
    /// The **smallest** value above the needle.
    Larger,
    /// The same text, with the **wildcards** in it.
    Wildcard,
}

impl MatchMode {
    /// The match mode of an **argument**.
    fn of(value: &Value) -> Result<MatchMode, String> {
        let Some(mode) = value.as_number() else {
            return Err("#VALUE!".to_string());
        };
        if !mode.is_finite() {
            return Err("#VALUE!".to_string());
        }
        match mode.trunc() as i64 {
            0 => Ok(MatchMode::Exact),
            -1 => Ok(MatchMode::Smaller),
            1 => Ok(MatchMode::Larger),
            2 => Ok(MatchMode::Wildcard),
            _ => Err("#VALUE!".to_string()),
        }
    }
}

/// The **search mode**: which way the search goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchMode {
    /// From the **first** to the last.
    FirstToLast,
    /// From the **last** to the first.
    LastToFirst,
    /// The **binary** search, from the first to the last.
    Binary,
    /// The **binary** search, from the last to the first.
    BinaryReversed,
}

impl SearchMode {
    /// The search mode of an **argument**.
    fn of(value: &Value) -> Result<SearchMode, String> {
        let Some(mode) = value.as_number() else {
            return Err("#VALUE!".to_string());
        };
        if !mode.is_finite() {
            return Err("#VALUE!".to_string());
        }
        match mode.trunc() as i64 {
            1 => Ok(SearchMode::FirstToLast),
            -1 => Ok(SearchMode::LastToFirst),
            2 => Ok(SearchMode::Binary),
            -2 => Ok(SearchMode::BinaryReversed),
            _ => Err("#VALUE!".to_string()),
        }
    }

    /// Whether the search goes from the last to the first.
    fn reversed(self) -> bool {
        matches!(self, SearchMode::LastToFirst | SearchMode::BinaryReversed)
    }

    /// Whether the search is the **binary** one.
    fn binary(self) -> bool {
        matches!(self, SearchMode::Binary | SearchMode::BinaryReversed)
    }
}

/// The **match mode** of a search, when the search is the **binary**
/// one.
///
/// The wildcards are not used in a binary search: the search is the
/// exact one.
fn mode_of(mode: MatchMode, search: SearchMode) -> MatchMode {
    if search.binary() && mode == MatchMode::Wildcard {
        return MatchMode::Exact;
    }
    mode
}

/// The **order** a search reads the values in: every kind for the
/// binary search, and the needle's own kind for the other.
fn order_for(search: SearchMode) -> Order {
    if search.binary() { Order::EveryKind } else { Order::SameKind }
}

/// The **order** of the values of a search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    /// The order of the values of the **same kind** as the needle: the
    /// numbers with the numbers and the texts with the texts.
    SameKind,
    /// The **tree's order** over every kind: the numbers, then the
    /// text, then the booleans, then the errors, and the empty cells
    /// last.
    EveryKind,
}

/// Whether a **needle** and a **value** are the same, without the
/// wildcards.
///
/// The tree's rule for a lookup: two numbers are the same number, two
/// texts are the same text **without case**, and a value of one kind
/// is not a value of another kind.
fn same_value(needle: &Value, value: &Value) -> bool {
    match (needle, value) {
        (Value::Number(one), Value::Number(other)) => one == other,
        (Value::Bool(one), Value::Bool(other)) => one == other,
        (Value::Text(one), Value::Text(other)) => {
            crate::criteria::compare_text(one, other) == std::cmp::Ordering::Equal
        }
        (Value::Error(one), Value::Error(other)) => one == other,
        (Value::Empty, Value::Empty) => true,
        _ => false,
    }
}

/// The **order** of a value and the needle, for the searches that
/// look for the next smaller or the next larger.
///
/// The tree's rule: two numbers are ordered as numbers — and
/// **exactly** as numbers, and not as the fifteen digits of an
/// equality — two texts as texts without case, and a value of one
/// kind is not ordered against a needle of another kind.
///
/// The **binary** search is over every kind, and then the order is
/// the tree's own: the numbers, then the text, then the booleans,
/// then the errors, and the empty cells last.
fn order_of(value: &Value, needle: &Value, order: Order) -> Option<std::cmp::Ordering> {
    match order {
        Order::SameKind => match (value, needle) {
            (Value::Number(one), Value::Number(other)) => one.partial_cmp(other),
            (Value::Text(one), Value::Text(other)) => Some(crate::criteria::compare_text(one, other)),
            _ => None,
        },
        // The tree's order over every kind: the **kinds** decide
        // first, and then the values of the same kind are compared as
        // the tree compares them.
        Order::EveryKind => Some(crate::sorting::compare(value, needle)),
    }
}

/// The **place** of a needle in a vector of values.
///
/// The exact search is the first match — or the last, when the search
/// goes from the last to the first. The searches for the next smaller
/// and the next larger try the **exact** search first, and then take
/// the best candidate: the largest value below the needle, or the
/// smallest above it.
///
/// The **order** is the one the search reads the values in: the values
/// of the needle's own kind, or — for the binary search — every kind,
/// in the tree's order.
fn place_of(
    needle: &Value,
    values: &[Value],
    mode: MatchMode,
    reversed: bool,
    order: Order,
) -> Option<usize> {
    let equal = |value: &Value| match mode {
        MatchMode::Wildcard => match (needle, value) {
            (Value::Text(pattern), Value::Text(text)) => crate::formula::wildcard(pattern, text),
            _ => same_value(needle, value),
        },
        _ => same_value(needle, value),
    };
    let order_of_places: Vec<usize> =
        if reversed { (0..values.len()).rev().collect() } else { (0..values.len()).collect() };
    if let Some(place) = order_of_places.iter().copied().find(|place| equal(&values[*place])) {
        return Some(place);
    }
    if !matches!(mode, MatchMode::Smaller | MatchMode::Larger) {
        return None;
    }
    let mut best: Option<usize> = None;
    for place in order_of_places {
        let Some(relation) = order_of(&values[place], needle, order) else { continue };
        let candidate = match mode {
            MatchMode::Smaller => relation == std::cmp::Ordering::Less,
            MatchMode::Larger => relation == std::cmp::Ordering::Greater,
            _ => false,
        };
        if !candidate {
            continue;
        }
        let better = match best.map(|best| order_of(&values[best], needle, order)) {
            None => true,
            Some(None) => true,
            Some(Some(previous)) => match mode {
                // The best candidate is the one **closest** to the
                // needle: the largest below it, and the smallest
                // above it.
                MatchMode::Smaller => previous == std::cmp::Ordering::Less,
                MatchMode::Larger => previous == std::cmp::Ordering::Greater,
                _ => false,
            },
        };
        if better {
            best = Some(place);
        }
    }
    best
}

/// The **needle** of a lookup: a value, and the empty one is the
/// number zero.
fn lookup_needle(value: &Value) -> Value {
    match value {
        Value::Empty => Value::Number(0.0),
        other => other.clone(),
    }
}

/// The **match type** of a search: how a value may meet the needle.
///
/// This is the tree's rule, and it is one sentence: the **exact**
/// match is the answer, and when there is none the answer is the
/// largest value that is not greater than the needle — or the
/// smallest value that is not less than it, when the search is the
/// other way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchType {
    /// The **exact** search, and nothing else.
    Exact,
    /// The exact search first, and then the **largest** value that is
    /// not greater than the needle.
    Largest,
    /// The exact search first, and then the **smallest** value that is
    /// not less than the needle.
    Smallest,
}

/// The **place** of a needle in a vector of values, when the search is
/// the tree's.
///
/// The exact match is the answer, and it is the **first** one. When
/// there is none, the answer is the best candidate: the largest value
/// that is not greater than the needle, or the smallest value that is
/// not less than it. A value of another kind is not ordered against
/// the needle, and so is not a candidate — and of the values that are
/// the best, the **first** is the answer.
fn place_by_type(needle: &Value, values: &[Value], kind: MatchType) -> Option<usize> {
    if let Some(place) = values.iter().position(|value| lookup_equal(needle, value)) {
        return Some(place);
    }
    if kind == MatchType::Exact {
        return None;
    }
    let mut best: Option<usize> = None;
    for place in 0..values.len() {
        let Some(order) = order_of(&values[place], needle, Order::SameKind) else { continue };
        let candidate = match kind {
            MatchType::Largest => order != std::cmp::Ordering::Greater,
            MatchType::Smallest => order != std::cmp::Ordering::Less,
            MatchType::Exact => false,
        };
        if !candidate {
            continue;
        }
        let better = match best {
            None => true,
            Some(previous) => match order_of(&values[previous], needle, Order::SameKind) {
                // The best candidate is the one **closest** to the
                // needle: the largest below it, and the smallest
                // above it.
                Some(previous_order) => match kind {
                    MatchType::Largest => previous_order == std::cmp::Ordering::Less,
                    MatchType::Smallest => previous_order == std::cmp::Ordering::Greater,
                    MatchType::Exact => false,
                },
                None => true,
            },
        };
        if better {
            best = Some(place);
        }
    }
    best
}

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
            "HLOOKUP" => self.hlookup_call(sheet, arguments, being),
            "LOOKUP" => self.lookup_call(sheet, arguments, being),
            "INDEX" => self.index_call(sheet, arguments, being),
            "ROWS" | "COLUMNS" => self.shape_call(sheet, name, arguments, being),
            "XLOOKUP" => self.xlookup_call(sheet, arguments, being),
            "XMATCH" => self.xmatch_call(sheet, arguments, being),
            "ADDRESS" => self.address_call(sheet, arguments, being),
            _ => Value::Error(format!("there is no function `{name}`")),
        }
    }

    /// The **shape** of a range: how many rows it has, or how many
    /// columns.
    ///
    /// A range is a range and has a shape; an argument that is not a
    /// range is one cell and has a shape of one row and one column —
    /// the tree's rule, and the rule that makes `ROWS(5)` one.
    fn shape_call(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        _being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let Some(argument) = arguments.first() else {
            return Value::Error("#VALUE!".to_string());
        };
        let size = match self.area_of(sheet, argument) {
            Ok((_, area)) => {
                let rows = area.end.row - area.start.row + 1;
                let columns = area.end.col - area.start.col + 1;
                if name == "ROWS" { rows } else { columns }
            }
            // An argument that is not a range is one cell.
            Err(_) => 1,
        };
        Value::Number(f64::from(size))
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
    ///
    /// The **match type** says how a value may meet the needle: `0`
    /// is the exact search, `1` is the approximate one — the largest
    /// value that is not greater than the needle — and `-1` is the
    /// approximate one the other way, the smallest value that is not
    /// less than the needle. The default is `1`, and a match type that
    /// is none of the three is a value error.
    fn match_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        let needle = lookup_needle(&needle);
        // The **match type**: the exact search, the approximate one, or
        // the approximate one the other way.
        let kind = match arguments.get(2) {
            None => MatchType::Largest,
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                let Some(number) = value.as_number() else {
                    return Value::Error("#VALUE!".to_string());
                };
                if !number.is_finite() {
                    return Value::Error("#VALUE!".to_string());
                }
                match number.trunc() as i64 {
                    0 => MatchType::Exact,
                    1 => MatchType::Largest,
                    -1 => MatchType::Smallest,
                    _ => return Value::Error("#VALUE!".to_string()),
                }
            }
        };
        let (_name, _area, values) = match self.values_of(sheet, &arguments[1], being) {
            Ok(values) => values,
            Err(message) => return Value::Error(message),
        };
        match place_by_type(&needle, &values, kind) {
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
        let needle = lookup_needle(&needle);
        // The **column** that is wanted: counted from one.
        let column = self.evaluate(sheet, &arguments[2], being);
        let Some(column) = column.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if column.is_nan() || column < 1.0 {
            return Value::Error("#VALUE!".to_string());
        }
        let column = column.trunc() as usize - 1;
        // The **range lookup**: the exact search, or the approximate
        // one. The default is the approximate one.
        let kind = match arguments.get(3) {
            None => MatchType::Largest,
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                match crate::formula::truth_of(&value) {
                    Ok(true) => MatchType::Largest,
                    Ok(false) => MatchType::Exact,
                    Err(message) => return Value::Error(message),
                }
            }
        };
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
        let keys: Vec<Value> = (0..rows).map(|row| values[row * columns].clone()).collect();
        let Some(place) = place_by_type(&needle, &keys, kind) else {
            return Value::Error("#N/A".to_string());
        };
        let cell = CellRef { col: area.start.col + column as u32, row: area.start.row + place as u32 };
        self.value_of_guarded(&name, cell, being)
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

    /// The tree's `HLOOKUP`: the value **across** a table.
    ///
    /// The **keys** are the first **row** of the table, and the index
    /// is a **row** of it — the tree's `HLOOKUP` is the `VLOOKUP` of
    /// the table the other way round.
    ///
    /// The search is the tree's, and it is the **approximate** one
    /// unless the lookup says it is exact — as it is for `VLOOKUP`.
    fn hlookup_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 3 || arguments.len() > 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        let needle = lookup_needle(&needle);
        // The **row** that is wanted: counted from one.
        let row = self.evaluate(sheet, &arguments[2], being);
        let Some(row) = row.as_number() else {
            return Value::Error("#VALUE!".to_string());
        };
        if row.is_nan() || row < 1.0 {
            return Value::Error("#VALUE!".to_string());
        }
        let row = row.trunc() as usize - 1;
        // The **range lookup**: the exact search, or the approximate
        // one. The default is the approximate one.
        let kind = match arguments.get(3) {
            None => MatchType::Largest,
            Some(argument) => {
                let value = self.evaluate(sheet, argument, being);
                match crate::formula::truth_of(&value) {
                    Ok(true) => MatchType::Largest,
                    Ok(false) => MatchType::Exact,
                    Err(message) => return Value::Error(message),
                }
            }
        };
        let (name, area, values) = match self.values_of(sheet, &arguments[1], being) {
            Ok(values) => values,
            Err(message) => return Value::Error(message),
        };
        let columns = (area.end.col - area.start.col + 1) as usize;
        let rows = (area.end.row - area.start.row + 1) as usize;
        if row >= rows {
            return Value::Error("#REF!".to_string());
        }
        // The **keys** are the first row of the table.
        let keys: Vec<Value> = values.iter().take(columns).cloned().collect();
        let Some(place) = place_by_type(&needle, &keys, kind) else {
            return Value::Error("#N/A".to_string());
        };
        let cell = CellRef { col: area.start.col + place as u32, row: area.start.row + row as u32 };
        self.value_of_guarded(&name, cell, being)
    }

    /// The tree's `LOOKUP`: the value of a **vector** at the place of
    /// the needle.
    ///
    /// The **keys** are the vector, and the **answer** is the value at
    /// that place in it — or in the vector that is given as the third
    /// argument, at the same place. A **table** of results is answered
    /// by its first column, and it must have a row for every key.
    ///
    /// The search is the **approximate** one, and nothing else: the
    /// exact match is the answer when there is one, and the largest
    /// value that is not greater than the needle when there is not.
    ///
    /// A **table** may be given instead of a vector: the keys are its
    /// first row when it is wider than it is tall, and its first
    /// column when it is taller than it is wide; and the answer is
    /// the entry of its last row, or of its last column, at that
    /// place.
    fn lookup_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        let needle = lookup_needle(&needle);
        let (name, area, values) = match self.values_of(sheet, &arguments[1], being) {
            Ok(values) => values,
            Err(message) => return Value::Error(message),
        };
        let rows = (area.end.row - area.start.row + 1) as usize;
        let columns = (area.end.col - area.start.col + 1) as usize;
        // The **wide** table: its keys are its first row, and its
        // answer is in its last row.
        let wide = columns > rows;
        // The **keys**: a vector, or the first row of a wide table, or
        // the first column of a tall one.
        let keys: Vec<Value> = if rows == 1 || columns == 1 {
            values.clone()
        } else if wide {
            values.iter().take(columns).cloned().collect()
        } else {
            (0..rows).map(|row| values[row * columns].clone()).collect()
        };
        let Some(place) = place_by_type(&needle, &keys, MatchType::Largest) else {
            return Value::Error("#N/A".to_string());
        };
        // The **result**: the vector that is given, or the table
        // itself.
        if let Some(argument) = arguments.get(2) {
            let (result_name, result_area, result_values) = match self.values_of(sheet, argument, being) {
                Ok(values) => values,
                Err(message) => return Value::Error(message),
            };
            let result_rows = (result_area.end.row - result_area.start.row + 1) as usize;
            let result_columns = (result_area.end.col - result_area.start.col + 1) as usize;
            if result_rows == 1 || result_columns == 1 {
                let Some(value) = result_values.get(place) else {
                    return Value::Error("#N/A".to_string());
                };
                return value.clone();
            }
            // A table of results: the first column of it, at that
            // place — and it must have a row for every key.
            if result_rows < keys.len() {
                return Value::Error("#N/A".to_string());
            }
            let cell = CellRef { col: result_area.start.col, row: result_area.start.row + place as u32 };
            return self.value_of_guarded(&result_name, cell, being);
        }
        // The **answer** in the table itself.
        if rows == 1 || columns == 1 {
            let Some(value) = values.get(place) else {
                return Value::Error("#N/A".to_string());
            };
            return value.clone();
        }
        if wide {
            let cell = CellRef { col: area.start.col + place as u32, row: area.end.row };
            return self.value_of_guarded(&name, cell, being);
        }
        let cell = CellRef { col: area.end.col, row: area.start.row + place as u32 };
        self.value_of_guarded(&name, cell, being)
    }

    /// The tree's `XLOOKUP`: the value of the **return array** at the
    /// place of the needle.
    ///
    /// The lookup array is a **vector** — a row or a column — and the
    /// return array is of the same shape: a column lookup returns the
    /// **rows** of the return array, and a row lookup returns its
    /// **columns**. The tree's `XLOOKUP` may return a whole row or a
    /// whole column, and this tree has no arrays, so a return array
    /// that is wider than one cell is refused.
    ///
    /// The needle that is **not there** is the value that is given in
    /// the fourth argument, and `#N/A` when there is none.
    fn xlookup_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 3 || arguments.len() > 6 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        let needle = lookup_needle(&needle);
        let (lookup_sheet, lookup) = match self.area_of(sheet, &arguments[1]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let (return_sheet, returns) = match self.area_of(sheet, &arguments[2]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let lookup_rows = (lookup.end.row - lookup.start.row + 1) as usize;
        let lookup_columns = (lookup.end.col - lookup.start.col + 1) as usize;
        // The lookup array is a **vector**: a row or a column.
        if lookup_rows > 1 && lookup_columns > 1 {
            return Value::Error("#VALUE!".to_string());
        }
        let return_rows = (returns.end.row - returns.start.row + 1) as usize;
        let return_columns = (returns.end.col - returns.start.col + 1) as usize;
        // The **orientation**: a column lookup returns the rows of the
        // return array, and a row lookup returns its columns. A lookup
        // array of one cell takes the orientation of the return array.
        let vertical = if lookup_rows == 1 && lookup_columns == 1 {
            return_rows == 1 || return_columns != 1
        } else {
            lookup_columns == 1
        };
        if vertical && return_rows != lookup_rows {
            return Value::Error("#VALUE!".to_string());
        }
        if !vertical && return_columns != lookup_columns {
            return Value::Error("#VALUE!".to_string());
        }
        let mode = match self.match_mode(sheet, arguments.get(4), being) {
            Ok(mode) => mode,
            Err(message) => return Value::Error(message),
        };
        let search = match self.search_mode(sheet, arguments.get(5), being) {
            Ok(search) => search,
            Err(message) => return Value::Error(message),
        };
        let values = self.vector_of(&lookup_sheet, &lookup, vertical, being);
        let Some(place) =
            place_of(&needle, &values, mode_of(mode, search), search.reversed(), order_for(search))
        else {
            // The needle that is **not there**.
            return match arguments.get(3) {
                Some(argument) => self.evaluate(sheet, argument, being),
                None => Value::Error("#N/A".to_string()),
            };
        };
        // The **result**: the cell of the return array.
        if vertical {
            if return_columns != 1 {
                return Value::Error("not supported yet: an array".to_string());
            }
            let cell = CellRef { col: returns.start.col, row: returns.start.row + place as u32 };
            return self.value_of_guarded(&return_sheet, cell, being);
        }
        if return_rows != 1 {
            return Value::Error("not supported yet: an array".to_string());
        }
        let cell = CellRef { col: returns.start.col + place as u32, row: returns.start.row };
        self.value_of_guarded(&return_sheet, cell, being)
    }

    /// The tree's `XMATCH`: the **place** of the needle in a vector of
    /// values, counted from one.
    fn xmatch_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 4 {
            return Value::Error("#VALUE!".to_string());
        }
        let needle = self.evaluate(sheet, &arguments[0], being);
        if let Value::Error(message) = &needle {
            return Value::Error(message.clone());
        }
        let needle = lookup_needle(&needle);
        let (lookup_sheet, lookup) = match self.area_of(sheet, &arguments[1]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let rows = (lookup.end.row - lookup.start.row + 1) as usize;
        let columns = (lookup.end.col - lookup.start.col + 1) as usize;
        if rows > 1 && columns > 1 {
            return Value::Error("#VALUE!".to_string());
        }
        let mode = match self.match_mode(sheet, arguments.get(2), being) {
            Ok(mode) => mode,
            Err(message) => return Value::Error(message),
        };
        let search = match self.search_mode(sheet, arguments.get(3), being) {
            Ok(search) => search,
            Err(message) => return Value::Error(message),
        };
        let values = self.vector_of(&lookup_sheet, &lookup, columns == 1, being);
        match place_of(&needle, &values, mode_of(mode, search), search.reversed(), order_for(search)) {
            Some(place) => Value::Number((place + 1) as f64),
            None => Value::Error("#N/A".to_string()),
        }
    }

    /// The tree's `ADDRESS`: the **address** of a cell, as text.
    ///
    /// The tree's rule:
    ///
    /// - the **row** is counted from one and the **column** from one,
    ///   and a row or a column that is not there is a value error;
    /// - the **absolute** argument says which part of the address is
    ///   anchored: one for both, two for the row, three for the column
    ///   and four for neither;
    /// - the **style** argument says whether the address is written
    ///   as `A1` or as `R1C1`, and `A1` is the default;
    /// - the **sheet** argument puts the address in a sheet, and a
    ///   sheet whose name needs quotes is quoted.
    fn address_call(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 5 {
            return Value::Error("#VALUE!".to_string());
        }
        let row = match self.number_at(sheet, arguments, 0, being) {
            Ok(number) => number,
            Err(message) => return Value::Error(message),
        };
        let column = match self.number_at(sheet, arguments, 1, being) {
            Ok(number) => number,
            Err(message) => return Value::Error(message),
        };
        let absolute = match self.optional_number(sheet, arguments, 2, being) {
            Ok(Some(number)) => number.trunc() as i64,
            Ok(None) => 1,
            Err(message) => return Value::Error(message),
        };
        let style = match self.optional_number(sheet, arguments, 3, being) {
            Ok(Some(number)) => number != 0.0,
            Ok(None) => true,
            Err(message) => return Value::Error(message),
        };
        let row = row.trunc();
        let column = column.trunc();
        if !(1.0..=crate::formula::MAX_FORMULA_ROWS as f64).contains(&row)
            || !(1.0..=crate::formula::MAX_FORMULA_COLUMNS as f64).contains(&column)
            || !(1..=4).contains(&absolute)
        {
            return Value::Error("#VALUE!".to_string());
        }
        let row = row as u64;
        let column = column as u64;
        let (row_anchored, column_anchored) = match absolute {
            1 => (true, true),
            2 => (true, false),
            3 => (false, true),
            _ => (false, false),
        };
        let cell = if style {
            format!(
                "{}{}{}{}",
                if column_anchored { "$" } else { "" },
                CellRef::column_text((column - 1) as u32),
                if row_anchored { "$" } else { "" },
                row
            )
        } else {
            // The `R1C1` style: the row and the column, and the parts
            // of them that are not anchored are in brackets.
            format!(
                "R{row_part}C{column_part}",
                row_part = if row_anchored { row.to_string() } else { format!("[{row}]") },
                column_part = if column_anchored { column.to_string() } else { format!("[{column}]") }
            )
        };
        // The **sheet**: the name of one, quoted when it needs it.
        let Some(argument) = arguments.get(4) else {
            return Value::Text(cell);
        };
        let name = match self.text_at(sheet, arguments, 4, being) {
            Ok(text) => text,
            Err(message) => return Value::Error(message),
        };
        let _ = argument;
        if name.is_empty() {
            return Value::Text(format!("!{cell}"));
        }
        Value::Text(format!("{}!{cell}", crate::formula::quoted_sheet(&name)))
    }

    /// The **match mode** of a search: how a value may meet the
    /// needle.
    fn match_mode(
        &self,
        sheet: &str,
        argument: Option<&Expr>,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<MatchMode, String> {
        let Some(argument) = argument else { return Ok(MatchMode::Exact) };
        let value = self.evaluate(sheet, argument, being);
        MatchMode::of(&value)
    }

    /// The **search mode** of a search: which way the search goes.
    fn search_mode(
        &self,
        sheet: &str,
        argument: Option<&Expr>,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<SearchMode, String> {
        let Some(argument) = argument else { return Ok(SearchMode::FirstToLast) };
        let value = self.evaluate(sheet, argument, being);
        SearchMode::of(&value)
    }

    /// The **values** of a range, in the order a vector is read: down
    /// a column, or across a row.
    fn vector_of(
        &self,
        sheet: &str,
        area: &crate::formula::Area,
        vertical: bool,
        being: &mut Vec<(String, CellRef)>,
    ) -> Vec<Value> {
        let rows = (area.end.row - area.start.row + 1) as usize;
        let columns = (area.end.col - area.start.col + 1) as usize;
        let mut values = Vec::new();
        if vertical {
            for row in 0..rows {
                let cell = CellRef { col: area.start.col, row: area.start.row + row as u32 };
                values.push(self.value_of_guarded(sheet, cell, being));
            }
        } else {
            for column in 0..columns {
                let cell = CellRef { col: area.start.col + column as u32, row: area.start.row };
                values.push(self.value_of_guarded(sheet, cell, being));
            }
        }
        values
    }
}
