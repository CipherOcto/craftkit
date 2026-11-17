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
//! # The order
//!
//! The tree's order: numbers, then text, then booleans, then errors,
//! and the **empty** cells last — whatever the order is. The text is
//! compared without case, and a search that says the case matters
//! puts the capitals after the small letters.
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

/// The **order** of two values, the tree's way.
///
/// The empty cells are last, and the numbers come before the text,
/// and the text before the booleans, and the booleans before the
/// errors.
fn compare(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Empty, Value::Empty) => Ordering::Equal,
        (Value::Empty, _) => Ordering::Greater,
        (_, Value::Empty) => Ordering::Less,
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

/// Whether the first row of a range is a **header**.
///
/// The tree's guess: a column whose first cell is text and whose
/// second cell is not text and is not empty votes for it, and the
/// votes must be more than none.
fn guess_header(cells: &Cells, area: &Area) -> bool {
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

/// A **key** of a sort: the column it sorts by, whether the order is
/// **descending**, and the **list** it orders the words by, when it
/// orders them by a list.
struct Key {
    /// The column the key sorts by.
    column: u32,
    /// Whether the order is descending.
    descending: bool,
    /// The place of the list the words are ordered by, when they are.
    list: Option<u64>,
}

/// The **keys** of a sort, from the caller's list.
///
/// A key names the **column** it sorts by, and says whether the order is
/// **descending**. A list with no key in it is refused: a sort with no
/// key is not a sort.
fn keys_of(value: &Json, id: &str) -> Result<Vec<Key>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "the keys of a sort are a list".to_string(),
        });
    };
    let mut keys = Vec::new();
    for key in list {
        let Some(column) = key.get("column").and_then(|column| column.as_str()) else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "a key names the column it sorts by".to_string(),
            });
        };
        let descending = key.get("descending").and_then(|descending| descending.as_bool()).unwrap_or(false);
        let list = key.get("list").and_then(|list| list.as_u64());
        keys.push(Key { column: column_at(column, id)?, descending, list });
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
    /// **Sort** a range of cells by one of its columns.
    ///
    /// The **keys** of the sort may be more than one: the first key is
    /// the one that decides, and the next one is looked at only when the
    /// first cannot tell two rows apart.
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
            /// The column to sort by, as `B`. Without it, the first column of
            /// the range.
            column: Option<Text>,
            /// Whether the order is **descending**.
            descending: Option<Bool>,
            /// Whether the first row is a **header** and stays where it is.
            header: Option<Bool>,
            /// The **keys** of the sort: a list of them, and the first is
            /// the one that decides. Each key names the **column** it
            /// sorts by, says whether the order is **descending**, and may
            /// name the **list** the words are ordered by — the place of
            /// the list in the workbook.
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
            // The **keys**: the ones that were named, or the one column
            // that was named, or the first column of the range.
            let keys = match &self.keys {
                Some(keys) => {
                    if self.column.is_some() || self.descending.is_some() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "a sort is sorted by its keys or by its column, and not by both".to_string(),
                        });
                    }
                    keys_of(keys, &id)?
                }
                None => {
                    let column = match &self.column {
                        Some(column) => column_at(&column.0, &id)?,
                        None => area.start.col,
                    };
                    vec![Key {
                        column,
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
                if key.column < area.start.col || key.column > area.end.col {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the column {} is not in the range {}",
                            CellRef::column_text(key.column),
                            area.to_text()
                        ),
                    });
                }
            }
            // A **merge** in the range: the tree refuses, because a merged
            // region is one cell and a row of a range is not.
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
            // The **header**: said, or guessed.
            let header = self.header.map(|flag| flag.0).unwrap_or_else(|| guess_header(&cells, &area));
            let body = if header && area.start.row < area.end.row {
                Area::new(CellRef { col: area.start.col, row: area.start.row + 1 }, area.end)
            } else {
                area
            };
            // The **keys**: the value of the key cell of every row, for
            // every key.
            let book = crate::book_of(session.document());
            let mut rows: Vec<(u32, Vec<Value>)> = Vec::new();
            for row in body.start.row..=body.end.row {
                let mut values = Vec::with_capacity(keys.len());
                for key in &keys {
                    let cell = CellRef { col: key.column, row };
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
                rows.push((row, values));
            }
            // The **order**: the rows are put in order, and the empty cells
            // are last whatever the order is. The first key that tells the
            // two rows apart decides, and the next one is looked at only
            // when the first cannot.
            let mut order: Vec<usize> = (0..rows.len()).collect();
            order.sort_by(|one, other| {
                let left = &rows[*one].1;
                let right = &rows[*other].1;
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
                        "moved": 0,
                    }),
                ));
            }
            let mut notes = crate::notes::notes_of(node);
            let mut links = crate::links::links_of(node);
            let mut comments = crate::comments::comments_of(node);
            // The **rows** that move: what each cell of a row holds. They
            // are gathered first, because a row that moves may stand where
            // another row was, and a row that was there is still to be
            // moved.
            let columns: Vec<u32> = (area.start.col..=area.end.col).collect();
            let mut contents: Vec<Vec<Option<crate::formula::Cell>>> = Vec::new();
            let mut carried: Vec<(String, crate::notes::Note, Option<crate::links::Link>)> = Vec::new();
            // The **comments** travel with the row they describe, as the
            // notes do.
            let mut carried_comments: Vec<(String, crate::comments::Comment)> = Vec::new();
            for row in body.start.row..=body.end.row {
                let mut line = Vec::new();
                for column in &columns {
                    let cell = CellRef { col: *column, row };
                    line.push(cells.get(cell).cloned());
                    let text = cell.to_text();
                    if let Some(note) = notes.get(&text) {
                        carried.push((text.clone(), note.clone(), links.get(&text).cloned()));
                    }
                    if let Some(comment) = comments.get(&text) {
                        carried_comments.push((text.clone(), comment.clone()));
                    }
                }
                contents.push(line);
            }
            let mut moved = 0u64;
            for (destination, source) in order.iter().enumerate() {
                let source_row = body.start.row + *source as u32;
                let destination_row = body.start.row + destination as u32;
                if source_row == destination_row {
                    continue;
                }
                let delta = i64::from(destination_row) - i64::from(source_row);
                for (index, column) in columns.iter().enumerate() {
                    let from = CellRef { col: *column, row: source_row };
                    let to = CellRef { col: *column, row: destination_row };
                    let entry = contents[*source][index].clone();
                    let entry = match entry {
                        Some(mut entry) => {
                            if let Some(formula) = &entry.formula {
                                // A formula's **relative** references move
                                // with the cell, and a reference that would
                                // leave the sheet is refused.
                                let Some(shifted) = crate::formula::shift_in_formula(formula, delta, 0) else {
                                    return Err(EngineError::Failed {
                                        id: id.clone(),
                                        message: format!(
                                            "the formula in {} would leave the sheet when the row moves",
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
            // The **note** and the **link** travel with the row they
            // describe.
            for (address, note, link) in carried {
                let Some(cell) = CellRef::parse(&address) else { continue };
                let source_row = cell.row;
                let Some(place) = order.iter().position(|source| body.start.row + *source as u32 == source_row)
                else {
                    continue;
                };
                let destination_row = body.start.row + place as u32;
                let to = CellRef { col: cell.col, row: destination_row }.to_text();
                notes.remove(&address);
                links.remove(&address);
                notes.insert(to.clone(), note);
                if let Some(link) = link {
                    links.insert(to, link);
                }
            }
            // The **comment** travels with the row it describes.
            for (address, comment) in carried_comments {
                let Some(cell) = CellRef::parse(&address) else { continue };
                let source_row = cell.row;
                let Some(place) = order.iter().position(|source| body.start.row + *source as u32 == source_row)
                else {
                    continue;
                };
                let destination_row = body.start.row + place as u32;
                let to = CellRef { col: cell.col, row: destination_row }.to_text();
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
            // The **keys** of the sort, as the answer says them: the
            // column of each of them and whether it is descending.
            let said: Vec<Json> = keys
                .iter()
                .map(|key| {
                    json!({
                        "column": CellRef::column_text(key.column),
                        "descending": key.descending,
                    })
                })
                .collect();
            Ok(Outcome::changed(format!("sorted {name}!{}", area.to_text())).with_data(json!({
                "sheet": sheet,
                "range": area.to_text(),
                "column": CellRef::column_text(keys[0].column),
                "header": header,
                "descending": keys[0].descending,
                "keys": said,
                "moved": moved,
            })))

        }
    }
}
