//! The **sparkline** of a cell.
//!
//! A sparkline is a **little picture of a range**, drawn in a cell: a
//! line, a set of columns or a row of wins and losses. The range is
//! the fact — the sparkline does not hold numbers of its own — and
//! the values it shows are **derived** from the cells of the sheet,
//! so a sparkline cannot be out of date with the sheet it reads.
//!
//! The rules:
//!
//! - a **kind** that is not one of the three — `line`, `column` and
//!   `win_loss` — is refused;
//! - a **range** that is not a range is refused;
//! - a cell that holds **no number** shows **nothing**: the value is
//!   not a zero, and a win and a loss are about the sign of a number
//!   that is there;
//! - a cell may hold a value **and** a sparkline: they are two facts,
//!   and the tree keeps them apart as well.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Value};

/// The **kinds** of sparkline there are.
pub const KINDS: [&str; 3] = ["line", "column", "win_loss"];

/// The kind of sparkline, from its name.
pub fn kind_of(text: &str) -> Option<&'static str> {
    let wanted = text.trim().to_ascii_lowercase();
    KINDS.iter().find(|kind| **kind == wanted).copied()
}

/// The **sparklines** of a sheet: the cell and what it holds.
pub fn sparklines_of(sheet: &Node) -> Vec<(String, Json)> {
    let Some(map) = sheet.data.get("sparklines").and_then(|sparklines| sparklines.as_object()) else {
        return Vec::new();
    };
    map.iter().map(|(cell, sparkline)| (cell.clone(), sparkline.clone())).collect()
}

/// The **sparkline** of a cell, if it has one.
pub fn sparkline_of(sheet: &Node, cell: &str) -> Option<Json> {
    sheet.data.get("sparklines").and_then(|sparklines| sparklines.get(cell)).cloned()
}

/// The **values** of a range: the numbers of its cells, and nothing
/// for a cell that holds no number.
pub fn values_of(sheet: &Node, area: &Area) -> Vec<Option<f64>> {
    let cells = crate::cells_of(sheet);
    (area.start.row..=area.end.row)
        .flat_map(|row| (area.start.col..=area.end.col).map(move |col| (row, col)))
        .map(|(row, col)| match cells.get(CellRef { col, row }).and_then(|cell| cell.value.clone()) {
            Some(Value::Number(number)) => Some(number),
            _ => None,
        })
        .collect()
}

/// The **signs** of a range: what a win and a loss are.
///
/// A number above zero is a **win**, a number below zero is a
/// **loss**, and a number that is nothing at all — or a number that is
/// exactly zero — is **nothing**: the tree draws no bar for it.
pub fn signs_of(values: &[Option<f64>]) -> Vec<Option<&'static str>> {
    values
        .iter()
        .map(|value| match value {
            Some(number) if *number > 0.0 => Some("win"),
            Some(number) if *number < 0.0 => Some("loss"),
            _ => None,
        })
        .collect()
}

/// The **sheet** a command is about, and whether it is a sheet.
fn sheet_of(session: &Session, id: &str, sheet: u64) -> Result<Node, EngineError> {
    let Some(node) = session.document().node(sheet).cloned() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
        });
    }
    Ok(node)
}

/// The **area** a range names, and whether it is one.
fn area_of(text: &str, id: &str) -> Result<Area, EngineError> {
    let area = crate::fill::area_at(text.trim(), id)?;
    if area.start.row == area.end.row && area.start.col == area.end.col {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the range {text} is one cell, and a sparkline reads a range"),
        });
    }
    Ok(area)
}

commands! {
    /// **Set** the sparkline of a cell.
    ///
    /// The cell shows a line, a set of columns or a row of wins and
    /// losses, taken from the range.
    grid.sparkline.set {
        area: "grid",
        layer: "grid",
        icon: "chart",
        mutates,
        undoable,
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The cell the sparkline is drawn in.
            cell: Text,
            /// The range it reads.
            range: Text,
            /// The kind: `line`, `column` or `win_loss`.
            kind: Option<Text>,
            /// The colour of the sparkline, as `#rrggbb`.
            colour: Option<Text>,
            /// Whether the **markers** are drawn.
            markers: Option<Bool>,
        },
        run: {
            let id = "grid.sparkline.set".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, &id, sheet)?;
            let cell = crate::cell_at(self.cell.0.trim(), &id)?.to_text();
            let area = area_of(&self.range.0, &id)?;
            let kind = match &self.kind {
                Some(kind) => match kind_of(&kind.0) {
                    Some(kind) => kind.to_string(),
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "`{}` is not a kind of sparkline: it is one of {}",
                                kind.0.trim(),
                                KINDS.join(", ")
                            ),
                        });
                    }
                },
                None => "line".to_string(),
            };
            let colour = match &self.colour {
                Some(colour) => {
                    let text = colour.0.trim();
                    match craftkit_sdk::Colour::from_hex(text) {
                        Ok(colour) => Some(colour),
                        Err(why) => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{text}` is not a colour: {why}"),
                            });
                        }
                    }
                }
                None => None,
            };
            let markers = self.markers.map(|markers| markers.0).unwrap_or(false);
            let sparkline = json!({
                "range": area.to_text(),
                "kind": kind.clone(),
                "colour": match &colour {
                    Some(colour) => json!([colour.0[0], colour.0[1], colour.0[2], colour.0[3]]),
                    None => Json::Null,
                },
                "markers": markers,
            });
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                if !node.data["sparklines"].is_object() {
                    node.data["sparklines"] = json!({});
                }
                if let Some(map) = node.data["sparklines"].as_object_mut() {
                    map.insert(cell.clone(), sparkline.clone());
                }
            }
            Ok(Outcome::changed(format!("the cell `{cell}` holds a sparkline of {}", area.to_text()))
                .with_data(json!({ "sheet": sheet, "cell": cell, "sparkline": sparkline })))

        }
    }

    /// **Read** the sparkline of a cell: what it reads and what it shows.
    ///
    /// Without a cell, every sparkline of the sheet is read.
    ///
    /// The values are **derived**: they are worked out from the cells of
    /// the sheet every time they are asked for.
    grid.sparkline.read {
        area: "grid",
        layer: "grid",
        icon: "chart",
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The cell. Without it, every sparkline of the sheet is read.
            cell: Option<Text>,
        },
        run: {
            let id = "grid.sparkline.read".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let read = |cell: &str, sparkline: &Json| -> Result<Json, EngineError> {
                let range = sparkline.get("range").and_then(|range| range.as_str()).unwrap_or_default();
                let area = crate::fill::area_at(range, &id)?;
                let values = values_of(&node, &area);
                let kind = sparkline.get("kind").and_then(|kind| kind.as_str()).unwrap_or("line").to_string();
                let mut derived = json!({ "values": values });
                if kind == "win_loss" {
                    derived["signs"] = json!(signs_of(&values));
                }
                Ok(json!({ "cell": cell, "sparkline": sparkline, "data": derived }))
            };
            match &self.cell {
                Some(cell) => {
                    let cell = crate::cell_at(cell.0.trim(), &id)?.to_text();
                    let Some(sparkline) = sparkline_of(&node, &cell) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the cell `{cell}` holds no sparkline"),
                        });
                    };
                    let read = read(&cell, &sparkline)?;
                    let mut data = json!({ "sheet": sheet });
                    if let Some(map) = data.as_object_mut()
                        && let Some(read) = read.as_object()
                    {
                        for (key, value) in read {
                            map.insert(key.clone(), value.clone());
                        }
                    }
                    Ok(Outcome::new(format!("the cell `{cell}` holds a sparkline")).with_data(data))
                }
                None => {
                    let mut sparklines: Vec<Json> = Vec::new();
                    for (cell, sparkline) in sparklines_of(&node) {
                        sparklines.push(read(&cell, &sparkline)?);
                    }
                    Ok(Outcome::new(format!("the sheet holds {} sparkline(s)", sparklines.len()))
                        .with_data(json!({ "sheet": sheet, "sparklines": sparklines })))
                }
            }

        }
    }

    /// **Take** the sparkline of a cell away.
    grid.sparkline.remove {
        area: "grid",
        layer: "grid",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The cell.
            cell: Text,
        },
        run: {
            let id = "grid.sparkline.remove".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let cell = crate::cell_at(self.cell.0.trim(), &id)?.to_text();
            if sparkline_of(&node, &cell).is_none() {
                return Err(EngineError::Failed { id, message: format!("the cell `{cell}` holds no sparkline") });
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet)
                && let Some(map) = node.data["sparklines"].as_object_mut()
            {
                map.remove(&cell);
            }
            Ok(Outcome::changed(format!("the cell `{cell}` holds no sparkline"))
                .with_data(json!({ "sheet": sheet, "cell": cell })))

        }
    }
}
