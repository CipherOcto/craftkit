//! The **exporting** of a sheet as text of values.
//!
//! A sheet may be **exported**: the cells of a range are written as a
//! text of values separated by a delimiter — a **CSV** — which is the
//! same shape the tree's import reads.
//!
//! The rules:
//!
//! - a cell is written as it is **shown**: the text a person reads,
//!   with its format;
//! - a field that holds the **delimiter**, a quote or a line break is
//!   **quoted**, and a quote inside it is doubled;
//! - an **empty** cell is an empty field, and the fields of a row
//!   are all there — a row of three cells and one of two are lines
//!   of the same width only when the range says so;
//! - the **delimiter** is one character, and it is a comma unless
//!   the file says otherwise.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::CellRef;

/// The **field** a cell is written as: quoted when it has to be.
fn field_of(text: &str, delimiter: char) -> String {
    let quoted = text.contains(delimiter) || text.contains('"') || text.contains('\n') || text.contains('\r');
    if !quoted {
        return text.to_string();
    }
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// **Export** a range of a sheet as a text of values.
///
/// A query: it does not change the document — the text is given, and
/// writing it to a file is the caller's business.
#[derive(Command)]
#[command(id = "grid.export.csv", area = "grid", layer = "grid", icon = "list")]
pub struct ExportCsv {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The range to export. Without it, the used range is exported.
    pub range: Option<Text>,
    /// The **delimiter**: one character. The default is a comma.
    pub delimiter: Option<Text>,
}

impl ExportCsv {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.export.csv".to_string();
        let sheet = self.sheet.0;
        let delimiter = match &self.delimiter {
            None => ',',
            Some(delimiter) => {
                let trimmed = delimiter.0.trim();
                let mut characters = trimmed.chars();
                let Some(first) = characters.next() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a delimiter is one character, and this is none".to_string(),
                    });
                };
                if characters.next().is_some() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{trimmed}` is not a delimiter: a delimiter is one character"),
                    });
                }
                first
            }
        };
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
        let name = node.name.clone();
        let cells = crate::cells_of(node);
        // The **range**: the one that was named, or the used range.
        let area = match &self.range {
            Some(range) => crate::fill::area_at(&range.0, &id)?,
            None => match crate::reading::used_range(&cells) {
                Some(area) => area,
                None => {
                    return Ok(Outcome::new(format!("{name} holds nothing to export")).with_data(json!({
                        "sheet": sheet,
                        "text": "",
                        "rows": 0,
                        "columns": 0,
                    })));
                }
            },
        };
        // The **lines**: one per row of the range.
        let mut lines: Vec<String> = Vec::new();
        for row in area.start.row..=area.end.row {
            let mut fields: Vec<String> = Vec::new();
            for column in area.start.col..=area.end.col {
                let cell = CellRef { col: column, row };
                let text = crate::finding::text_of(session, sheet, &name, &cells, cell, false);
                fields.push(field_of(&text, delimiter));
            }
            lines.push(fields.join(&delimiter.to_string()));
        }
        let rows = lines.len();
        let columns = area.end.col.saturating_sub(area.start.col) + 1;
        let text = lines.join("\n");
        Ok(Outcome::new(format!("exported {rows} row(s)")).with_data(json!({
            "sheet": sheet,
            "range": area.to_text(),
            "delimiter": delimiter.to_string(),
            "rows": rows,
            "columns": columns,
            "text": text,
        })))
    }
}
