//! The **conditional formats** of a sheet.
//!
//! A conditional format is a **range** of cells, a **criterion** and a
//! **look**: the cells of the range whose value matches the criterion
//! are written with it. The look is a **number format** and a **fill**,
//! and each of them is a part of its own: a rule may carry the format
//! and no fill, and a rule may carry the fill and no format.
//!
//! The rules **layer**: every rule that matches is tried, and for each
//! part of the look the rule that is **seen first** and says something
//! about it wins — the tree's own rule, where two rules set the same
//! part of a cell the one seen first wins. And the tree's **stop if
//! true**: a rule that matched and says stop ends the walk, and the
//! rules after it are not tried.
//!
//! And the format a cell is **written with** is **derived**: the
//! conditional format that matches it wins over the cell's own format,
//! as it does in the tree and in Excel — the conditional format is a
//! rule about the cell and the cell's own format is a fact about the
//! cell, and the rule is on top.
//!
//! # What the port found
//!
//! - **The tree's rule is a style and this one's is a look.** The
//!   tree's conditional format carries a style — a fill, a font
//!   colour, an icon set — and the style is shared between cells. Here
//!   it carries a **number format** and a **fill**: the format is a
//!   cell's own thing (the tree's format is a style too, and this
//!   tree's format is the cell's own), and the fill is a cell's own
//!   thing too (§821). The tree's font colour and its icon set are
//!   still not here.
//! - **The tree's criteria are many and this one's is one.** The
//!   tree's rule may be a comparison, a colour scale, a data bar, an
//!   icon set or an expression. Here a rule is a **comparison** — the
//!   same criterion the filter of a sheet uses (§347) — and the other
//!   kinds are commands of their own when they come.
//! - **The tree's rules are ordered and the first that matches wins.**
//!   The tree's rules are in a list and it walks them in **priority** —
//!   the rule that names the lower number is seen first. Here they are
//!   in a list too, and the rule that names the **lower priority** is
//!   tried first; a rule that names no priority is tried after the
//!   ones that do, in the order the sheet holds it. The tree renumbers
//!   the priorities when a rule comes or goes; here the priority is
//!   **named** and it stays where it is.
//! - **The tree's rules layer and here they layer too.** The tree
//!   applies every rule that matches and where two of them set the
//!   same part of a cell the one seen first wins; here each part of
//!   the look is the part of the first rule that matched and said
//!   something about it. And the tree's **stop if true** — a rule
//!   that ends the walk when it matched — is here as a **stop**.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Criterion, FILTER_OPS};

/// A **conditional format** of a sheet.
pub struct Conditional {
    /// The range of cells it is about.
    pub range: String,
    /// The criterion a cell's value is judged by.
    pub criterion: Criterion,
    /// The **format code** the matching cells are written with, when
    /// the rule names one.
    pub format: Option<String>,
    /// The **fill** the matching cells are filled with, when the rule
    /// names one.
    pub fill: Option<String>,
    /// The **stop**: the tree's *stop if true*. A rule that matched
    /// and says stop ends the walk, and the rules after it are not
    /// tried.
    pub stop: Option<bool>,
    /// The **priority** of the rule, when it names one: the lower the
    /// number, the earlier the rule is tried.
    pub priority: Option<u32>,
}

/// The **conditional formats** of a sheet, in order.
pub fn conditionals_of(node: &craftkit_sdk::Node) -> Vec<Conditional> {
    let mut formats = Vec::new();
    let Some(list) = node.data.get("conditionals").and_then(|list| list.as_array()) else {
        return formats;
    };
    for entry in list {
        let Some(range) = entry.get("range").and_then(|range| range.as_str()) else { continue };
        let op = entry.get("op").and_then(|op| op.as_str()).unwrap_or("=").to_string();
        let value = entry.get("value").and_then(|value| value.as_str()).unwrap_or("").to_string();
        let format = entry.get("format").and_then(|format| format.as_str()).map(|format| format.to_string());
        let fill = entry.get("fill").and_then(|fill| fill.as_str()).map(|fill| fill.to_string());
        let stop = entry.get("stop").and_then(|stop| stop.as_bool());
        let priority = entry
            .get("priority")
            .and_then(|priority| priority.as_u64())
            .and_then(|priority| u32::try_from(priority).ok());
        formats.push(Conditional {
            range: range.to_string(),
            criterion: Criterion { op, value },
            format,
            fill,
            stop,
            priority,
        });
    }
    formats
}

/// The **area** a conditional format's range names.
pub(crate) fn area_of(range: &str) -> Option<Area> {
    let trimmed = range.trim();
    let area = match trimmed.split_once(':') {
        Some((first, second)) => {
            let first = crate::formula::Anchor::parse(first)?;
            let second = crate::formula::Anchor::parse(second)?;
            Area::new(first.cell, second.cell)
        }
        None => {
            let only = crate::formula::Anchor::parse(trimmed)?;
            Area::new(only.cell, only.cell)
        }
    };
    Some(area)
}

/// The **conditional formats** of a sheet, in the order they are
/// tried.
///
/// The rules are tried in **priority**: the rule that names the lower
/// number is tried first, and a rule that names none is tried after
/// the ones that do, in the order the sheet holds them. The tree
/// numbers the rules the same way — the rule that names the lower
/// number is seen first, and where two rules set the same part of a
/// cell the one seen first wins.
pub fn ordered_conditionals(node: &craftkit_sdk::Node) -> Vec<Conditional> {
    let mut conditionals = conditionals_of(node);
    conditionals.sort_by_key(|conditional| conditional.priority.unwrap_or(u32::MAX));
    conditionals
}

/// What a **conditional format** says about a cell.
#[derive(Debug, Clone, Default)]
pub struct Looks {
    /// The **format code** the cell is written with.
    pub format: Option<String>,
    /// The **fill** the cell is filled with.
    pub fill: Option<String>,
}

/// The **look** the conditional formats of a sheet give a cell.
///
/// The rules are tried in **priority**, and for each **part** of the
/// look the rule that is seen first and says something about it wins:
/// the tree's own rule, where two rules set the same part of a cell
/// the one seen first wins.
///
/// And the tree's **stop if true**: a rule that matched and says stop
/// ends the walk, and the rules after it are not tried.
pub fn looks_for(node: &craftkit_sdk::Node, cell: CellRef, value: &crate::formula::Value) -> Looks {
    let mut looks = Looks::default();
    for conditional in ordered_conditionals(node) {
        let Some(area) = area_of(&conditional.range) else { continue };
        if !area.covers(cell) {
            continue;
        }
        if !conditional.criterion.matches(value) {
            continue;
        }
        if looks.format.is_none() {
            looks.format = conditional.format.clone();
        }
        if looks.fill.is_none() {
            looks.fill = conditional.fill.clone();
        }
        if conditional.stop.unwrap_or(false) {
            break;
        }
    }
    looks
}

/// The conditional format that **writes** a cell, if one does.
///
/// The first one that matches wins, and the rules are tried in
/// priority.
pub fn format_for(node: &craftkit_sdk::Node, cell: CellRef, value: &crate::formula::Value) -> Option<String> {
    looks_for(node, cell, value).format
}

/// Put a **conditional format** on a range of cells.
///
/// The cells of the range whose value matches the criterion are
/// written with the format.
#[derive(Command)]
#[command(id = "grid.conditional.set", area = "grid", layer = "grid", icon = "brush", mutates, undoable)]
pub struct SetConditional {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The range it is about: `A1:A10`.
    pub range: Text,
    /// The operation: `=`, `!=`, `<`, `<=`, `>` or `>=`.
    pub op: Text,
    /// The value it compares with.
    pub value: Text,
    /// The format the matching cells are written with. A rule may
    /// carry the format, the fill, or both.
    pub format: Option<Text>,
    /// The fill the matching cells are filled with: `#rrggbb`. A rule
    /// may carry the fill and no format.
    pub fill: Option<Text>,
    /// The tree's **stop if true**: the walk ends at this rule when it
    /// matched, and the rules after it are not tried.
    pub stop: Option<Bool>,
    /// The **priority** of the rule: the lower the number, the earlier
    /// the rule is tried.
    pub priority: Option<Number>,
}

impl SetConditional {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.conditional.set".to_string();
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
        let range = self.range.0.trim().to_string();
        let Some(area) = area_of(&range) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{range}` is not a range of cells"),
            });
        };
        let op = self.op.0.trim().to_string();
        if !Criterion::op_is_known(&op) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no operation `{op}`: the operations are {}",
                    FILTER_OPS.join(", ")
                ),
            });
        }
        let value = self.value.0.clone();
        if (op == "<" || op == "<=" || op == ">" || op == ">=") && value.trim().parse::<f64>().is_err() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the operation `{op}` compares numbers, and `{value}` is not one"),
            });
        }
        // The **format**: the tree's rule says the format of the
        // cells, and a rule may say the fill and no format.
        let format = match &self.format {
            Some(given) => {
                let given = given.0.trim().to_string();
                if given.chars().count() > crate::format::MAX_FORMAT_CHARS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "a format is at most {} characters long",
                            crate::format::MAX_FORMAT_CHARS
                        ),
                    });
                }
                match crate::formatnames::resolve(&given) {
                    Ok(code) => Some(code),
                    Err(refusal) => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: crate::formatnames::message(&given, &refusal),
                        });
                    }
                }
            }
            None => None,
        };
        // The **fill**: a colour, and nothing else.
        let fill = match &self.fill {
            Some(given) => {
                let Some(colour) = crate::colours::colour_of(&given.0) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{}` is not a colour: a colour is `#rrggbb`", given.0.trim()),
                    });
                };
                match colour {
                    Some(colour) => Some(colour),
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "a rule's fill is a colour, and `none` is not one".to_string(),
                        });
                    }
                }
            }
            None => None,
        };
        if format.is_none() && fill.is_none() {
            return Err(EngineError::Failed {
                id,
                message: "name what the rule says: the format, the fill, or both".to_string(),
            });
        }
        let stop = self.stop.map(|stop| stop.0);
        let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
        // The **priority** is a whole number, and it is not a negative
        // one.
        let priority = match self.priority {
            Some(priority) => {
                let number = priority.0;
                if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > u32::MAX as f64 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the priority of a rule is a whole number, and {number} is not one"),
                    });
                }
                Some(number as u32)
            }
            None => None,
        };
        let mut list = conditionals_of(node)
            .into_iter()
            .map(|conditional| {
                json!({
                    "range": conditional.range,
                    "op": conditional.criterion.op,
                    "value": conditional.criterion.value,
                    "format": conditional.format,
                    "fill": conditional.fill,
                    "stop": conditional.stop,
                    "priority": conditional.priority,
                })
            })
            .collect::<Vec<_>>();
        // A range may have **one** conditional format: a second one for
        // the same range replaces the first.
        list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
        list.push(json!({
            "range": range,
            "op": op,
            "value": value,
            "format": format,
            "fill": fill,
            "stop": stop,
            "priority": priority,
        }));
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["conditionals"] = json!(list);
        }
        Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
            json!({
                "sheet": sheet,
                "range": range,
                "op": op,
                "value": value,
                "format": format,
                "fill": fill,
                "stop": stop,
            }),
        ))
    }
}

/// Take a **conditional format** away.
#[derive(Command)]
#[command(id = "grid.conditional.clear", area = "grid", layer = "grid", icon = "trash", mutates, undoable)]
pub struct ClearConditional {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The range it was about: `A1:A10`.
    pub range: Text,
}

impl ClearConditional {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.conditional.clear".to_string();
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
        let range = self.range.0.trim().to_string();
        let Some(area) = area_of(&range) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{range}` is not a range of cells"),
            });
        };
        let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
        let conditionals = conditionals_of(node);
        if !conditionals.iter().any(|conditional| conditional.range == range) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no conditional format of {range} on the sheet `{sheet}`"),
            });
        }
        let list: Vec<_> = conditionals
            .into_iter()
            .filter(|conditional| conditional.range != range)
            .map(|conditional| {
                json!({
                    "range": conditional.range,
                    "op": conditional.criterion.op,
                    "value": conditional.criterion.value,
                    "format": conditional.format,
                    "fill": conditional.fill,
                    "stop": conditional.stop,
                    "priority": conditional.priority,
                })
            })
            .collect();
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["conditionals"] = json!(list);
        }
        Ok(Outcome::changed(format!("the conditional format of {range} is taken away"))
            .with_data(json!({ "sheet": sheet, "range": range })))
    }
}

/// The **cells** of a range that a conditional format holds for.
///
/// The cells are **derived**: they are worked out from the cells each
/// time they are asked for.
#[derive(Command)]
#[command(id = "grid.conditional.check", area = "grid", layer = "grid", icon = "list")]
pub struct CheckConditional {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The range it is about: `A1:A10`.
    pub range: Text,
}

impl CheckConditional {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.conditional.check".to_string();
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
        let range = self.range.0.trim().to_string();
        let Some(area) = area_of(&range) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{range}` is not a range of cells"),
            });
        };
        let conditionals = ordered_conditionals(node);
        let book = crate::book_of(session.document());
        let name = node.name.clone();
        let mut cells: Vec<Json> = Vec::new();
        for row in area.start.row..=area.end.row {
            for col in area.start.col..=area.end.col {
                let cell = CellRef { col, row };
                let value = book.value_of(&name, cell);
                let matched = conditionals.iter().find(|conditional| {
                    area_of(&conditional.range).is_some_and(|area| area.covers(cell))
                        && conditional.criterion.matches(&value)
                });
                if let Some(matched) = matched {
                    cells.push(json!({
                        "cell": cell.to_text(),
                        "format": matched.format,
                    }));
                }
            }
        }
        let count = cells.len();
        Ok(Outcome::new(format!("{count} cell(s) match")).with_data(json!({
            "sheet": sheet,
            "range": range,
            "cells": cells,
        })))
    }
}
