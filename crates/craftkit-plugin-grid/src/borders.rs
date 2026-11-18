//! The **border** of a cell.
//!
//! A cell may carry a **border**: a line on one of its edges, of a
//! style, in a colour. The tree's border is part of the cell's
//! **style** — a font, a fill, a border, an alignment and a number
//! format — and here the border is a fact of the sheet, kept beside
//! the cells for the same reason the notes are (§ the note module):
//! a cell here is a value, a formula, a format and a style, and a
//! border is none of those four.
//!
//! The rules:
//!
//! - the **edges** of a cell are the four the tree has — `top`,
//!   `right`, `bottom` and `left` — and an edge that is not one of
//!   them is **refused**, and so is a style that is not one of the
//!   tree's;
//! - the **colour** is a colour of this tree: `#rrggbb`, and
//!   nothing else. A border with no colour said has the **black** one,
//!   and that is said here rather than left to a reader;
//! - a border of the style **none** is no border: it takes the border
//!   away, and an edge that has no border is not kept;
//! - a cell that is **covered by a merge** is refused: the border
//!   would be drawn under another cell's;
//! - the **formatting** of a protected sheet that does not allow the
//!   formatting is refused, and the border is the formatting.
//!
//! And a **range** of cells may be given a border as well: the
//! **preset** says which edges of which cells get the line — `all`,
//! `outside`, `inside`, or one of the four edges on its own.
//!
//! And the border is **taken away** with the formatting it is: the
//! tree's *clear formats* clears the style, and the border is part of
//! it, and the tree's *clear all* clears the style too.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **edges** a cell has.
const EDGES: [&str; 4] = ["top", "right", "bottom", "left"];

/// The **styles** a border may have — the tree's own.
const STYLES: [&str; 13] = [
    "thin",
    "medium",
    "thick",
    "dashed",
    "dotted",
    "double",
    "hair",
    "mediumDashed",
    "dashDot",
    "mediumDashDot",
    "dashDotDot",
    "mediumDashDotDot",
    "slantDashDot",
];

/// The **colour** a border has when none is said.
const DEFAULT_COLOUR: &str = "#000000";

/// A **border**: the style of the line and the colour it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Border {
    /// The style of the line.
    pub style: String,
    /// The colour of the line, as `#rrggbb`.
    pub colour: String,
}

/// The **borders** of a sheet: by the address of the cell they are on.
pub fn borders_of(node: &Node) -> BTreeMap<String, BTreeMap<String, Border>> {
    match node.data.get("borders") {
        Some(borders) => borders_from(borders),
        None => BTreeMap::new(),
    }
}

/// The **borders** a value holds: by the address of the cell they are
/// on.
pub fn borders_from(value: &Json) -> BTreeMap<String, BTreeMap<String, Border>> {
    let mut borders = BTreeMap::new();
    let Some(map) = value.as_object() else {
        return borders;
    };
    for (address, edges) in map {
        let Some(edges) = edges.as_object() else { continue };
        let mut one = BTreeMap::new();
        for (edge, border) in edges {
            let Some(style) = border.get("style").and_then(|style| style.as_str()) else { continue };
            let Some(colour) = border.get("colour").and_then(|colour| colour.as_str()) else { continue };
            one.insert(edge.to_string(), Border { style: style.to_string(), colour: colour.to_string() });
        }
        if !one.is_empty() {
            borders.insert(address.to_string(), one);
        }
    }
    borders
}

/// The borders of a sheet, as they are kept.
pub fn borders_into(borders: &BTreeMap<String, BTreeMap<String, Border>>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (address, edges) in borders {
        let mut one = craftkit_sdk::serde_json::Map::new();
        for (edge, border) in edges {
            let mut line = craftkit_sdk::serde_json::Map::new();
            line.insert("style".to_string(), Json::String(border.style.clone()));
            line.insert("colour".to_string(), Json::String(border.colour.clone()));
            one.insert(edge.clone(), Json::Object(line));
        }
        map.insert(address.clone(), Json::Object(one));
    }
    Json::Object(map)
}

/// The **borders** a **rule** names, when they are borders this tree
/// has.
///
/// A rule's border names the **edges** it draws and the **line** of each
/// of them: the style of the line and, without it, the colour of the
/// line is the tree's own black.
///
/// An edge this tree does not have, a style it does not have and a
/// colour that is not a colour are refused: a rule that draws a line
/// the tree cannot draw is a rule the tree cannot keep.
pub fn borders_of_rule(value: &Json, id: &str) -> Result<BTreeMap<String, Border>, EngineError> {
    let Some(map) = value.as_object() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a rule's border names the edges it draws: `{\"top\": {\"style\": \"thin\"}}`"
                .to_string(),
        });
    };
    let mut out = BTreeMap::new();
    for (edge, line) in map {
        let Some(edge) = edge_of(edge) else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "`{edge}` is not an edge of a cell: the edges are `top`, `right`, `bottom` and `left`"
                ),
            });
        };
        let Some(given) = line.get("style").and_then(|style| style.as_str()) else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the border of the `{edge}` edge names no style of a line"),
            });
        };
        let Some(style) = style_of(given) else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("`{given}` is not a style of a line"),
            });
        };
        let colour = match line.get("colour").and_then(|colour| colour.as_str()) {
            Some(given) => match colour_of(given) {
                Some(colour) => colour,
                None => {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: format!("`{given}` is not a colour of a line"),
                    });
                }
            },
            None => DEFAULT_COLOUR.to_string(),
        };
        out.insert(edge.to_string(), Border { style: style.to_string(), colour });
    }
    Ok(out)
}

/// The **borders** a value holds: the edges they are on, by the edge.
///
/// The edges this tree does not have, and the lines it cannot draw, are
/// left out: a value that was written by something else is read as far
/// as it can be read.
pub fn edges_from(value: &Json) -> BTreeMap<String, Border> {
    let mut out = BTreeMap::new();
    let Some(map) = value.as_object() else { return out };
    for (edge, line) in map {
        let Some(style) = line.get("style").and_then(|style| style.as_str()) else { continue };
        let Some(colour) = line.get("colour").and_then(|colour| colour.as_str()) else { continue };
        out.insert(edge.to_string(), Border { style: style.to_string(), colour: colour.to_string() });
    }
    out
}

/// The borders of a rule, as they are kept.
pub fn edges_into(borders: &BTreeMap<String, Border>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (edge, border) in borders {
        let mut line = craftkit_sdk::serde_json::Map::new();
        line.insert("style".to_string(), json!(border.style));
        line.insert("colour".to_string(), json!(border.colour));
        map.insert(edge.to_string(), Json::Object(line));
    }
    Json::Object(map)
}

/// The **borders of one cell**, if it has any.
pub fn borders_of_cell(node: &Node, cell: &str) -> BTreeMap<String, Border> {
    borders_of(node).remove(cell).unwrap_or_default()
}

/// Take the **borders of a cell** away, and say whether it had any.
pub fn take_borders(node: &mut Node, cell: &str) -> bool {
    let mut borders = borders_of(node);
    let had = borders.remove(cell).is_some();
    node.data["borders"] = borders_into(&borders);
    had
}

/// The **edge** a name says, if it says one this tree has.
fn edge_of(given: &str) -> Option<&'static str> {
    EDGES.iter().find(|edge| edge.eq_ignore_ascii_case(given)).copied()
}

/// The **style** a name says, if it says one this tree has.
fn style_of(given: &str) -> Option<&'static str> {
    STYLES.iter().find(|style| style.eq_ignore_ascii_case(given)).copied()
}

/// The **colour** a name says, if it says one this tree has.
fn colour_of(given: &str) -> Option<String> {
    let text = given.trim();
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex.to_ascii_lowercase()))
}

/// The most **cells** a border of a range may reach.
///
/// The tree's own bound: a border is written on every cell of the
/// range, and a range of a million cells is a million lines.
const MAX_BORDER_CELLS: u64 = 1_000_000;

/// The **presets** a border of a range has: which edges of which
/// cells get the line.
const PRESETS: [&str; 7] = ["all", "outside", "inside", "top", "bottom", "left", "right"];

/// The **preset** a name says, if it says one this tree has.
fn preset_of(given: &str) -> Option<&'static str> {
    PRESETS.iter().find(|preset| preset.eq_ignore_ascii_case(given)).copied()
}

/// The **edges** a preset gives to a cell of a range.
///
/// The edges are the tree's four: `top`, `right`, `bottom` and `left`.
/// A cell on the rim of the range has the edges that face out of it,
/// and a cell that is not on the rim has the edges that face in.
fn edges_of(preset: &str, area: &crate::formula::Area, cell: crate::formula::CellRef) -> Vec<&'static str> {
    let top = cell.row == area.start.row;
    let bottom = cell.row == area.end.row;
    let left = cell.col == area.start.col;
    let right = cell.col == area.end.col;
    let mut edges: Vec<&'static str> = Vec::new();
    match preset {
        "all" => edges.extend(["top", "right", "bottom", "left"]),
        "outside" => {
            if top {
                edges.push("top");
            }
            if right {
                edges.push("right");
            }
            if bottom {
                edges.push("bottom");
            }
            if left {
                edges.push("left");
            }
        }
        "inside" => {
            if !top {
                edges.push("top");
            }
            if !right {
                edges.push("right");
            }
            if !bottom {
                edges.push("bottom");
            }
            if !left {
                edges.push("left");
            }
        }
        "top" if top => edges.push("top"),
        "bottom" if bottom => edges.push("bottom"),
        "left" if left => edges.push("left"),
        "right" if right => edges.push("right"),
        _ => {}
    }
    edges
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
    /// **Set** the border of one edge of a cell.
    ///
    /// A border of the style `none` takes the border away — the tree's
    /// own: a line of no style is no line.
    grid.cell.border.set {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// The **edge**: `top`, `right`, `bottom` or `left`.
            edge: Text,
            /// The **style** of the line: `thin`, `medium`, `thick`, `dashed`,
            /// `dotted`, `double`, `hair`, `mediumDashed`, `dashDot`,
            /// `mediumDashDot`, `dashDotDot`, `mediumDashDotDot` or
            /// `slantDashDot` — and `none`, which takes the border away.
            style: Text,
            /// The **colour** of the line, as `#rrggbb`. Without it the line
            /// is black.
            colour: Option<Text>,
        },
        run: {
            let id = "grid.cell.border.set".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let Some(edge) = edge_of(self.edge.0.trim()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not an edge: a cell has four — top, right, bottom and left",
                        self.edge.0.trim()
                    ),
                });
            };
            let given = self.style.0.trim();
            let style = if given.eq_ignore_ascii_case("none") {
                None
            } else {
                match style_of(given) {
                    Some(style) => Some(style),
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "`{given}` is not a border style: a style is one of {}",
                                STYLES.join(", ")
                            ),
                        });
                    }
                }
            };
            let colour = match &self.colour {
                Some(colour) => match colour_of(&colour.0) {
                    Some(colour) => colour,
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("`{}` is not a colour: a colour is `#rrggbb`", colour.0.trim()),
                        });
                    }
                },
                None => DEFAULT_COLOUR.to_string(),
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            crate::merges::refuse_covered(&node, &[cell], &id)?;
            crate::protection::refuse_format(&node, &id)?;
            let address = cell.to_text();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                let mut borders = borders_of(node);
                let mut edges = borders.remove(&address).unwrap_or_default();
                match style {
                    Some(style) => {
                        edges.insert(
                            edge.to_string(),
                            Border { style: style.to_string(), colour: colour.clone() },
                        );
                    }
                    None => {
                        edges.remove(edge);
                    }
                }
                if edges.is_empty() {
                    borders.remove(&address);
                } else {
                    borders.insert(address.clone(), edges);
                }
                node.data["borders"] = borders_into(&borders);
            }
            match style {
                Some(style) => {
                    Ok(Outcome::changed(format!("the cell `{address}` has a {style} border on its {edge}"))
                        .with_data(json!({
                            "sheet": sheet,
                            "cell": address,
                            "edge": edge,
                            "style": style,
                            "colour": colour,
                        })))
                }
                None => Ok(Outcome::changed(format!("the cell `{address}` has no {edge} border"))
                    .with_data(json!({ "sheet": sheet, "cell": address, "edge": edge, "style": "none" }))),
            }

        }
    }

    /// **Read** the borders of a cell, or of the whole sheet.
    ///
    /// Without a cell, every border of the sheet is read.
    grid.cell.border.get {
        area: "grid",
        layer: "grid",
        icon: "grid",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address. Without it, every border of the sheet is
            /// read.
            cell: Option<Text>,
        },
        run: {
            let id = "grid.cell.border.get".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            match &self.cell {
                Some(cell) => {
                    let cell = crate::cell_at(&cell.0, &id)?;
                    let edges = borders_of_cell(node, &cell.to_text());
                    let mut map = craftkit_sdk::serde_json::Map::new();
                    for (edge, border) in &edges {
                        map.insert(
                            edge.clone(),
                            json!({ "style": border.style.clone(), "colour": border.colour.clone() }),
                        );
                    }
                    Ok(Outcome::new(format!("the cell {} has its borders", cell.to_text()))
                        .with_data(json!({ "cell": cell.to_text(), "borders": Json::Object(map) })))
                }
                None => {
                    let borders = borders_of(node);
                    let mut map = craftkit_sdk::serde_json::Map::new();
                    for (address, edges) in &borders {
                        let mut one = craftkit_sdk::serde_json::Map::new();
                        for (edge, border) in edges {
                            one.insert(
                                edge.clone(),
                                json!({ "style": border.style.clone(), "colour": border.colour.clone() }),
                            );
                        }
                        map.insert(address.clone(), Json::Object(one));
                    }
                    Ok(Outcome::new("the sheet has its borders").with_data(json!({
                        "borders": Json::Object(map),
                    })))
                }
            }

        }
    }

    /// **Take away** the border of one edge of a cell.
    ///
    /// The other edges of the cell stay.
    grid.cell.border.clear {
        area: "grid",
        layer: "grid",
        icon: "eraser",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// The **edge**: `top`, `right`, `bottom` or `left`.
            edge: Text,
        },
        run: {
            let id = "grid.cell.border.clear".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let Some(edge) = edge_of(self.edge.0.trim()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not an edge: a cell has four — top, right, bottom and left",
                        self.edge.0.trim()
                    ),
                });
            };
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            crate::merges::refuse_covered(&node, &[cell], &id)?;
            crate::protection::refuse_format(&node, &id)?;
            let address = cell.to_text();
            let mut had = false;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                let mut borders = borders_of(node);
                if let Some(edges) = borders.get_mut(&address) {
                    had = edges.remove(edge).is_some();
                    if edges.is_empty() {
                        borders.remove(&address);
                    }
                }
                node.data["borders"] = borders_into(&borders);
            }
            Ok(Outcome::changed(format!("the cell `{address}` has no {edge} border"))
                .with_data(json!({ "sheet": sheet, "cell": address, "edge": edge, "had": had })))

        }
    }

    /// **Give** a range of cells a border.
    ///
    /// The **preset** says which edges of which cells get the line:
    /// `all` gives every cell of the range all four of its edges,
    /// `outside` gives the cells on the rim the edges that face out of
    /// the range, `inside` gives the cells that are not on the rim the
    /// edges that face in, and `top`, `bottom`, `left` and `right` give
    /// one edge to the cells that are on that edge of the range.
    ///
    /// A border of the style `none` takes the border away — the tree's
    /// own: a line of no style is no line.
    grid.range.border {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet the cells are in.
            sheet: NodeRef,
            /// The **range** of cells: `A1`, `B2:D4`.
            range: Text,
            /// The **preset**: `all`, `outside`, `inside`, `top`, `bottom`,
            /// `left` or `right`.
            preset: Text,
            /// The **style** of the line: `thin`, `medium`, `thick`, `dashed`,
            /// `dotted`, `double`, `hair`, `mediumDashed`, `dashDot`,
            /// `mediumDashDot`, `dashDotDot`, `mediumDashDotDot` or
            /// `slantDashDot` — and `none`, which takes the border away.
            ///
            /// It is a thin line without it.
            style: Option<Text>,
            /// The **colour** of the line, as `#rrggbb`. Without it the line
            /// is black.
            colour: Option<Text>,
        },
        run: {
            let id = "grid.range.border".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, sheet, &id)?;
            let area = crate::fill::area_at(&self.range.0, &id)?;
            let Some(preset) = preset_of(self.preset.0.trim()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a preset: a border of a range is one of {}",
                        self.preset.0.trim(),
                        PRESETS.join(", ")
                    ),
                });
            };
            let style = match &self.style {
                None => Some("thin".to_string()),
                Some(text) => {
                    let given = text.0.trim();
                    if given.eq_ignore_ascii_case("none") {
                        None
                    } else {
                        match style_of(given) {
                            Some(style) => Some(style.to_string()),
                            None => {
                                return Err(EngineError::Failed {
                                    id: id.clone(),
                                    message: format!(
                                        "`{given}` is not a border style: a style is one of {}",
                                        STYLES.join(", ")
                                    ),
                                });
                            }
                        }
                    }
                }
            };
            let colour = match &self.colour {
                None => DEFAULT_COLOUR.to_string(),
                Some(text) => match colour_of(&text.0) {
                    Some(colour) => colour,
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("`{}` is not a colour: a colour is `#rrggbb`", text.0.trim()),
                        });
                    }
                },
            };
            if area.count() > MAX_BORDER_CELLS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the range is too large for a border (at most {MAX_BORDER_CELLS} cells)"),
                });
            }
            // The **cells** the border is written on.
            let mut targets: Vec<crate::formula::CellRef> = Vec::new();
            for row in area.start.row..=area.end.row {
                for col in area.start.col..=area.end.col {
                    targets.push(crate::formula::CellRef { col, row });
                }
            }
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            crate::merges::refuse_covered(&node, &targets, &id)?;
            crate::protection::refuse_format(&node, &id)?;
            let mut written = 0usize;
            let mut edges_written = 0usize;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                let mut borders = borders_of(node);
                for target in &targets {
                    let edges = edges_of(preset, &area, *target);
                    if edges.is_empty() {
                        continue;
                    }
                    let address = target.to_text();
                    let mut held = borders.remove(&address).unwrap_or_default();
                    for edge in edges {
                        match &style {
                            Some(style) => {
                                held.insert(
                                    edge.to_string(),
                                    Border { style: style.clone(), colour: colour.clone() },
                                );
                            }
                            None => {
                                held.remove(edge);
                            }
                        }
                        edges_written += 1;
                    }
                    if held.is_empty() {
                        borders.remove(&address);
                    } else {
                        borders.insert(address, held);
                    }
                    written += 1;
                }
                node.data["borders"] = borders_into(&borders);
            }
            Ok(Outcome::changed(format!("the cells of {} have a {preset} border", area.to_text()))
                .with_data(json!({
                    "sheet": sheet,
                    "range": area.to_text(),
                    "preset": preset,
                    "style": match &style {
                        Some(style) => json!(style),
                        None => json!("none"),
                    },
                    "colour": colour,
                    "cells": written,
                    "edges": edges_written,
                })))

        }
    }
}
