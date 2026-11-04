//! **Reading** a workbook.
//!
//! A workbook may be read whole: the answer is everything the workbook
//! holds and everything that is said about it.
//!
//! The facts of a workbook are its **name**, the **names** it defines,
//! the **styles** it carries, the **lists** of its own words, and the
//! **calendar** its serials are counted in.
//!
//! One part of the answer is **derived**: the **sheets** of the
//! workbook are its children, and the **used range** of each sheet is
//! the stretch of it that holds something.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// Read a **workbook**.
#[derive(Command)]
#[command(id = "grid.workbook.get", area = "grid", layer = "grid", icon = "list")]
pub struct ReadWorkbook {
    /// The workbook to read.
    pub workbook: NodeRef,
}

impl ReadWorkbook {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.workbook.get".to_string();
        let workbook = self.workbook.0;
        let Some(node) = session.document().node(workbook) else {
            return Err(EngineError::Failed { id, message: format!("there is no workbook `{workbook}`") });
        };
        if node.kind != "grid.workbook" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
            });
        }
        // The **sheets** of the workbook, and the used range of each.
        let mut sheets: Vec<Json> = Vec::new();
        for sheet in node.children.iter().filter(|child| child.kind == "grid.sheet") {
            let cells = crate::cells_of(sheet);
            let used = crate::reading::used_range(&cells).map(|area| area.to_text());
            sheets.push(json!({
                "id": sheet.id,
                "name": sheet.name,
                "used_range": used,
                "cells": cells.len(),
            }));
        }
        let names = node.data.get("names").cloned().unwrap_or(Json::Null);
        let styles = node.data.get("styles").cloned().unwrap_or(Json::Null);
        let lists = node.data.get("custom_lists").cloned().unwrap_or(Json::Null);
        let system = node
            .data
            .get("date_system")
            .and_then(|system| system.as_str())
            .map(|system| system.to_string())
            .unwrap_or_else(|| crate::dates::System::default().name().to_string());
        Ok(Outcome::new(format!("the workbook `{}`", node.name)).with_data(json!({
            "id": workbook,
            "name": node.name,
            "sheets": sheets,
            "names": names,
            "styles": styles,
            "custom_lists": lists,
            "date_system": system,
        })))
    }
}
