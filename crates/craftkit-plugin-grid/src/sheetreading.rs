//! **Reading** a sheet.
//!
//! A sheet may be read whole: the answer is everything the sheet holds
//! and everything that is said about it.
//!
//! Some of the answer is **derived**: the **used range** is the stretch
//! of the sheet that holds something, and the number of **cells** that
//! hold something is counted, not written down. The rest are facts:
//! the sheet's **name**, its **view**, its **zoom**, the rows and
//! columns that are **hidden**, its **protection**, and the **calendar**
//! its dates are counted in.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

use crate::protection::Protection;
use crate::view::view_of;

commands! {
    /// Read a **sheet**.
    grid.sheet.get {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet to read.

            sheet: NodeRef,
        },
        run: {

            let id = "grid.sheet.get".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed { id, message: format!("there is no sheet `{sheet}`") });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let name = node.name.clone();
            // The **used range** is derived: the stretch of the sheet that
            // holds something.
            let cells = crate::cells_of(node);
            let used = crate::reading::used_range(&cells).map(|area| area.to_text());
            let count = cells.len();
            // The **view** of the sheet.
            let view = view_of(node);
            let (rows, columns) = match view.freeze {
                Some((rows, columns)) => (json!(rows), json!(columns)),
                None => (Json::Null, Json::Null),
            };
            // The **protection** of the sheet, if it has one.
            let protection = match Protection::of(node) {
                Some(protection) => protection.to_json(),
                None => Json::Null,
            };
            let hidden_rows = node.data.get("hidden_rows").cloned().unwrap_or(Json::Null);
            let hidden_columns = node.data.get("hidden_columns").cloned().unwrap_or(Json::Null);
            let zoom = node.data.get("zoom").cloned().unwrap_or(Json::Null);
            let workbook = crate::tabs::workbook_of(session, sheet);
            // The **calendar** the dates of the sheet are counted in: it is
            // the workbook's, and every sheet of a workbook is in it.
            let system = crate::system_of(session, sheet);
            Ok(Outcome::new(format!("the sheet `{name}`")).with_data(json!({
                "id": sheet,
                "name": name,
                "workbook": workbook,
                "used_range": used,
                "cells": count,
                "view": json!({
                    "gridlines": view.gridlines,
                    "headings": view.headings,
                    "zeros": view.zeros,
                    "right_to_left": view.right_to_left,
                    "frozen_rows": rows,
                    "frozen_columns": columns,
                }),
                "zoom": zoom,
                "hidden_rows": hidden_rows,
                "hidden_columns": hidden_columns,
                "protection": protection,
                "date_system": system.name(),
            })))

        }
    }
}
