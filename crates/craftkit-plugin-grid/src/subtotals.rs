//! The **subtotal** of a group of rows.
//!
//! The tree's own command: the rows of a range are grouped by the
//! value of a column, and a total row goes under each group of them.
//!
//! # The rules
//!
//! - The **groups** are the rows that share the value of the group
//!   column and that follow one another. The tree's own rule. The
//!   first row of the range is a **header**, unless it is said not to
//!   be one.
//! - A **total row** goes under each group: the name of the group and
//!   the word of the way — "East Total" — in the group column, and the
//!   formula `=SUBTOTAL(9,B2:B3)` in each of the columns that are
//!   totalled. The tree's own rule.
//! - The **grand total** goes at the end: the name "Grand Total" and a
//!   formula over every row. A row that holds a subtotal is left out
//!   of it, so the grand total is the total of the detail. The tree's
//!   own rule.
//! - The totals go **below** their detail, and the summary of the
//!   sheet goes below as well. The tree's own rule.
//! - A way of totalling that is **not one of the tree's** is refused
//!   by name, and so is a range that is not a range and a column that
//!   is not a column.
//!
//! # What it does not do
//!
//! - **The total row is not bold**: the tree bolds it, and the styles
//!   of a cell are another thing.
//! - **The rows are not hidden**: the tree's groups are an outline and
//!   the outline is here, but the rows are not folded away.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Text};

/// The **ways** the rows may be totalled in: the tree's own list, with
/// the **function number** of the subtotal and the **word** the total
/// row is labelled with.
const WAYS: [(&str, u64, &str); 6] = [
    ("count", 3, "Count"),
    ("average", 1, "Average"),
    ("max", 4, "Max"),
    ("min", 5, "Min"),
    ("product", 6, "Product"),
    ("sum", 9, "Total"),
];

/// The **area** a range names.
fn area_of(range: &str) -> Option<crate::formula::Area> {
    let trimmed = range.trim();
    let area = match trimmed.split_once(':') {
        Some((first, second)) => {
            let first = crate::formula::Anchor::parse(first)?;
            let second = crate::formula::Anchor::parse(second)?;
            crate::formula::Area::new(first.cell, second.cell)
        }
        None => {
            let only = crate::formula::Anchor::parse(trimmed)?;
            crate::formula::Area::new(only.cell, only.cell)
        }
    };
    Some(area)
}

/// The **column** a name says: the letters of a column, or the column
/// of an address.
fn column_of(name: &str) -> Option<u32> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(anchor) = crate::formula::Anchor::parse(trimmed) {
        return Some(anchor.cell.col);
    }
    if !trimmed.chars().all(|letter| letter.is_ascii_alphabetic()) {
        return None;
    }
    let mut col: u32 = 0;
    for letter in trimmed.chars() {
        col = col.checked_mul(26)?;
        col = col.checked_add(u32::from(letter.to_ascii_uppercase() as u8 - b'A'))?;
    }
    if u64::from(col) >= crate::formula::MAX_COLS {
        return None;
    }
    Some(col)
}

/// The **text** of a value, as a cell shows it.
fn display_of(value: &crate::formula::Value) -> String {
    match value {
        crate::formula::Value::Empty => String::new(),
        crate::formula::Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        crate::formula::Value::Text(text) => text.clone(),
        crate::formula::Value::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        crate::formula::Value::Error(message) => message.clone(),
    }
}

/// Put a **text** in a cell.
fn set_text(sheet: &mut Node, cell: crate::formula::CellRef, text: &str) {
    let mut cells = crate::sheet_cells_mut(sheet);
    cells.set_value(cell, crate::formula::Value::Text(text.to_string()));
    sheet.data["cells"] = crate::cells_into(&cells);
}

/// Put a **formula** in a cell.
fn set_formula(sheet: &mut Node, cell: crate::formula::CellRef, formula: &str) {
    let mut cells = crate::sheet_cells_mut(sheet);
    cells.set_formula(cell, formula.to_string());
    sheet.data["cells"] = crate::cells_into(&cells);
}

/// Group the **rows** of a group of rows: the tree's own outline.
fn group_rows(sheet: &mut Node, first: u64, last: u64) {
    let mut levels = crate::outline::levels_of(sheet);
    for row in first..=last {
        let level = levels.get(&row).copied().unwrap_or(0);
        levels.insert(row, level + 1);
    }
    sheet.data["row_outline"] = crate::outline::levels_into(&levels);
}

commands! {
    /// The **subtotal** of the groups of rows.
    ///
    /// A change: a total row goes under each group of the rows, and a
    /// grand total goes at the end.
    grid.data.subtotal {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the rows are in.

            sheet: NodeRef,
            /// The range of the rows: `A1:B4`.

            range: Text,
            /// The column the rows are grouped by: `A`.

            group_by: Text,
            /// The way the rows are totalled in.
            ///
            /// `sum` — the default — `count`, `average`, `max`, `min` or
            /// `product`.

            function: Option<Text>,
            /// The columns that are totalled.
            ///
            /// Without it, the last column of the range.

            columns: Option<Json>,
            /// Whether the first row of the range is a **header**.
            ///
            /// It is, without it being said.

            header: Option<Bool>,
        },
        run: {

            let id = "grid.data.subtotal".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
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
            if crate::protection::Protection::of(&node).is_some_and(|protection| protection.locked) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the sheet `{sheet}` is protected, and the cells of a protected sheet may not be changed"
                    ),
                });
            }
            let Some(area) = area_of(&self.range.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a range of cells", self.range.0.trim()),
                });
            };
            let Some(group_col) = column_of(&self.group_by.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a column", self.group_by.0.trim()),
                });
            };
            let way = self
                .function
                .as_ref()
                .map(|function| function.0.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "sum".to_string());
            let Some((_, code, label)) = WAYS.iter().find(|(name, _, _)| *name == way) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the way `{way}` is not one of this tree's: it is one of count, average, max, min, product, sum"
                    ),
                });
            };
            let columns: Vec<u32> = match &self.columns {
                Some(columns) => {
                    let Some(columns) = columns.as_array() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the columns that are totalled are a list of columns".to_string(),
                        });
                    };
                    let mut held: Vec<u32> = Vec::new();
                    for column in columns {
                        let Some(text) = column.as_str() else {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: "the columns that are totalled are a list of columns".to_string(),
                            });
                        };
                        let Some(col) = column_of(text) else {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{text}` is not a column"),
                            });
                        };
                        held.push(col);
                    }
                    held
                }
                None => vec![area.end.col],
            };
            let header = self.header.map(|header| header.0).unwrap_or(true);
            let name = node.name.clone();
            // The **groups**: the rows that share the value of the group
            // column and that follow one another.
            let book = crate::book_of(session.document());
            let mut groups: Vec<(u32, u32, String)> = Vec::new();
            let first = area.start.row + u32::from(header);
            if first <= area.end.row {
                let mut start = first;
                let mut group =
                    display_of(&book.value_of(&name, crate::formula::CellRef { col: group_col, row: start }));
                for row in (first + 1)..=area.end.row {
                    let value =
                        display_of(&book.value_of(&name, crate::formula::CellRef { col: group_col, row }));
                    if value != group {
                        groups.push((start, row - 1, group.clone()));
                        start = row;
                        group = value;
                    }
                }
                groups.push((start, area.end.row, group));
            }
            // The totals go **below** their detail, and so does the summary
            // of the sheet: the tree's own rule.
            let mut sheet_node = node;
            if sheet_node.data.get("summary_below").and_then(|below| below.as_bool()) != Some(true) {
                sheet_node.data["summary_below"] = json!(true);
            }
            // The **total rows**, from the bottom up so that the rows above
            // them stay where they are.
            let mut inserted: u64 = 0;
            for (first, last, group) in groups.iter().rev() {
                let at = u64::from(*last) + 2;
                crate::inserting::insert_rows(&mut sheet_node, &name, at, 1)
                    .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
                let row = *last + 1;
                set_text(
                    &mut sheet_node,
                    crate::formula::CellRef { col: group_col, row },
                    &format!("{group} {label}"),
                );
                for col in &columns {
                    let range = format!(
                        "{}:{}",
                        crate::formula::CellRef { col: *col, row: *first }.to_text(),
                        crate::formula::CellRef { col: *col, row: *last }.to_text()
                    );
                    set_formula(
                        &mut sheet_node,
                        crate::formula::CellRef { col: *col, row },
                        &format!("=SUBTOTAL({code},{range})"),
                    );
                }
                group_rows(&mut sheet_node, u64::from(*first) + 1, u64::from(*last) + 1);
                inserted += 1;
            }
            // The **grand total**, at the end.
            let grand = area.end.row + inserted as u32 + 1;
            crate::inserting::insert_rows(&mut sheet_node, &name, u64::from(grand) + 1, 1)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            set_text(
                &mut sheet_node,
                crate::formula::CellRef { col: group_col, row: grand },
                &format!("Grand {label}"),
            );
            for col in &columns {
                let range = format!(
                    "{}:{}",
                    crate::formula::CellRef { col: *col, row: area.start.row + u32::from(header) }.to_text(),
                    crate::formula::CellRef { col: *col, row: grand - 1 }.to_text()
                );
                set_formula(
                    &mut sheet_node,
                    crate::formula::CellRef { col: *col, row: grand },
                    &format!("=SUBTOTAL({code},{range})"),
                );
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data = sheet_node.data.clone();
            }
            Ok(Outcome::changed(format!("{} group(s) of rows are totalled", groups.len())).with_data(json!({
                "sheet": sheet,
                "groups": groups.len(),
                "rows": inserted + 1,
                "way": way,
                "code": code,
            })))

        }
    }
}
