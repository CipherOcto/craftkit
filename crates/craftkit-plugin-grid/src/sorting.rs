//! The **sorting** of a range.
//!
//! A range of cells may be **sorted**: the rows of the range are put
//! in order by the values of one of its columns — or of several, for
//! the rows the first one cannot tell apart.
//!
//! # What moves with a row
//!
//! A row is not only its values: the **format** and the **style** of
//! a cell move with it, and so does its **formula** — with the
//! formula's **relative** references moved with it, and its anchored
//! references left where they are. And the **note** and the **link**
//! of a cell travel with the row they describe, as they do in the
//! tree.
//!
//! # The orientation
//!
//! A sort may put the **rows** of the range in order — which is what
//! it does — or its **columns**. The sort is then by a **row** of the
//! range, and the columns are the lines that move: a column takes
//! its **format**, its **formula** and its **note** with it, as a row
//! does.
//!
//! The **header** follows the orientation: in a sort of the rows it is
//! the first **row** that stays where it is, and in a sort of the
//! columns it is the first **column**.
//!
//! # The order
//!
//! The tree's order: numbers, then text, then booleans, then errors,
//! and the **empty** cells last — whatever the order is. The text is
//! compared without case, and a search that says the case matters
//! puts the capitals after the small letters.
//!
//! And the **kinds** are in that order too: a number comes before a
//! text, a text before a boolean, and a boolean before an error. The
//! **binary search** of a lookup reads the same order (§1008).
//!
//! The tree **guesses** whether the first row of the range is a
//! **header**: a column whose first cell is text and whose second
//! cell is not, votes for it. So does this one, and the guess may be
//! said otherwise.

use std::cmp::Ordering;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Text};

use crate::formula::{Area, CellRef, Cells, Value};

/// A **column**: the letters of it, as `B`.
fn column_at(text: &str, id: &str) -> Result<u32, EngineError> {
    let text = text.trim().trim_start_matches('$');
    if text.is_empty() || !text.chars().all(|character| character.is_ascii_alphabetic()) {
        return Err(EngineError::Failed { id: id.to_string(), message: format!("`{text}` is not a column") });
    }
    // The letters are a base of twenty-six: `A` is 0, `Z` is 25 and
    // `AA` is 26.
    let mut column = 0u32;
    for character in text.chars() {
        let letter = character.to_ascii_uppercase() as u32 - 'A' as u32;
        column =
            column.checked_mul(26).and_then(|column| column.checked_add(letter + 1)).ok_or_else(|| {
                EngineError::Failed { id: id.to_string(), message: format!("`{text}` is not a column") }
            })?;
    }
    column.checked_sub(1).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("`{text}` is not a column"),
    })
}

/// The **text** a value shows: the tree's own text of a cell.
fn text_of(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => number.to_string(),
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => if *flag { "TRUE" } else { "FALSE" }.to_string(),
        Value::Error(message) => message.clone(),
    }
}

/// The **order** of two values by a **list**: the words of the list in
/// the order of the list, and the words that are not in it after them,
/// in the tree's own order.
fn compare_by_list(left: &Value, right: &Value, list: &[String]) -> Ordering {
    let place = |value: &Value| {
        let text = text_of(value);
        list.iter().position(|word| *word == text)
    };
    match (place(left), place(right)) {
        (Some(one), Some(other)) => one.cmp(&other),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => compare(left, right),
    }
}

/// The **place of a kind** in the tree's order: the numbers, then the
/// text, then the booleans, then the errors, and the empty cells
/// last.
///
/// This is the order of the **kinds**, and it is the first thing the
/// tree's order says: two values of different kinds are ordered by
/// their kinds, and two of the same kind are compared as they are.
pub(crate) fn rank(value: &Value) -> u8 {
    match value {
        Value::Number(_) => 0,
        Value::Text(_) => 1,
        Value::Bool(_) => 2,
        Value::Error(_) => 3,
        Value::Empty => 4,
    }
}

/// The **order** of two values, the tree's way.
///
/// The **kinds** decide first: the numbers, then the text, then the
/// booleans, then the errors, and the empty cells last. And two values
/// of the **same kind** are compared as they are: two numbers as
/// numbers, two texts as texts without case — and the capitals after
/// the small letters — two booleans as booleans, and two errors by
/// their message.
///
/// This is the tree's one order: the sort of a range uses it, and so
/// does the **binary search** of a lookup (§1008).
pub(crate) fn compare(left: &Value, right: &Value) -> Ordering {
    match rank(left).cmp(&rank(right)) {
        Ordering::Equal => compare_same(left, right),
        other => other,
    }
}

/// The **order** of two values of the **same kind**, the tree's way.
fn compare_same(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Empty, Value::Empty) => Ordering::Equal,
        (Value::Number(one), Value::Number(other)) => one.partial_cmp(other).unwrap_or(Ordering::Equal),
        (Value::Text(one), Value::Text(other)) => {
            crate::criteria::compare_text(one, other).then_with(|| case_order(one, other))
        }
        (Value::Bool(one), Value::Bool(other)) => one.cmp(other),
        (Value::Error(one), Value::Error(other)) => one.as_str().cmp(other.as_str()),
        _ => Ordering::Equal,
    }
}

/// The **case** of two texts: the capitals come after the small
/// letters.
fn case_order(one: &str, other: &str) -> Ordering {
    one.chars()
        .zip(other.chars())
        .find(|(left, right)| left != right)
        .map_or(Ordering::Equal, |(left, right)| right.is_lowercase().cmp(&left.is_lowercase()))
}

/// Which way a sort runs: the **rows** of the range, or its
/// **columns**.
///
/// The orientation decides what a **line** is — a row or a column —
/// and what a **key** names: a column of the range in a sort of the
/// rows, and a row of it in a sort of the columns.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Orientation {
    /// The rows of the range are put in order.
    Rows,
    /// The columns of the range are put in order.
    Columns,
}

impl Orientation {
    /// The word that says the orientation.
    fn to_text(self) -> &'static str {
        match self {
            Orientation::Rows => "rows",
            Orientation::Columns => "columns",
        }
    }
}

/// Whether the first line of a range is a **header**.
///
/// The tree's guess: a column whose first cell is text and whose
/// second cell is not text and is not empty votes for it, and the
/// votes must be more than none.
///
/// The guess follows the **orientation**: in a sort of the rows the
/// first **row** is looked at, and in a sort of the columns the first
/// **column** is — the guess of one is the guess of the other turned
/// on its side.
fn guess_header(cells: &Cells, area: &Area, orientation: Orientation) -> bool {
    match orientation {
        Orientation::Rows => {
            if area.start.row == area.end.row {
                return false;
            }
            let mut votes = 0i32;
            for col in area.start.col..=area.end.col {
                let first = CellRef { col, row: area.start.row };
                let second = CellRef { col, row: area.start.row + 1 };
                let first = cells.get(first).and_then(|entry| entry.value.clone());
                let second = cells.get(second).and_then(|entry| entry.value.clone());
                if let (Some(Value::Text(_)), Some(second)) = (&first, &second)
                    && !matches!(second, Value::Text(_))
                    && !matches!(second, Value::Empty)
                {
                    votes += 1;
                }
            }
            votes > 0
        }
        Orientation::Columns => {
            if area.start.col == area.end.col {
                return false;
            }
            let mut votes = 0i32;
            for row in area.start.row..=area.end.row {
                let first = CellRef { col: area.start.col, row };
                let second = CellRef { col: area.start.col + 1, row };
                let first = cells.get(first).and_then(|entry| entry.value.clone());
                let second = cells.get(second).and_then(|entry| entry.value.clone());
                if let (Some(Value::Text(_)), Some(second)) = (&first, &second)
                    && !matches!(second, Value::Text(_))
                    && !matches!(second, Value::Empty)
                {
                    votes += 1;
                }
            }
            votes > 0
        }
    }
}

/// A **key** of a sort: the line it sorts by, whether the order is
/// **descending**, and the **list** it orders the words by, when it
/// orders them by a list.
struct Key {
    /// The line the key sorts by: a **column** in a sort of the rows,
    /// and a **row** in a sort of the columns.
    line: u32,
    /// Whether the order is descending.
    descending: bool,
    /// The place of the list the words are ordered by, when they are.
    list: Option<u64>,
}

/// The **row** a number says, if it is one: counted from **one**, as
/// the tree counts its rows.
fn row_at(value: &Json, id: &str) -> Result<u32, EngineError> {
    let Some(row) = value.as_u64() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a row is counted from one".to_string(),
        });
    };
    if row < 1 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a row is counted from one: {row} is not one"),
        });
    }
    u32::try_from(row - 1).map_err(|_| EngineError::Failed {
        id: id.to_string(),
        message: format!("{row} is not a row of a sheet"),
    })
}

/// The **keys** of a sort, from the caller's list.
///
/// A key names the line it sorts by — a **column** in a sort of the
/// rows, a **row** in a sort of the columns — and says whether the
/// order is **descending**. A list with no key in it is refused: a
/// sort with no key is not a sort.
///
/// A key that names the wrong kind of line is refused too: the tree
/// does not guess which way a sort was meant to run.
fn keys_of(value: &Json, id: &str, orientation: Orientation) -> Result<Vec<Key>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "the keys of a sort are a list".to_string(),
        });
    };
    let mut keys = Vec::new();
    for key in list {
        let line = match orientation {
            Orientation::Rows => {
                if key.get("row").is_some() {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: "a sort of the rows is by a column, and not by a row".to_string(),
                    });
                }
                let Some(column) = key.get("column").and_then(|column| column.as_str()) else {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: "a key names the column it sorts by".to_string(),
                    });
                };
                column_at(column, id)?
            }
            Orientation::Columns => {
                if key.get("column").is_some() {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: "a sort of the columns is by a row, and not by a column".to_string(),
                    });
                }
                let Some(row) = key.get("row") else {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: "a key names the row it sorts by".to_string(),
                    });
                };
                row_at(row, id)?
            }
        };
        let descending = key.get("descending").and_then(|descending| descending.as_bool()).unwrap_or(false);
        let list = key.get("list").and_then(|list| list.as_u64());
        keys.push(Key { line, descending, list });
    }
    if keys.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a sort has at least one key".to_string(),
        });
    }
    Ok(keys)
}

commands! {
    /// **Sort** a range of cells by one of its lines.
    ///
    /// The **orientation** says which way the range is sorted: its
    /// **rows** are put in order by a **column** of the range, or its
    /// **columns** are put in order by a **row** of it.
    ///
    /// The **keys** of the sort may be more than one: the first key is
    /// the one that decides, and the next one is looked at only when the
    /// first cannot tell two lines apart. A key names the **column** it
    /// sorts by in a sort of the rows, and the **row** it sorts by in a
    /// sort of the columns.
    grid.sort {
        area: "grid",
        layer: "grid",
        icon: "arrow",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range to sort. Without it, the used range is sorted.
            range: Option<Text>,
            /// Which way the range is sorted: the **rows** of it, or its
            /// **columns**. Without it, the rows are sorted.
            orientation: Option<Text>,
            /// The column to sort by, as `B`. Without it, the first column of
            /// the range. It is a sort of the rows only: a sort of the columns
            /// is by a row.
            column: Option<Text>,
            /// Whether the order is **descending**.
            descending: Option<Bool>,
            /// Whether the first row is a **header** and stays where it is. In
            /// a sort of the columns it is the first **column** that stays.
            header: Option<Bool>,
            /// The **keys** of the sort: a list of them, and the first is
            /// the one that decides. Each key names the **column** it
            /// sorts by — or the **row**, in a sort of the columns — says
            /// whether the order is **descending**, and may name the **list**
            /// the words are ordered by — the place of the list in the
            /// workbook.
            ///
            /// The keys and the column may not be given together: a sort
            /// is sorted by its keys or by its column, and not by both.
            keys: Option<Json>,
        },
        run: {
            let id = "grid.sort".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            crate::protection::refuse_sort(node, &id)?;
            let name = node.name.clone();
            let mut cells = crate::cells_of(node);
            // The **range**: the one that was named, or the used range.
            let area = match &self.range {
                Some(range) => crate::fill::area_at(&range.0, &id)?,
                None => match crate::reading::used_range(&cells) {
                    Some(area) => area,
                    None => return Ok(Outcome::new(format!("{name} holds nothing to sort"))),
                },
            };
            // The **orientation**: the rows of the range, or its columns.
            let orientation = match self.orientation.as_ref().map(|text| text.0.trim().to_ascii_lowercase()) {
                Some(text) if text == "rows" => Orientation::Rows,
                Some(text) if text == "columns" => Orientation::Columns,
                Some(text) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a sort is of the rows or of the columns, and not of `{text}`"),
                    });
                }
                None => Orientation::Rows,
            };
            // A sort of the columns is by a row: the column is refused.
            if orientation == Orientation::Columns && self.column.is_some() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a sort of the columns is by a row, and not by a column".to_string(),
                });
            }
            // The **keys**: the ones that were named, or the one column
            // that was named, or the first column of the range — and the
            // first **row** of it, in a sort of the columns.
            let keys = match &self.keys {
                Some(keys) => {
                    if self.column.is_some() || self.descending.is_some() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "a sort is sorted by its keys or by its column, and not by both".to_string(),
                        });
                    }
                    keys_of(keys, &id, orientation)?
                }
                None => {
                    let line = match orientation {
                        Orientation::Rows => match &self.column {
                            Some(column) => column_at(&column.0, &id)?,
                            None => area.start.col,
                        },
                        Orientation::Columns => area.start.row,
                    };
                    vec![Key {
                        line,
                        descending: self.descending.map(|flag| flag.0).unwrap_or(false),
                        list: None,
                    }]
                }
            };
            // The **lists** of the workbook: a key may order the words
            // by one of them, and a key that names a list that is not
            // there is refused.
            let lists = crate::custom_lists_of(session, sheet);
            for key in &keys {
                if let Some(place) = key.list
                    && (place == 0 || place as usize > lists.len())
                {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the workbook has {} list(s), and there is no list {place}",
                            lists.len()
                        ),
                    });
                }
                match orientation {
                    Orientation::Rows => {
                        if key.line < area.start.col || key.line > area.end.col {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the column {} is not in the range {}",
                                    CellRef::column_text(key.line),
                                    area.to_text()
                                ),
                            });
                        }
                    }
                    Orientation::Columns => {
                        if key.line < area.start.row || key.line > area.end.row {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the row {} is not in the range {}",
                                    key.line + 1,
                                    area.to_text()
                                ),
                            });
                        }
                    }
                }
            }
            // A **merge** in the range: the tree refuses, because a merged
            // region is one cell and a line of a range is not.
            for merge in crate::merges::merges_of(node) {
                if merge.start == merge.end {
                    continue;
                }
                let touches = merge.start.row <= area.end.row
                    && merge.end.row >= area.start.row
                    && merge.start.col <= area.end.col
                    && merge.end.col >= area.start.col;
                if touches {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the merge {} is in the range, and a merged region is one cell",
                            merge.to_text()
                        ),
                    });
                }
            }
            // The **header**: said, or guessed. It is the first **row**
            // that stays in a sort of the rows, and the first **column**
            // in a sort of the columns.
            let header = self.header.map(|flag| flag.0).unwrap_or_else(|| guess_header(&cells, &area, orientation));
            // The **body**: the lines that move. A line is a row in a sort
            // of the rows, and a column in a sort of the columns; the
            // **cross** is the other way about.
            let (line_start, line_end, cross_start, cross_end) = match orientation {
                Orientation::Rows => (
                    if header && area.start.row < area.end.row { area.start.row + 1 } else { area.start.row },
                    area.end.row,
                    area.start.col,
                    area.end.col,
                ),
                Orientation::Columns => (
                    if header && area.start.col < area.end.col { area.start.col + 1 } else { area.start.col },
                    area.end.col,
                    area.start.row,
                    area.end.row,
                ),
            };
            // The **cell** of a line and a cross: the place they meet.
            let cell_of = |line: u32, cross: u32| -> CellRef {
                match orientation {
                    Orientation::Rows => CellRef { col: cross, row: line },
                    Orientation::Columns => CellRef { col: line, row: cross },
                }
            };
            // The **keys**: the value of the key cell of every line, for
            // every key.
            let book = crate::book_of(session.document());
            let mut lines: Vec<(u32, Vec<Value>)> = Vec::new();
            for line in line_start..=line_end {
                let mut values = Vec::with_capacity(keys.len());
                for key in &keys {
                    let cell = match orientation {
                        Orientation::Rows => CellRef { col: key.line, row: line },
                        Orientation::Columns => CellRef { col: line, row: key.line },
                    };
                    let value = match cells.get(cell) {
                        Some(entry) => match (&entry.value, &entry.formula) {
                            (Some(value), _) => value.clone(),
                            (None, Some(_)) => book.value_of(&name, cell),
                            (None, None) => Value::Empty,
                        },
                        None => Value::Empty,
                    };
                    values.push(value);
                }
                lines.push((line, values));
            }
            // The **order**: the lines are put in order, and the empty cells
            // are last whatever the order is. The first key that tells the
            // two lines apart decides, and the next one is looked at only
            // when the first cannot.
            let mut order: Vec<usize> = (0..lines.len()).collect();
            order.sort_by(|one, other| {
                let left = &lines[*one].1;
                let right = &lines[*other].1;
                for (index, key) in keys.iter().enumerate() {
                    let ordering = match key.list {
                        Some(place) => {
                            let list = lists.get(place.saturating_sub(1) as usize).cloned().unwrap_or_default();
                            compare_by_list(&left[index], &right[index], &list)
                        }
                        None => compare(&left[index], &right[index]),
                    };
                    if ordering == Ordering::Equal {
                        continue;
                    }
                    if key.descending
                        && !matches!(left[index], Value::Empty)
                        && !matches!(right[index], Value::Empty)
                    {
                        return ordering.reverse();
                    }
                    return ordering;
                }
                Ordering::Equal
            });
            if order.iter().enumerate().all(|(index, source)| index == *source) {
                return Ok(Outcome::new(format!("{name}!{} is already in order", area.to_text())).with_data(
                    json!({
                        "sheet": sheet,
                        "range": area.to_text(),
                        "orientation": orientation.to_text(),
                        "moved": 0,
                    }),
                ));
            }
            let mut notes = crate::notes::notes_of(node);
            let mut links = crate::links::links_of(node);
            let mut comments = crate::comments::comments_of(node);
            // The **lines** that move: what each cell of a line holds. They
            // are gathered first, because a line that moves may stand where
            // another line was, and a line that was there is still to be
            // moved.
            let mut contents: Vec<Vec<Option<crate::formula::Cell>>> = Vec::new();
            let mut carried: Vec<(String, crate::notes::Note, Option<crate::links::Link>)> = Vec::new();
            // The **comments** travel with the line they describe, as the
            // notes do.
            let mut carried_comments: Vec<(String, crate::comments::Comment)> = Vec::new();
            for line in line_start..=line_end {
                let mut across = Vec::new();
                for cross in cross_start..=cross_end {
                    let cell = cell_of(line, cross);
                    across.push(cells.get(cell).cloned());
                    let text = cell.to_text();
                    if let Some(note) = notes.get(&text) {
                        carried.push((text.clone(), note.clone(), links.get(&text).cloned()));
                    }
                    if let Some(comment) = comments.get(&text) {
                        carried_comments.push((text.clone(), comment.clone()));
                    }
                }
                contents.push(across);
            }
            let mut moved = 0u64;
            for (destination, source) in order.iter().enumerate() {
                let source_line = line_start + *source as u32;
                let destination_line = line_start + destination as u32;
                if source_line == destination_line {
                    continue;
                }
                let delta = i64::from(destination_line) - i64::from(source_line);
                for (index, cross) in (cross_start..=cross_end).enumerate() {
                    let from = cell_of(source_line, cross);
                    let to = cell_of(destination_line, cross);
                    let entry = contents[*source][index].clone();
                    let entry = match entry {
                        Some(mut entry) => {
                            if let Some(formula) = &entry.formula {
                                // A formula's **relative** references move
                                // with the cell, and a reference that would
                                // leave the sheet is refused.
                                let (row_delta, col_delta) = match orientation {
                                    Orientation::Rows => (delta, 0),
                                    Orientation::Columns => (0, delta),
                                };
                                let Some(shifted) = crate::formula::shift_in_formula(formula, row_delta, col_delta) else {
                                    return Err(EngineError::Failed {
                                        id: id.clone(),
                                        message: format!(
                                            "the formula in {} would leave the sheet when the line moves",
                                            from.to_text()
                                        ),
                                    });
                                };
                                entry.formula = Some(shifted);
                            }
                            Some(entry)
                        }
                        None => None,
                    };
                    match entry {
                        Some(entry) => {
                            cells.set_entry(to, entry);
                        }
                        None => {
                            cells.take(to);
                        }
                    }
                }
                moved += 1;
            }
            // The **line** a source line came to: the place it stands in
            // now, if it is one of the lines that moved.
            let place_of = |source_line: u32| -> Option<u32> {
                order
                    .iter()
                    .position(|source| line_start + *source as u32 == source_line)
                    .map(|place| line_start + place as u32)
            };
            // The **note** and the **link** travel with the line they
            // describe.
            for (address, note, link) in carried {
                let Some(cell) = CellRef::parse(&address) else { continue };
                let source_line = match orientation {
                    Orientation::Rows => cell.row,
                    Orientation::Columns => cell.col,
                };
                let Some(destination_line) = place_of(source_line) else { continue };
                let to = match orientation {
                    Orientation::Rows => CellRef { col: cell.col, row: destination_line },
                    Orientation::Columns => CellRef { col: destination_line, row: cell.row },
                }
                .to_text();
                notes.remove(&address);
                links.remove(&address);
                notes.insert(to.clone(), note);
                if let Some(link) = link {
                    links.insert(to, link);
                }
            }
            // The **comment** travels with the line it describes.
            for (address, comment) in carried_comments {
                let Some(cell) = CellRef::parse(&address) else { continue };
                let source_line = match orientation {
                    Orientation::Rows => cell.row,
                    Orientation::Columns => cell.col,
                };
                let Some(destination_line) = place_of(source_line) else { continue };
                let to = match orientation {
                    Orientation::Rows => CellRef { col: cell.col, row: destination_line },
                    Orientation::Columns => CellRef { col: destination_line, row: cell.row },
                }
                .to_text();
                comments.remove(&address);
                comments.insert(to, comment);
            }
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["cells"] = crate::cells_into(&cells);
            node.data["notes"] = crate::notes::notes_into(&notes);
            node.data["links"] = crate::links::links_into(&links);
            node.data["comments"] = crate::comments::comments_into(&comments);
            // The **keys** of the sort, as the answer says them: the line
            // of each of them and whether it is descending.
            let said: Vec<Json> = keys
                .iter()
                .map(|key| match orientation {
                    Orientation::Rows => json!({
                        "column": CellRef::column_text(key.line),
                        "descending": key.descending,
                    }),
                    Orientation::Columns => json!({
                        "row": key.line + 1,
                        "descending": key.descending,
                    }),
                })
                .collect();
            let mut data = json!({
                "sheet": sheet,
                "range": area.to_text(),
                "orientation": orientation.to_text(),
                "header": header,
                "descending": keys[0].descending,
                "keys": said,
                "moved": moved,
            });
            if let Some(map) = data.as_object_mut() {
                match orientation {
                    Orientation::Rows => {
                        map.insert("column".to_string(), json!(CellRef::column_text(keys[0].line)));
                    }
                    Orientation::Columns => {
                        map.insert("row".to_string(), json!(keys[0].line + 1));
                    }
                }
            }
            Ok(Outcome::changed(format!("sorted {name}!{}", area.to_text())).with_data(data))
        }
    }
}
