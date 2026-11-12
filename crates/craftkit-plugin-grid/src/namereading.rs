//! **Reading** a name.
//!
//! A name is a **word** that stands for a range of cells, and a name
//! may be read: the answer is the range it stands for and the **values**
//! of the cells of that range.
//!
//! The values are **derived**: the book computes them, and so the value
//! of a cell that holds a formula is the value the formula computes.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{CellRef, Value};

/// Read a **name** of a workbook.
#[derive(Command)]
#[command(id = "grid.name.get", area = "grid", layer = "grid", icon = "list")]
pub struct ReadName {
    /// The workbook the name is in.
    pub workbook: NodeRef,
    /// The name.
    pub name: Text,
}

impl ReadName {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.name.get".to_string();
        let workbook = self.workbook.0;
        let Some(node) = session.document().node(workbook) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{workbook}`"),
            });
        };
        if node.kind != "grid.workbook" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
            });
        }
        let name = self.name.0.trim().to_string();
        let names = node.data.get("names").cloned().unwrap_or(json!({}));
        let Some(refers_to) =
            names.get(&name).and_then(|refers_to| refers_to.as_str()).map(|r| r.to_string())
        else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no name `{name}` in the workbook `{workbook}`"),
            });
        };
        // The range the name stands for, and the sheet it is in.
        let (sheet_name, area) = crate::names::parse_refers_to(&refers_to)
            .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let document = session.document();
        let Some(sheet_node) =
            document.walk().find(|node| node.kind == "grid.sheet" && node.name == sheet_name)
        else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet_name}`, and the name stands for a range of it"),
            });
        };
        let sheet = sheet_node.id;
        let cells = crate::cells_of(sheet_node);
        // The **values** are derived: the book computes them.
        let book = crate::book_of(document);
        let mut rows: Vec<Vec<Json>> = Vec::new();
        for row in area.start.row..=area.end.row {
            let mut cells_in_row: Vec<Json> = Vec::new();
            for col in area.start.col..=area.end.col {
                let cell = CellRef { col, row };
                let value = match cells.get(cell) {
                    Some(entry) => match (&entry.value, &entry.formula) {
                        (Some(value), _) => value.clone(),
                        (None, Some(_)) => book.value_of(&sheet_name, cell),
                        (None, None) => Value::Empty,
                    },
                    None => Value::Empty,
                };
                cells_in_row.push(value.to_json());
            }
            rows.push(cells_in_row);
        }
        Ok(Outcome::new(format!("the name `{name}` stands for {refers_to}")).with_data(json!({
            "workbook": workbook,
            "name": name,
            "refers_to": refers_to,
            "sheet": sheet,
            "range": area.to_text(),
            "values": rows,
        })))
    }
}
