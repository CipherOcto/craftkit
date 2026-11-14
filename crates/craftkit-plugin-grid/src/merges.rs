//! The **merges** of a sheet.
//!
//! A merge says that a range of cells is **one cell**: the value lives
//! in the cell at the **top left** and the other cells of the range
//! are **covered**. The tree's merged region is one cell that is drawn
//! as many, and the tree's rule is that the value of the region is the
//! value of its top left cell.
//!
//! Here a merge is a **fact of the sheet**: the ranges that are
//! merged. And three rules hold it together:
//!
//! - a merge may not **overlap** another merge, because a cell that is
//!   in two regions is in none of them;
//! - a merge may not be made over cells that hold a **value**, because
//!   the value of a merged region is the value of its top left cell
//!   and the values of the others would be lost — so the merge is
//!   **refused** and the refusal says which cell holds a value;
//! - a **covered** cell may not be **written** to: it is not a cell
//!   anybody may put a value in, and a value put in one would be a
//!   value in a region that has one value.
//!
//! # What the port found
//!
//! - **The tree's merge is a command on a range and so is this one.**
//!   The tree merges the range that is chosen and unmerges it again.
//!   Here the range is given to `grid.merge.set` and `grid.merge.clear`
//!   by its own text, and the range of a merge is exactly the range
//!   that was merged — `A1:B2` and not `A1:B3`.
//! - **The tree keeps the value of the region in the top left cell and
//!   loses the others.** The tree's merge asks and moves the value
//!   that is in the other cells into the top left, or asks the user
//!   whether it may. Here the merge is **refused** when the other
//!   cells hold a value: the caller empties them first, and nothing
//!   is lost quietly.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

use crate::formula::{Anchor, Area, CellRef};

/// The **merges** of a sheet, as the areas they cover.
pub fn merges_of(node: &Node) -> Vec<Area> {
    let mut areas = Vec::new();
    let Some(list) = node.data.get("merges").and_then(|merges| merges.as_array()) else {
        return areas;
    };
    for merge in list {
        let Some(text) = merge.as_str() else { continue };
        let Some((_, area)) = parse_area(text) else { continue };
        areas.push(area);
    }
    areas
}

/// The **area** a range names, if it names one: `A1:B4` or `A1`.
pub fn parse_area(text: &str) -> Option<(String, Area)> {
    let trimmed = text.trim();
    let area = match trimmed.split_once(':') {
        Some((first, second)) => {
            let first = Anchor::parse(first)?;
            let second = Anchor::parse(second)?;
            Area::new(first.cell, second.cell)
        }
        None => {
            let only = Anchor::parse(trimmed)?;
            Area::new(only.cell, only.cell)
        }
    };
    Some((trimmed.to_string(), area))
}

/// The **merge** that covers a cell, if one does.
///
/// A cell is covered when it is in a merged region and it is not the
/// cell at the top left of that region.
pub fn covering(merges: &[Area], cell: CellRef) -> Option<Area> {
    merges.iter().find(|area| area.covers(cell) && area.start != cell).copied()
}

/// Refuse a **write** into a covered cell.
///
/// The refusal says which merge covers it.
pub fn refuse_covered(node: &Node, cells: &[CellRef], id: &str) -> Result<(), EngineError> {
    let merges = merges_of(node);
    for cell in cells {
        if let Some(area) = covering(&merges, *cell) {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "the cell {} is covered by the merge {}:{}, and the value of a merged region is the value of its top left cell",
                    cell.to_text(),
                    area.start.to_text(),
                    area.end.to_text()
                ),
            });
        }
    }
    Ok(())
}

/// The **cells** of an area, in order.
fn cells_of(area: &Area) -> Vec<CellRef> {
    let mut cells = Vec::new();
    for row in area.start.row..=area.end.row {
        for col in area.start.col..=area.end.col {
            cells.push(CellRef { col, row });
        }
    }
    cells
}

commands! {
    /// **Merge** a range of cells.
    ///
    /// The range becomes one cell: its value is the value of the cell at
    /// the top left, and the other cells are covered.
    grid.merge.set {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range that is merged: `A1:B2`.
            range: Text,
        },
        run: {
            let id = "grid.merge.set".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let text = self.range.0.trim().to_string();
            let Some((_, area)) = parse_area(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{text}` is not a range of cells"),
                });
            };
            if area.start == area.end {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a merge of one cell is not a merge: {text}"),
                });
            }
            let merges = merges_of(node);
            for merge in &merges {
                if areas_touch(merge, &area) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the range {text} touches the merge {}:{}, and a cell that is in two regions is in none of them",
                            merge.start.to_text(),
                            merge.end.to_text()
                        ),
                    });
                }
            }
            // The **cells** of the region but the top left one: they hold
            // nothing, or the merge is refused.
            let cells = cells_of(&area);
            let values = crate::cells_of(node);
            for cell in cells.iter().skip(1) {
                if let Some(entry) = values.get(*cell) {
                    let holds = entry.formula.is_some()
                        || matches!(&entry.value, Some(value) if value != &crate::formula::Value::Empty);
                    if holds {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the cell {} holds a value, and the value of a merged region is the value of its top left cell",
                                cell.to_text()
                            ),
                        });
                    }
                }
            }
            let mut list: Vec<String> =
                merges.iter().map(|area| format!("{}:{}", area.start.to_text(), area.end.to_text())).collect();
            list.push(format!("{}:{}", area.start.to_text(), area.end.to_text()));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["merges"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {text} is merged")).with_data(json!({
                "sheet": sheet,
                "range": text,
                "start": area.start.to_text(),
                "end": area.end.to_text(),
            })))

        }
    }

    /// **Unmerge** a range of cells.
    ///
    /// The range must be exactly a range that was merged.
    grid.merge.clear {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range that is unmerged: `A1:B2`.
            range: Text,
        },
        run: {
            let id = "grid.merge.clear".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let text = self.range.0.trim().to_string();
            let Some((_, area)) = parse_area(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{text}` is not a range of cells"),
                });
            };
            let merges = merges_of(node);
            let Some(merge) = merges.iter().find(|merge| merge.start == area.start && merge.end == area.end)
            else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no merge of {text} on the sheet `{sheet}`"),
                });
            };
            let list: Vec<String> = merges
                .iter()
                .filter(|other| other.start != merge.start || other.end != merge.end)
                .map(|area| format!("{}:{}", area.start.to_text(), area.end.to_text()))
                .collect();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["merges"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {text} is unmerged")).with_data(json!({
                "sheet": sheet,
                "range": text,
            })))

        }
    }

    /// **Merge** a range of cells **across** its rows.
    ///
    /// The tree's Merge Across: every row of the range is merged on its
    /// own, so a range of three rows and two columns becomes three merges
    /// of two cells each — and not one merge of six cells.
    ///
    /// The rule of a merge here is the rule of every merge here: the
    /// cells of the row but the first may not hold a value, and the merge
    /// is refused when one of them does.
    grid.merge.across {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range that is merged across: `A1:B3`.
            range: Text,
        },
        run: {
            let id = "grid.merge.across".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let text = self.range.0.trim().to_string();
            let Some((_, area)) = parse_area(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{text}` is not a range of cells"),
                });
            };
            if area.start.col == area.end.col {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a merge across needs a range of more than one column, and {text} has one"),
                });
            }
            // The **rows** of the range, each one a merge of its own.
            let mut rows: Vec<Area> = Vec::new();
            for row in area.start.row..=area.end.row {
                rows.push(Area::new(CellRef { col: area.start.col, row }, CellRef { col: area.end.col, row }));
            }
            let merges = merges_of(node);
            for row in &rows {
                for merge in &merges {
                    if areas_touch(merge, row) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the range {text} touches the merge {}:{}, and a cell that is in two regions is in none of them",
                                merge.start.to_text(),
                                merge.end.to_text()
                            ),
                        });
                    }
                }
            }
            // The **cells** of every row but the first one: they hold
            // nothing, or the merge is refused.
            let cells = crate::cells_of(node);
            for row in &rows {
                for cell in cells_of(row).iter().skip(1) {
                    let Some(entry) = cells.get(*cell) else { continue };
                    let holds = entry.formula.is_some()
                        || matches!(&entry.value, Some(value) if value != &crate::formula::Value::Empty);
                    if holds {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the cell {} holds a value, and the value of a merged region is the value of its top left cell",
                                cell.to_text()
                            ),
                        });
                    }
                }
            }
            let mut list: Vec<String> =
                merges.iter().map(|area| format!("{}:{}", area.start.to_text(), area.end.to_text())).collect();
            for row in &rows {
                list.push(format!("{}:{}", row.start.to_text(), row.end.to_text()));
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["merges"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {text} is merged across its rows")).with_data(json!({
                "sheet": sheet,
                "range": text,
                "merges": rows.iter().map(|row| format!("{}:{}", row.start.to_text(), row.end.to_text())).collect::<Vec<_>>(),
            })))

        }
    }

    /// **Merge** a range of cells and **centre** its text.
    ///
    /// The tree's Merge and Center: the range becomes one cell and its text
    /// is centred.
    ///
    /// The tree's own rule: a range that is **already merged** is
    /// unmerged again — the button is a button and a button pressed twice
    /// does the opposite.
    grid.merge.center {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range that is merged: `A1:B2`.
            range: Text,
        },
        run: {
            let id = "grid.merge.center".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let text = self.range.0.trim().to_string();
            let Some((_, area)) = parse_area(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{text}` is not a range of cells"),
                });
            };
            if area.start == area.end {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a merge of one cell is not a merge: {text}"),
                });
            }
            let merges = merges_of(node);
            let merged = merges.iter().any(|merge| merge.start == area.start && merge.end == area.end);
            if merged {
                // The range is already merged: the button pressed twice
                // unmerges it — the tree's own rule.
                let list: Vec<String> = merges
                    .iter()
                    .filter(|other| other.start != area.start || other.end != area.end)
                    .map(|area| format!("{}:{}", area.start.to_text(), area.end.to_text()))
                    .collect();
                if let Some(node) = session.document_mut()?.node_mut(sheet) {
                    node.data["merges"] = json!(list);
                }
                return Ok(Outcome::changed(format!("the range {text} is unmerged")).with_data(json!({
                    "sheet": sheet,
                    "range": text,
                    "merged": false,
                })));
            }
            for merge in &merges {
                if areas_touch(merge, &area) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the range {text} touches the merge {}:{}, and a cell that is in two regions is in none of them",
                            merge.start.to_text(),
                            merge.end.to_text()
                        ),
                    });
                }
            }
            // The **cells** of the region but the top left one: they hold
            // nothing, or the merge is refused.
            let cells = crate::cells_of(node);
            for cell in cells_of(&area).iter().skip(1) {
                let Some(entry) = cells.get(*cell) else { continue };
                let holds = entry.formula.is_some()
                    || matches!(&entry.value, Some(value) if value != &crate::formula::Value::Empty);
                if holds {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the cell {} holds a value, and the value of a merged region is the value of its top left cell",
                            cell.to_text()
                        ),
                    });
                }
            }
            let mut list: Vec<String> =
                merges.iter().map(|area| format!("{}:{}", area.start.to_text(), area.end.to_text())).collect();
            list.push(format!("{}:{}", area.start.to_text(), area.end.to_text()));
            // The text of the region is the text of its top left cell, and
            // it is **centred**: the tree's own rule.
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                let mut cells = crate::sheet_cells_mut(node);
                let mut align = cells.align(area.start).copied().unwrap_or_else(crate::formula::Align::plain);
                align.horizontal = crate::formula::AlignH::Center;
                cells.set_align(area.start, Some(align));
                node.data["merges"] = json!(list);
                node.data["cells"] = crate::cells_into(&cells);
            }
            Ok(Outcome::changed(format!("the range {text} is merged and its text is centred")).with_data(json!({
                "sheet": sheet,
                "range": text,
                "merged": true,
            })))

        }
    }
}

/// Whether two areas **touch**: they share a cell.
pub(crate) fn areas_touch(one: &Area, other: &Area) -> bool {
    let rows = one.start.row.max(other.start.row)..=one.end.row.min(other.end.row);
    let columns = one.start.col.max(other.start.col)..=one.end.col.min(other.end.col);
    rows.start() <= rows.end() && columns.start() <= columns.end()
}
