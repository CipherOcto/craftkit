//! The **outline** of a sheet: the groups of its rows and of its
//! columns.
//!
//! A row may be **in a group**: the rows of a group are gathered
//! under a **summary row**, and the summary row may be before the
//! group or after it. The tree's outline is a **level** on every row
//! — from one to seven — and the groups are worked out from the
//! levels: the runs of rows at or above a level are the groups, and a
//! gap between two rows ends every group.
//!
//! So the **levels** are the fact and the **groups** are derived: a
//! row that is moved is regrouped, and nothing is written down that
//! could be wrong.
//!
//! The rules:
//!
//! - the deepest level a group may have is **seven**, the tree's own,
//!   and a group that would go deeper is **refused**;
//! - a row that is **not** in a group has the level **zero**, and
//!   taking a group off a row that has none is **refused**;
//! - the **summary row** of a group is the row after it — or the row
//!   before it, when the sheet says its summaries sit before their
//!   detail — and a group that has no such row has no summary;
//! - a group whose rows are **all hidden** is a **collapsed** group:
//!   it is said, and it is derived from the rows that are hidden.
//!
//! # What the port found
//!
//! - **The tree's outline is a flag on every row of the sheet** — the
//!   tree keeps a line of information for every row, hidden or not,
//!   and the outline level is on that line. Here a sheet carries the
//!   **levels of the rows that are in a group**, and a row that is not
//!   on the list is in no group.
//! - **The tree's outline is of rows and of columns**, and the tree's
//!   summary may sit above or below the rows and left or right of the
//!   columns. Here the outline is of the **rows**, and the summary of
//!   a group is below it unless the sheet says otherwise.
//! - **The tree's group takes a whole block of rows** — `5:8` — and
//!   the tree's ungroup takes one back. Here so does this one.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The **deepest** level a group may have: the tree's own.
const MAX_LEVEL: u8 = 7;

/// A **group** of rows: a run of rows at or above a level, and the
/// summary row that gathers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// The level of the group.
    pub level: u8,
    /// The first row of the group.
    pub start: u64,
    /// The last row of the group.
    pub end: u64,
    /// The summary row of the group.
    pub summary: u64,
    /// Whether the group has a summary row.
    pub has_summary: bool,
    /// Whether every row of the group is hidden.
    pub collapsed: bool,
}

/// The **levels** of the rows of a sheet: by the number of the row.
pub fn levels_of(node: &Node) -> BTreeMap<u64, u8> {
    let mut levels = BTreeMap::new();
    let Some(map) = node.data.get("row_outline").and_then(|levels| levels.as_object()) else {
        return levels;
    };
    for (row, level) in map {
        let Some(row) = row.parse::<u64>().ok() else { continue };
        let Some(level) = level.as_u64() else { continue };
        if level == 0 || level > MAX_LEVEL as u64 {
            continue;
        }
        levels.insert(row, level as u8);
    }
    levels
}

/// The levels of a sheet, as they are kept.
pub fn levels_into(levels: &BTreeMap<u64, u8>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (row, level) in levels {
        map.insert(row.to_string(), json!(level));
    }
    Json::Object(map)
}

/// Whether the sheet's **summary rows** sit before their detail.
pub fn summary_below(node: &Node) -> bool {
    node.data.get("summary_below").and_then(|below| below.as_bool()).unwrap_or(true)
}

/// Whether the sheet's **summary columns** sit before their detail.
pub fn summary_right(node: &Node) -> bool {
    node.data.get("summary_right").and_then(|right| right.as_bool()).unwrap_or(true)
}

/// The **levels** of the columns of a sheet: by the letters of the
/// column.
pub fn column_levels_of(node: &Node) -> BTreeMap<u32, u8> {
    let mut levels = BTreeMap::new();
    let Some(map) = node.data.get("column_outline").and_then(|levels| levels.as_object()) else {
        return levels;
    };
    for (column, level) in map {
        let Some(index) = crate::formula::CellRef::column_index(column.trim()) else { continue };
        let Some(level) = level.as_u64() else { continue };
        if level == 0 || level > MAX_LEVEL as u64 {
            continue;
        }
        levels.insert(index, level as u8);
    }
    levels
}

/// The levels of the columns of a sheet, as they are kept.
pub fn column_levels_into(levels: &BTreeMap<u32, u8>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (column, level) in levels {
        map.insert(crate::formula::CellRef::column_text(*column), json!(level));
    }
    Json::Object(map)
}

/// The **columns** of a sheet that are hidden.
fn hidden_columns(node: &Node) -> Vec<u32> {
    node.data
        .get("hidden_columns")
        .and_then(|columns| columns.as_array())
        .map(|columns| {
            columns
                .iter()
                .filter_map(|column| column.as_str())
                .filter_map(|column| crate::formula::CellRef::column_index(column.trim()))
                .collect()
        })
        .unwrap_or_default()
}

/// The **groups** of the columns of a sheet, worked out from the
/// levels.
pub fn column_groups_of(node: &Node) -> Vec<Group> {
    let levels: BTreeMap<u64, u8> =
        column_levels_of(node).into_iter().map(|(column, level)| (column as u64, level)).collect();
    let hidden = hidden_columns(node);
    let is_hidden = |column: u64| hidden.contains(&(column as u32));
    groups_from(&levels, &is_hidden, !summary_right(node))
}

/// The **rows** of a sheet that are hidden.
fn hidden_rows(node: &Node) -> Vec<u64> {
    node.data
        .get("hidden_rows")
        .and_then(|rows| rows.as_array())
        .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect())
        .unwrap_or_default()
}

/// The **groups** of the rows of a sheet, worked out from the levels.
///
/// The groups are the runs of rows at or above a level, and a gap
/// between two rows ends every group.
pub fn groups_of(node: &Node) -> Vec<Group> {
    let levels = levels_of(node);
    let hidden = hidden_rows(node);
    let is_hidden = |row: u64| hidden.contains(&row);
    groups_from(&levels, &is_hidden, !summary_below(node))
}

/// The **groups** of a list of levels, worked out the tree's way.
///
/// `summary_before` says whether the summary of a group sits before
/// the group rather than after it.
fn groups_from(
    levels: &BTreeMap<u64, u8>,
    is_hidden: &dyn Fn(u64) -> bool,
    summary_before: bool,
) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    // The open group of every level: where it starts, and whether
    // every row of it is hidden.
    let mut open: [Option<(u64, bool)>; MAX_LEVEL as usize] = [None; MAX_LEVEL as usize];
    let mut previous: Option<u64> = None;
    for (row, level) in levels {
        let level = (*level).min(MAX_LEVEL);
        if let Some(previous) = previous {
            // A **gap** ends every group, and a row that is shallower
            // than the one before it ends the groups below its own
            // level.
            if previous.checked_add(1) != Some(*row) {
                close(&mut open, &mut groups, 1, previous, summary_before);
            } else if level < MAX_LEVEL {
                close(&mut open, &mut groups, level + 1, previous, summary_before);
            }
        }
        for slot in open.iter_mut().take(level as usize) {
            match slot {
                Some((_, hidden)) => *hidden &= is_hidden(*row),
                None => *slot = Some((*row, is_hidden(*row))),
            }
        }
        previous = Some(*row);
    }
    if let Some(previous) = previous {
        close(&mut open, &mut groups, 1, previous, summary_before);
    }
    groups.sort_by_key(|group| (group.end, group.level));
    groups
}

/// **Close** the groups of the levels from `from` up, at the row
/// `end`.
fn close(
    open: &mut [Option<(u64, bool)>; MAX_LEVEL as usize],
    groups: &mut Vec<Group>,
    from: u8,
    end: u64,
    summary_before: bool,
) {
    for level in (from..=MAX_LEVEL).rev() {
        let Some((start, collapsed)) = open.get_mut(level as usize - 1).and_then(|slot| slot.take()) else {
            continue;
        };
        let summary = if summary_before { start.saturating_sub(1) } else { end + 1 };
        let has_summary = summary >= 1 && !(start..=end).contains(&summary);
        groups.push(Group { level, start, end, summary, has_summary, collapsed });
    }
}

/// A **group** of rows, as it is reported.
pub fn group_json(group: &Group) -> Json {
    group_json_with(group, |row| json!(row))
}

/// A **group**, as it is reported, with the names of its lines.
fn group_json_with(group: &Group, name: impl Fn(u64) -> Json) -> Json {
    json!({
        "level": group.level,
        "start": name(group.start),
        "end": name(group.end),
        "summary": name(group.summary),
        "has_summary": group.has_summary,
        "collapsed": group.collapsed,
    })
}

/// The **rows** a range names: `5:8` or `5`.
fn rows_of(text: &str, id: &str) -> Result<(u64, u64), EngineError> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), last.trim()),
        None => (trimmed, trimmed),
    };
    let row = |text: &str| -> Result<u64, EngineError> {
        text.parse::<u64>().map_err(|_| EngineError::Failed {
            id: id.to_string(),
            message: format!("`{text}` is not a row: a row is counted from one"),
        })
    };
    let first = row(first)?;
    let last = row(last)?;
    if first == 0 || last == 0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a row is counted from one".to_string(),
        });
    }
    if first > last {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{trimmed}` is not a range of rows: it runs backwards"),
        });
    }
    Ok((first, last))
}

/// The **columns** a range names: `C:E` or `C`.
fn columns_of(text: &str, id: &str) -> Result<(u32, u32), EngineError> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), last.trim()),
        None => (trimmed, trimmed),
    };
    let column = |text: &str| -> Result<u32, EngineError> {
        crate::formula::CellRef::column_index(text).ok_or_else(|| EngineError::Failed {
            id: id.to_string(),
            message: format!("`{text}` is not a column: a column is `A`, `B` or `AA`"),
        })
    };
    let first = column(first)?;
    let last = column(last)?;
    if first > last {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{trimmed}` is not a range of columns: it runs backwards"),
        });
    }
    Ok((first, last))
}

/// The **sheet** a command is about, or the error that says there is
/// no such sheet.
fn sheet_of(session: &Session, sheet: u64, id: &str) -> Result<u64, EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is not a sheet"),
        });
    }
    Ok(sheet)
}

commands! {
    /// **Group** a range of rows.
    ///
    /// Every row of the range goes one level deeper, and a group that
    /// would go deeper than the tree's seven levels is refused.
    grid.row.group {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The rows that are grouped: `5:8`.
            rows: Text,
        },
        run: {
            let id = "grid.row.group".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = rows_of(&self.rows.0, &id)?;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut levels = levels_of(&node);
            for row in first..=last {
                let level = levels.get(&row).copied().unwrap_or(0);
                if level + 1 > MAX_LEVEL {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the row {row} is already in a group of the deepest level, which is {MAX_LEVEL}"
                        ),
                    });
                }
            }
            for row in first..=last {
                let level = levels.get(&row).copied().unwrap_or(0);
                levels.insert(row, level + 1);
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["row_outline"] = levels_into(&levels);
            }
            Ok(Outcome::changed(format!("the rows {first}:{last} are grouped"))
                .with_data(json!({ "sheet": sheet, "first": first, "last": last })))

        }
    }

    /// **Ungroup** a range of rows.
    ///
    /// Every row of the range comes one level up, and a row that is in no
    /// group at all is refused: there is nothing to undo on it.
    grid.row.ungroup {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The rows that are ungrouped: `5:8`.
            rows: Text,
        },
        run: {
            let id = "grid.row.ungroup".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = rows_of(&self.rows.0, &id)?;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut levels = levels_of(&node);
            for row in first..=last {
                if levels.get(&row).copied().unwrap_or(0) == 0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the row {row} is in no group, and there is nothing to undo on it"),
                    });
                }
            }
            for row in first..=last {
                let level = levels.get(&row).copied().unwrap_or(0);
                if level == 1 {
                    levels.remove(&row);
                } else {
                    levels.insert(row, level - 1);
                }
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["row_outline"] = levels_into(&levels);
            }
            Ok(Outcome::changed(format!("the rows {first}:{last} are ungrouped"))
                .with_data(json!({ "sheet": sheet, "first": first, "last": last })))

        }
    }

    /// **Read** the groups of a sheet.
    grid.row.groups {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
        },
        run: {
            let id = "grid.row.groups".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let groups: Vec<Json> = groups_of(node).iter().map(group_json).collect();
            Ok(Outcome::new(format!("the sheet `{sheet}` has {} group(s)", groups.len()))
                .with_data(json!({ "sheet": sheet, "groups": groups })))

        }
    }

    /// **Say** where the summary rows of a sheet sit.
    ///
    /// The tree's own setting: the summaries sit below their detail, or
    /// above it; and the summaries of the columns sit right of their
    /// detail, or left of it.
    grid.sheet.summary {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Whether the summary rows sit **below** their detail.
            below: Option<Bool>,
            /// Whether the summary columns sit **right** of their detail.
            right: Option<Bool>,
        },
        run: {
            let id = "grid.sheet.summary".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let below = self.below.map(|below| below.0);
            let right = self.right.map(|right| right.0);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                if let Some(below) = below {
                    node.data["summary_below"] = json!(below);
                }
                if let Some(right) = right {
                    node.data["summary_right"] = json!(right);
                }
            }
            // The tree's own rule: the command says where the summaries
            // **are**, and not the ones that were given to it. A summary
            // that is not said is below and to the right.
            let node = session.document().node(sheet);
            let below = node.map(summary_below).unwrap_or(true);
            let right = node.map(summary_right).unwrap_or(true);
            Ok(Outcome::changed(format!("the summaries of the sheet `{sheet}` are placed"))
                .with_data(json!({ "sheet": sheet, "below": below, "right": right })))

        }
    }

    /// **Group** a range of columns.
    ///
    /// Every column of the range goes one level deeper, and a group that
    /// would go deeper than the tree's seven levels is refused.
    grid.column.group {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The columns that are grouped: `C:E`.
            columns: Text,
        },
        run: {
            let id = "grid.column.group".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = columns_of(&self.columns.0, &id)?;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut levels = column_levels_of(&node);
            for column in first..=last {
                let level = levels.get(&column).copied().unwrap_or(0);
                if level + 1 > MAX_LEVEL {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the column {} is already in a group of the deepest level, which is {MAX_LEVEL}",
                            crate::formula::CellRef::column_text(column)
                        ),
                    });
                }
            }
            for column in first..=last {
                let level = levels.get(&column).copied().unwrap_or(0);
                levels.insert(column, level + 1);
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["column_outline"] = column_levels_into(&levels);
            }
            Ok(Outcome::changed(format!(
                "the columns {}:{} are grouped",
                crate::formula::CellRef::column_text(first),
                crate::formula::CellRef::column_text(last)
            ))
            .with_data(json!({
                "sheet": sheet,
                "first": crate::formula::CellRef::column_text(first),
                "last": crate::formula::CellRef::column_text(last),
            })))

        }
    }

    /// **Ungroup** a range of columns.
    ///
    /// Every column of the range comes one level up, and a column that is
    /// in no group at all is refused: there is nothing to undo on it.
    grid.column.ungroup {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The columns that are ungrouped: `C:E`.
            columns: Text,
        },
        run: {
            let id = "grid.column.ungroup".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = columns_of(&self.columns.0, &id)?;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut levels = column_levels_of(&node);
            for column in first..=last {
                if levels.get(&column).copied().unwrap_or(0) == 0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the column {} is in no group, and there is nothing to undo on it",
                            crate::formula::CellRef::column_text(column)
                        ),
                    });
                }
            }
            for column in first..=last {
                let level = levels.get(&column).copied().unwrap_or(0);
                if level == 1 {
                    levels.remove(&column);
                } else {
                    levels.insert(column, level - 1);
                }
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["column_outline"] = column_levels_into(&levels);
            }
            Ok(Outcome::changed(format!(
                "the columns {}:{} are ungrouped",
                crate::formula::CellRef::column_text(first),
                crate::formula::CellRef::column_text(last)
            ))
            .with_data(json!({
                "sheet": sheet,
                "first": crate::formula::CellRef::column_text(first),
                "last": crate::formula::CellRef::column_text(last),
            })))

        }
    }

    /// **Read** the groups of the columns of a sheet.
    grid.column.groups {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
        },
        run: {
            let id = "grid.column.groups".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let groups: Vec<Json> = column_groups_of(node)
                .iter()
                .map(|group| {
                    group_json_with(group, |column| json!(crate::formula::CellRef::column_text(column as u32)))
                })
                .collect();
            Ok(Outcome::new(format!("the sheet `{sheet}` has {} column group(s)", groups.len()))
                .with_data(json!({ "sheet": sheet, "groups": groups })))

        }
    }

    /// **Show** the rows or the columns down to an outline level.
    ///
    /// The tree's level buttons: a level of **one** shows the top-level
    /// summaries only, and a deeper level shows more of the detail. The
    /// rows of a group that is deeper than the level asked for are hidden,
    /// and the rows of a group that is not deeper are shown.
    ///
    /// The tree's **collapsed** mark — the little **+** beside a summary
    /// row — is not written here: a group whose rows are all hidden is
    /// collapsed, and that is worked out from the rows.
    grid.outline.levels {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The level of the rows to show: one is only the top-level
            /// summaries.
            rows: Option<Number>,
            /// The level of the columns to show.
            cols: Option<Number>,
        },
        run: {
            let id = "grid.outline.levels".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            if self.rows.is_none() && self.cols.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message:
                        "give `rows` or `cols`: the outline level to show (one is only the top-level summaries)"
                            .to_string(),
                });
            }
            let rows = match self.rows {
                Some(level) => Some(level_of(level.0, &id)?),
                None => None,
            };
            let cols = match self.cols {
                Some(level) => Some(level_of(level.0, &id)?),
                None => None,
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut hidden_row_numbers = hidden_rows(&node);
            if let Some(level) = rows {
                for (row, row_level) in levels_of(&node) {
                    if row_level >= level {
                        if !hidden_row_numbers.contains(&row) {
                            hidden_row_numbers.push(row);
                        }
                    } else {
                        hidden_row_numbers.retain(|other| *other != row);
                    }
                }
                hidden_row_numbers.sort_unstable();
            }
            let mut hidden_column_texts: Vec<String> =
                hidden_columns(&node).into_iter().map(crate::formula::CellRef::column_text).collect();
            if let Some(level) = cols {
                for (column, column_level) in column_levels_of(&node) {
                    let text = crate::formula::CellRef::column_text(column);
                    if column_level >= level {
                        if !hidden_column_texts.contains(&text) {
                            hidden_column_texts.push(text);
                        }
                    } else {
                        hidden_column_texts.retain(|other| *other != text);
                    }
                }
                hidden_column_texts.sort();
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["hidden_rows"] = json!(hidden_row_numbers);
                node.data["hidden_columns"] = json!(hidden_column_texts);
            }
            Ok(Outcome::changed(format!("the outline of the sheet `{sheet}` is shown to a level")).with_data(
                json!({
                    "sheet": sheet,
                    "rows": rows,
                    "cols": cols,
                }),
            ))

        }
    }

    /// **Hide** the **detail** of a group of rows.
    ///
    /// The tree's Hide Detail: the rows of the group are hidden, and the
    /// summary row of the group is not — it is not in the group.
    grid.row.detail.hide {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The rows of the group: `5:8`.
            rows: Text,
            /// The level of the group, when the rows are in more than one.
            level: Option<Number>,
        },
        run: {
            let id = "grid.row.detail.hide".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = rows_of(&self.rows.0, &id)?;
            let level = match self.level {
                Some(level) => Some(level_of(level.0, &id)?),
                None => None,
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let hidden = hidden_rows(&node);
            let is_hidden = |row: u64| hidden.contains(&row);
            let Some((group, rows, _)) =
                detail_of(&levels_of(&node), &groups_of(&node), first, last, level, &is_hidden)
            else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no group of the rows {first}:{last}"),
                });
            };
            let mut hidden = hidden;
            for row in &rows {
                if !hidden.contains(row) {
                    hidden.push(*row);
                }
            }
            hidden.sort_unstable();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["hidden_rows"] = json!(hidden);
            }
            Ok(Outcome::changed(format!("the detail of the group {first}:{last} is hidden")).with_data(json!({
                "sheet": sheet,
                "start": group.start,
                "end": group.end,
                "level": group.level,
                "hidden": rows,
            })))

        }
    }

    /// **Show** the **detail** of a group of rows.
    ///
    /// The tree's Show Detail: the rows of the group are shown, and the
    /// rows of an inner group that is **collapsed on its own** stay
    /// hidden.
    ///
    /// The tree writes a mark beside a summary row when a group is
    /// collapsed on its own. Here a group is collapsed when all of its
    /// rows are hidden, and a group that is collapsed on its own is one
    /// whose summary row is still there.
    grid.row.detail.show {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The rows of the group: `5:8`.
            rows: Text,
            /// The level of the group, when the rows are in more than one.
            level: Option<Number>,
        },
        run: {
            let id = "grid.row.detail.show".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = rows_of(&self.rows.0, &id)?;
            let level = match self.level {
                Some(level) => Some(level_of(level.0, &id)?),
                None => None,
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let hidden = hidden_rows(&node);
            let is_hidden = |row: u64| hidden.contains(&row);
            let Some((group, rows, collapsed)) =
                detail_of(&levels_of(&node), &groups_of(&node), first, last, level, &is_hidden)
            else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no group of the rows {first}:{last}"),
                });
            };
            let mut hidden = hidden;
            for row in &rows {
                if collapsed.contains(row) {
                    continue;
                }
                hidden.retain(|other| other != row);
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["hidden_rows"] = json!(hidden);
            }
            Ok(Outcome::changed(format!("the detail of the group {first}:{last} is shown")).with_data(json!({
                "sheet": sheet,
                "start": group.start,
                "end": group.end,
                "level": group.level,
                "shown": rows,
            })))

        }
    }

    /// **Hide** the **detail** of a group of columns.
    ///
    /// The tree's Hide Detail, on the columns: the columns of the group
    /// are hidden, and the summary column of the group is not — it is not
    /// in the group.
    grid.column.detail.hide {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The columns of the group: `B:E`.
            columns: Text,
            /// The level of the group, when the columns are in more than one.
            level: Option<Number>,
        },
        run: {
            let id = "grid.column.detail.hide".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = columns_of(&self.columns.0, &id)?;
            let level = match self.level {
                Some(level) => Some(level_of(level.0, &id)?),
                None => None,
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let hidden: Vec<u32> = hidden_columns(&node);
            let is_hidden = |column: u64| hidden.contains(&(column as u32));
            let Some((group, columns, _)) = detail_of(
                &column_numbers_of(&node),
                &column_groups_of(&node),
                first as u64,
                last as u64,
                level,
                &is_hidden,
            ) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no group of the columns {first}:{last}"),
                });
            };
            let mut hidden = hidden;
            for column in &columns {
                let column = *column as u32;
                if !hidden.contains(&column) {
                    hidden.push(column);
                }
            }
            hidden.sort_unstable();
            let mut texts: Vec<String> = hidden.into_iter().map(crate::formula::CellRef::column_text).collect();
            texts.sort();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["hidden_columns"] = json!(texts);
            }
            Ok(Outcome::changed(format!("the detail of the group {first}:{last} is hidden")).with_data(json!({
                "sheet": sheet,
                "start": crate::formula::CellRef::column_text(group.start as u32),
                "end": crate::formula::CellRef::column_text(group.end as u32),
                "level": group.level,
                "hidden": columns
                    .iter()
                    .map(|column| crate::formula::CellRef::column_text(*column as u32))
                    .collect::<Vec<_>>(),
            })))

        }
    }

    /// **Show** the **detail** of a group of columns.
    ///
    /// The tree's Show Detail, on the columns: the columns of the group
    /// are shown, and the columns of an inner group that is **collapsed
    /// on its own** stay hidden.
    grid.column.detail.show {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The columns of the group: `B:E`.
            columns: Text,
            /// The level of the group, when the columns are in more than one.
            level: Option<Number>,
        },
        run: {
            let id = "grid.column.detail.show".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let (first, last) = columns_of(&self.columns.0, &id)?;
            let level = match self.level {
                Some(level) => Some(level_of(level.0, &id)?),
                None => None,
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let hidden: Vec<u32> = hidden_columns(&node);
            let is_hidden = |column: u64| hidden.contains(&(column as u32));
            let Some((group, columns, collapsed)) = detail_of(
                &column_numbers_of(&node),
                &column_groups_of(&node),
                first as u64,
                last as u64,
                level,
                &is_hidden,
            ) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no group of the columns {first}:{last}"),
                });
            };
            let mut hidden = hidden;
            for column in &columns {
                if collapsed.contains(column) {
                    continue;
                }
                hidden.retain(|other| other != &(*column as u32));
            }
            let mut texts: Vec<String> = hidden.into_iter().map(crate::formula::CellRef::column_text).collect();
            texts.sort();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["hidden_columns"] = json!(texts);
            }
            Ok(Outcome::changed(format!("the detail of the group {first}:{last} is shown")).with_data(json!({
                "sheet": sheet,
                "start": crate::formula::CellRef::column_text(group.start as u32),
                "end": crate::formula::CellRef::column_text(group.end as u32),
                "level": group.level,
                "shown": columns
                    .iter()
                    .map(|column| crate::formula::CellRef::column_text(*column as u32))
                    .collect::<Vec<_>>(),
            })))

        }
    }
}

/// An outline **level**: from one to the tree's seven levels and one
/// more, which is the level of everything.
fn level_of(level: f64, id: &str) -> Result<u8, EngineError> {
    if !level.is_finite() || level < 1.0 || level > MAX_LEVEL as f64 + 1.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "the outline level is from one to {}, and {level} is not one of them",
                MAX_LEVEL + 1
            ),
        });
    }
    Ok(level as u8)
}

/// The **group** a command is about: the deepest group that holds the
/// lines named, or the one whose summary is among them.
///
/// The tree's own rule: a group that holds the lines, and the deepest
/// of those; and when no group holds them, the group whose summary
/// line is one of them, and the deepest of those.
fn group_of(groups: &[Group], first: u64, last: u64, level: Option<u8>) -> Option<Group> {
    let at_level = |group: &Group| level.is_none_or(|level| group.level == level);
    let holds = groups
        .iter()
        .filter(|group| at_level(group) && group.start <= last && group.end >= first)
        .copied()
        .collect::<Vec<_>>();
    if !holds.is_empty() {
        let deepest = holds.iter().map(|group| group.level).max().unwrap_or(0);
        return holds.into_iter().find(|group| group.level == deepest);
    }
    let summaries = groups
        .iter()
        .filter(|group| {
            at_level(group) && group.has_summary && group.summary >= first && group.summary <= last
        })
        .copied()
        .collect::<Vec<_>>();
    let deepest = summaries.iter().map(|group| group.level).max().unwrap_or(0);
    summaries.into_iter().find(|group| group.level == deepest)
}

/// The **rows** of a group: the rows of its range that are in it.
fn rows_in(levels: &BTreeMap<u64, u8>, group: &Group) -> Vec<u64> {
    levels
        .range(group.start..=group.end)
        .filter(|(_, level)| **level >= group.level)
        .map(|(row, _)| *row)
        .collect()
}

/// The **detail** of a group: the group itself, the lines that are in
/// it, and the lines of the inner groups that are collapsed on their
/// own.
fn detail_of(
    levels: &BTreeMap<u64, u8>,
    groups: &[Group],
    first: u64,
    last: u64,
    level: Option<u8>,
    is_hidden: &dyn Fn(u64) -> bool,
) -> Option<(Group, Vec<u64>, Vec<u64>)> {
    let group = group_of(groups, first, last, level)?;
    let rows = rows_in(levels, &group);
    // An inner group that is **collapsed on its own** — every one of
    // its rows is hidden and its summary line is still there — keeps
    // its rows hidden.
    let collapsed: Vec<u64> = groups
        .iter()
        .filter(|other| {
            other.level > group.level
                && other.start >= group.start
                && other.end <= group.end
                && other.has_summary
                && other.summary != group.summary
        })
        .filter(|other| {
            let rows = rows_in(levels, other);
            !rows.is_empty() && rows.iter().all(|row| is_hidden(*row)) && !is_hidden(other.summary)
        })
        .flat_map(|other| rows_in(levels, other))
        .collect();
    Some((group, rows, collapsed))
}

/// The **levels** of the columns of a sheet, by their number.
fn column_numbers_of(node: &Node) -> BTreeMap<u64, u8> {
    column_levels_of(node).into_iter().map(|(column, level)| (column as u64, level)).collect()
}
