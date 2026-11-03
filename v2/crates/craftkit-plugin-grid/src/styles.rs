//! The **styles** of a workbook.
//!
//! A style is a **named format** of the workbook, and it is shared: a
//! hundred cells that name one style carry one format, and changing the
//! style changes every cell that names it.
//!
//! The tree's style is a whole thing — a font, a fill, a border, an
//! alignment and a number format — and the tree's named styles are
//! **built in**: a list of them that the tree knows, plus the ones the
//! file carries. Here a style carries a **format** and nothing else,
//! because a format is the only thing this tree writes on a cell.
//!
//! The tree's rule that is here: a style whose format is *General* does
//! not change the cell's format — the cell keeps its own. And the tree
//! finds a style by its name, without regard to case, so two styles
//! whose names differ only in case are one style.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::format::Format;

/// The **styles** of a workbook, from its payload.
fn styles_of_book(node: &craftkit_sdk::Node) -> Vec<(String, String)> {
    node.data
        .get("styles")
        .and_then(|styles| styles.as_array())
        .map(|styles| {
            styles
                .iter()
                .filter_map(|style| {
                    let name = style.get("name")?.as_str()?.to_string();
                    let format = style.get("format")?.as_str()?.to_string();
                    Some((name, format))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **workbook** a command is about, or the error that says there is
/// no such workbook.
fn the_workbook(session: &Session, workbook: u64, id: &str) -> Result<u64, EngineError> {
    let Some(node) = session.document().node(workbook) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no workbook `{workbook}`"),
        });
    };
    if node.kind != "grid.workbook" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
        });
    }
    Ok(workbook)
}

/// A **style** is added to a workbook.
///
/// The name must not be there already — the tree finds a style by its
/// name and without regard to case, so two styles whose names differ
/// only in case are one style — and the format must be a format this
/// tree has.
#[derive(Command)]
#[command(id = "grid.style.add", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct AddStyle {
    /// The workbook the style belongs to.
    pub workbook: NodeRef,
    /// The name of the style.
    pub name: Text,
    /// The format the style carries.
    pub format: Text,
}

impl AddStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.style.add".to_string();
        let workbook = the_workbook(session, self.workbook.0, &id)?;
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id, message: "a style has a name".to_string() });
        }
        let code = self.format.0.trim().to_string();
        if Format::parse(&code).is_none() {
            return Err(EngineError::Failed {
                id,
                message: format!("`{code}` is not a format this tree has"),
            });
        }
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if styles.iter().any(|(style, _)| style.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("the style `{name}` is already in the workbook"),
            });
        }
        let mut styles = styles
            .into_iter()
            .map(|(name, format)| json!({ "name": name, "format": format }))
            .collect::<Vec<_>>();
        styles.push(json!({ "name": name, "format": code }));
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["styles"] = json!(styles);
        }
        Ok(Outcome::changed(format!("the style `{name}` is in the workbook"))
            .with_data(json!({ "workbook": workbook, "name": name, "format": code })))
    }
}

/// The **format** of a style is changed.
///
/// Every cell that names the style is written with the new format: the
/// format of a cell is **derived**, and a style that changes changes
/// them all.
#[derive(Command)]
#[command(id = "grid.style.set", area = "grid", layer = "grid", icon = "pencil", mutates, undoable)]
pub struct SetStyle {
    /// The workbook the style belongs to.
    pub workbook: NodeRef,
    /// The name of the style.
    pub name: Text,
    /// The format the style carries.
    pub format: Text,
}

impl SetStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.style.set".to_string();
        let workbook = the_workbook(session, self.workbook.0, &id)?;
        let name = self.name.0.trim().to_string();
        let code = self.format.0.trim().to_string();
        if Format::parse(&code).is_none() {
            return Err(EngineError::Failed {
                id,
                message: format!("`{code}` is not a format this tree has"),
            });
        }
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if !styles.iter().any(|(style, _)| style.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        let styles: Vec<_> = styles
            .into_iter()
            .map(|(style, format)| {
                if style.eq_ignore_ascii_case(&name) {
                    json!({ "name": style, "format": code })
                } else {
                    json!({ "name": style, "format": format })
                }
            })
            .collect();
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["styles"] = json!(styles);
        }
        Ok(Outcome::changed(format!("the style `{name}` carries `{code}`"))
            .with_data(json!({ "workbook": workbook, "name": name, "format": code })))
    }
}

/// A **style** is taken away.
///
/// The cells that named it keep their own format: a style that is not
/// there is not a format, and a cell that names nothing is written as
/// it is.
#[derive(Command)]
#[command(id = "grid.style.remove", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct RemoveStyle {
    /// The workbook the style belongs to.
    pub workbook: NodeRef,
    /// The name of the style.
    pub name: Text,
}

impl RemoveStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.style.remove".to_string();
        let workbook = the_workbook(session, self.workbook.0, &id)?;
        let name = self.name.0.trim().to_string();
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if !styles.iter().any(|(style, _)| style.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        let styles: Vec<_> = styles
            .into_iter()
            .filter(|(style, _)| !style.eq_ignore_ascii_case(&name))
            .map(|(name, format)| json!({ "name": name, "format": format }))
            .collect();
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["styles"] = json!(styles);
        }
        Ok(Outcome::changed(format!("the style `{name}` is not in the workbook"))
            .with_data(json!({ "workbook": workbook, "name": name })))
    }
}

/// A **cell** names a style.
///
/// The style must be one the workbook has — a name that is not there is
/// refused, and never quietly ignored. An **empty name** means the
/// cell names no style, and the cell is written with its own format.
#[derive(Command)]
#[command(id = "grid.cell.style", area = "grid", layer = "grid", icon = "paint", mutates, undoable)]
pub struct SetCellStyle {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The name of the style. An empty name means the cell has no
    /// style.
    pub style: Text,
}

impl SetCellStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.style".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let name = self.style.0.trim().to_string();
        if !name.is_empty() {
            let styles = crate::styles_of(session, sheet);
            if !styles.iter().any(|(style, _)| style.eq_ignore_ascii_case(&name)) {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no style `{name}` in the workbook"),
                });
            }
        }
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed { id, message: format!("there is no sheet `{sheet}`") });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
            });
        }
        let mut cells = crate::sheet_cells_mut(node);
        cells.set_style(cell, if name.is_empty() { None } else { Some(name.clone()) });
        node.data["cells"] = crate::cells_into(&cells);
        Ok(Outcome::changed(format!("the cell {} names the style `{name}`", cell.to_text()))
            .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "style": name })))
    }
}
