//! The **scenarios** of a workbook: the tree's own what-if **scenarios**.
//!
//! The tree's own: a **scenario** is a set of **changing cells** and the
//! **values** they hold when the scenario is shown — a "what if the
//! price were this" of the tree's own — and a workbook holds a list of
//! them.
//!
//! # The rules
//!
//! - The **name** of a scenario is not empty and is at most **255**
//!   characters: the tree's own bound.
//! - A scenario has at most **32** changing cells: the tree's own
//!   bound, and a scenario with no changing cell is **refused**: the
//!   tree's own rule.
//! - The **values** are one for each changing cell, and when the
//!   caller says none they are the **values the cells hold now**: the
//!   tree's own rule.
//! - A scenario of the **same name on the same sheet** is refused, and
//!   it is replaced when the caller says so: the tree's own rule.
//! - A workbook holds at most **1000** scenarios: the tree's own bound.
//!
//! # Where they are kept
//!
//! The tree keeps the scenarios in a **hidden name** of the workbook —
//! a name that says nothing and holds a JSON text — and here they are
//! **nodes under the workbook**: a scenario is a thing of its own, and
//! the document holds it.
//!
//! # What it does not settle
//!
//! - The tree's own **summary** of the scenarios — a sheet that says
//!   what every scenario does to the book — is not here: it is a sheet
//!   of its own, and it is a trial of the book as well.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "grid.scenario";

/// The tree's own **bound** on the length of the name of a scenario.
const MAX_NAME: usize = 255;

/// The tree's own **bound** on the changing cells of a scenario.
const MAX_CELLS: usize = 32;

/// The tree's own **bound** on the scenarios of a workbook.
const MAX_SCENARIOS: usize = 1000;

/// The kind of the node a scenario is.
#[derive(craftkit_derive::NodeKind)]
#[node_kind(kind = "grid.scenario", label = "Scenario", layer = "grid", evaluates)]
pub struct ScenarioKind;

impl ScenarioKind {
    /// A scenario is not drawn.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// A scenario is not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a scenario **means**: its name, the sheet it is on, the
    /// cells it changes and the values they hold.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "name": node.data.get("name").cloned().unwrap_or(Json::Null),
            "sheet": node.data.get("sheet").cloned().unwrap_or(Json::Null),
            "changing": node.data.get("changing").cloned().unwrap_or(Json::Null),
            "values": node.data.get("values").cloned().unwrap_or(Json::Null),
            "comment": node.data.get("comment").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// The **scenarios** of a workbook: the nodes under it that are
/// scenarios.
fn scenarios_of(session: &Session, workbook: u64) -> Vec<u64> {
    let Some(node) = session.document().node(workbook) else {
        return Vec::new();
    };
    node.children.iter().filter(|child| child.kind == "grid.scenario").map(|child| child.id).collect()
}

/// The **name** a scenario carries.
fn name_of(session: &Session, scenario: u64) -> String {
    session
        .document()
        .node(scenario)
        .and_then(|node| node.data.get("name").and_then(|name| name.as_str()).map(|name| name.to_string()))
        .unwrap_or_default()
}

/// The **sheet** a scenario is on.
fn sheet_of(session: &Session, scenario: u64) -> u64 {
    session
        .document()
        .node(scenario)
        .and_then(|node| node.data.get("sheet").and_then(|sheet| sheet.as_u64()))
        .unwrap_or(0)
}

/// The scenario of a workbook with that **name**, on that sheet or on
/// any sheet: the tree's own rule — the sheet the caller names first,
/// and then any other.
fn the_scenario(session: &Session, workbook: u64, name: &str, sheet: Option<u64>) -> Option<u64> {
    let wanted = name.trim().to_lowercase();
    let scenarios = scenarios_of(session, workbook);
    if let Some(sheet) = sheet
        && let Some(found) = scenarios.iter().copied().find(|scenario| {
            sheet_of(session, *scenario) == sheet && name_of(session, *scenario).to_lowercase() == wanted
        })
    {
        return Some(found);
    }
    scenarios.iter().copied().find(|scenario| name_of(session, *scenario).to_lowercase() == wanted)
}

/// The **changing cells** of a scenario, with their **values**.
fn changing_of(session: &Session, scenario: u64) -> (Vec<String>, Vec<Json>) {
    let Some(node) = session.document().node(scenario) else {
        return (Vec::new(), Vec::new());
    };
    let changing = node
        .data
        .get("changing")
        .and_then(|list| list.as_array())
        .map(|list| list.iter().filter_map(|cell| cell.as_str().map(|cell| cell.to_string())).collect())
        .unwrap_or_default();
    let values = node.data.get("values").and_then(|list| list.as_array()).cloned().unwrap_or_default();
    (changing, values)
}

/// **Add** a **scenario** to a workbook.
///
/// The tree's own `scenarioAdd`.
#[derive(Command)]
#[command(id = "grid.scenario.add", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct AddScenario {
    /// The workbook the scenario is in.
    pub workbook: NodeRef,
    /// The sheet the scenario is on.
    pub sheet: NodeRef,
    /// The name of the scenario.
    pub name: Text,
    /// The **changing cells**: a list of the addresses of them.
    pub changing: Json,
    /// The **values** they hold: one for each changing cell.
    ///
    /// Leave them out and they are the values the cells hold now.
    pub values: Option<Json>,
    /// A **comment** on the scenario.
    pub comment: Option<Text>,
    /// Whether a scenario of the same name is **replaced**.
    pub replace: Option<Bool>,
}

impl AddScenario {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = format!("{COMMAND}.add");
        let workbook = self.workbook.0;
        let sheet = self.sheet.0;
        // The **name**: not empty and not too long: the tree's own rule.
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a scenario has a name".to_string() });
        }
        if name.chars().count() > MAX_NAME {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the name of a scenario is at most {MAX_NAME} characters"),
            });
        }
        // The **sheet**: a sheet of the workbook.
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
        // The **changing cells**: a list of addresses, and not too many:
        // the tree's own bound.
        let Some(list) = self.changing.as_array() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the changing cells of a scenario are a list of addresses".to_string(),
            });
        };
        if list.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a scenario has no changing cells".to_string(),
            });
        }
        if list.len() > MAX_CELLS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "a scenario has at most {MAX_CELLS} changing cells, and this one has {}",
                    list.len()
                ),
            });
        }
        let mut changing: Vec<String> = Vec::with_capacity(list.len());
        for item in list {
            let Some(address) = item.as_str() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the changing cells of a scenario are a list of addresses".to_string(),
                });
            };
            let address = address.trim().to_uppercase();
            crate::cell_at(&address, &id)?;
            changing.push(address);
        }
        // The **values**: one for each changing cell, and when the
        // caller says none they are the values the cells hold now: the
        // tree's own rule.
        let values: Vec<Json> = match &self.values {
            Some(values) => {
                let Some(list) = values.as_array() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the values of a scenario are a list of values".to_string(),
                    });
                };
                if list.len() != changing.len() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the scenario changes {} cells and holds {} values",
                            changing.len(),
                            list.len()
                        ),
                    });
                }
                list.clone()
            }
            None => {
                let cells = crate::cells_of(node);
                let mut values: Vec<Json> = Vec::with_capacity(changing.len());
                for address in &changing {
                    let cell = crate::cell_at(address, &id)?;
                    let value = cells.get(cell).and_then(|entry| entry.value.clone());
                    values.push(value.map(|value| value.to_json()).unwrap_or(Json::Null));
                }
                values
            }
        };
        // A scenario of the **same name on the same sheet** is refused,
        // and it is replaced when the caller says so: the tree's own
        // rule.
        let replace = matches!(self.replace, Some(Bool(true)));
        let existing = the_scenario(session, workbook, &name, Some(sheet));
        if let Some(existing) = existing {
            if sheet_of(session, existing) == sheet && !replace {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a scenario named `{name}` is already on this sheet"),
                });
            }
            if replace && sheet_of(session, existing) == sheet {
                let comment = self.comment.as_ref().map(|comment| comment.0.clone()).unwrap_or_default();
                let data = json!({
                    "name": name,
                    "sheet": sheet,
                    "changing": changing,
                    "values": values,
                    "comment": comment,
                });
                if let Some(node) = session.document_mut()?.node_mut(existing) {
                    node.data = data;
                }
                return Ok(Outcome::changed(format!("the scenario `{name}` is replaced")).with_data(json!({
                    "id": existing,
                    "workbook": workbook,
                    "sheet": sheet,
                    "name": name,
                    "changing": changing.len(),
                })));
            }
        }
        // A workbook holds at most **1000** scenarios: the tree's own
        // bound.
        if scenarios_of(session, workbook).len() >= MAX_SCENARIOS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a workbook holds at most {MAX_SCENARIOS} scenarios"),
            });
        }
        let comment = self.comment.as_ref().map(|comment| comment.0.clone()).unwrap_or_default();
        let data = json!({
            "name": name,
            "sheet": sheet,
            "changing": changing,
            "values": values,
            "comment": comment,
        });
        let scenario = session.document_mut()?.add_node_with_under(
            Some(workbook),
            name.clone(),
            "grid.scenario",
            None,
            data,
        );
        Ok(Outcome::changed(format!("the scenario `{name}` is added")).with_data(json!({
            "id": scenario,
            "workbook": workbook,
            "sheet": sheet,
            "name": name,
            "changing": changing.len(),
        })))
    }
}

/// **List** the **scenarios** of a workbook.
#[derive(Command)]
#[command(id = "grid.scenario.list", area = "grid", layer = "grid", icon = "list")]
pub struct ListScenarios {
    /// The workbook the scenarios are in.
    pub workbook: NodeRef,
    /// The sheet the scenarios are on.
    ///
    /// Leave it out and every scenario of the workbook is listed.
    pub sheet: Option<NodeRef>,
}

impl ListScenarios {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = format!("{COMMAND}.list");
        let _ = &id;
        let workbook = self.workbook.0;
        let sheet = self.sheet.map(|sheet| sheet.0);
        let mut scenarios: Vec<Json> = Vec::new();
        for scenario in scenarios_of(session, workbook) {
            let on = sheet_of(session, scenario);
            if let Some(sheet) = sheet
                && on != sheet
            {
                continue;
            }
            let (changing, values) = changing_of(session, scenario);
            scenarios.push(json!({
                "id": scenario,
                "name": name_of(session, scenario),
                "sheet": on,
                "changing": changing,
                "values": values,
                "comment": session
                    .document()
                    .node(scenario)
                    .and_then(|node| node.data.get("comment").and_then(|comment| comment.as_str()).map(|comment| comment.to_string()))
                    .unwrap_or_default(),
            }));
        }
        Ok(Outcome::changed(format!("the workbook `{workbook}` holds {} scenarios", scenarios.len()))
            .with_data(json!({ "workbook": workbook, "scenarios": scenarios })))
    }
}

/// **Show** a **scenario**: the changing cells hold the values of it.
///
/// The tree's own `scenarioShow`.
#[derive(Command)]
#[command(id = "grid.scenario.show", area = "grid", layer = "grid", icon = "play", mutates, undoable)]
pub struct ShowScenario {
    /// The workbook the scenario is in.
    pub workbook: NodeRef,
    /// The name of the scenario.
    pub name: Text,
    /// The sheet the scenario is on.
    pub sheet: Option<NodeRef>,
}

impl ShowScenario {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = format!("{COMMAND}.show");
        let workbook = self.workbook.0;
        let name = self.name.0.trim().to_string();
        let Some(scenario) = the_scenario(session, workbook, &name, self.sheet.map(|sheet| sheet.0)) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no scenario named `{name}` in this workbook"),
            });
        };
        let sheet = sheet_of(session, scenario);
        let (changing, values) = changing_of(session, scenario);
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let mut cells = crate::sheet_cells_mut(node);
        let mut shown: Vec<Json> = Vec::with_capacity(changing.len());
        for (index, address) in changing.iter().enumerate() {
            let cell = crate::cell_at(address, &id)?;
            // The tree's own rule: a number is a number and not typed
            // text.
            if let Some(value) = values.get(index)
                && !value.is_null()
            {
                cells.set_value(cell, crate::formula::Value::from_json(value));
            }
            shown.push(json!(address.clone()));
        }
        node.data["cells"] = crate::cells_into(&cells);
        Ok(Outcome::changed(format!("the scenario `{name}` is shown")).with_data(json!({
            "id": scenario,
            "workbook": workbook,
            "sheet": sheet,
            "name": name,
            "cells": shown,
        })))
    }
}

/// **Delete** a **scenario** of a workbook.
///
/// The tree's own `scenarioDelete`.
#[derive(Command)]
#[command(id = "grid.scenario.delete", area = "grid", layer = "grid", icon = "trash", mutates, undoable)]
pub struct DeleteScenario {
    /// The workbook the scenario is in.
    pub workbook: NodeRef,
    /// The name of the scenario.
    pub name: Text,
    /// The sheet the scenario is on.
    pub sheet: Option<NodeRef>,
}

impl DeleteScenario {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = format!("{COMMAND}.delete");
        let workbook = self.workbook.0;
        let name = self.name.0.trim().to_string();
        let Some(scenario) = the_scenario(session, workbook, &name, self.sheet.map(|sheet| sheet.0)) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no scenario named `{name}` in this workbook"),
            });
        };
        if !session.document_mut()?.remove_node(scenario) {
            return Err(EngineError::Failed {
                id,
                message: format!("the scenario `{name}` could not be taken away"),
            });
        }
        Ok(Outcome::changed(format!("the scenario `{name}` is taken away"))
            .with_data(json!({ "workbook": workbook, "name": name })))
    }
}
