//! The **styles** of a workbook.
//!
//! A style is a **named format** of the workbook, and it is shared: a
//! hundred cells that name one style carry one format, and changing the
//! style changes every cell that names it.
//!
//! The tree's style is a whole thing — a font, a fill, a border, an
//! alignment, a **protection** and a number format — and the tree's
//! named styles are **built in**: a list of them that the tree knows,
//! plus the ones the file carries. Here a style carries a **format**
//! and a **lock**, and the lock is the tree's protection: the tree's
//! cell is locked or unlocked by its style and not by itself.
//!
//! The tree's rules that are here:
//!
//! - a style whose format is *General* does not change the cell's
//!   format — the cell keeps its own;
//! - the tree finds a style by its name, without regard to case, so
//!   two styles whose names differ only in case are one style;
//! - the **lock** of a style is the lock of the cells that name it,
//!   and it is **derived**: changing the style changes every cell that
//!   names it.
//!
//! And the one rule that is **this tree's own**: a cell that says of
//! itself whether it is locked says more than the style, so the
//! cell's own word comes first, and the style's word comes before the
//! sheet's protection. The tree has no such a word on the cell — the
//! tree's cell has one style and the style says everything.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{Align, Font};

/// A **style** of a workbook: a name, a format and a lock.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// The name the style is found by.
    pub name: String,
    /// The **format code** the style carries.
    pub format: String,
    /// The **lock** the style carries: the cells that name it are
    /// locked when the sheet is protected, and `None` is a style that
    /// says nothing.
    pub locked: Option<bool>,
    /// The **alignment** the style carries, and `None` is a style that
    /// says nothing.
    pub align: Option<Align>,
    /// The **font** the style carries, and `None` is a style that
    /// says nothing.
    pub font: Option<Font>,
}

impl Style {
    /// The style, as the payload carries it.
    pub fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "name": self.name,
            "format": self.format,
            "locked": self.locked,
            "align": self.align.map(|align| align.to_json()),
            "font": self.font.as_ref().map(|font| font.to_json()),
        })
    }
}

/// The **styles** of a workbook, from its payload.
pub fn styles_of_book(node: &craftkit_sdk::Node) -> Vec<Style> {
    node.data
        .get("styles")
        .and_then(|styles| styles.as_array())
        .map(|styles| {
            styles
                .iter()
                .filter_map(|style| {
                    let name = style.get("name")?.as_str()?.to_string();
                    let format = style.get("format")?.as_str()?.to_string();
                    let locked = style.get("locked").and_then(|locked| locked.as_bool());
                    let align = style.get("align").and_then(Align::from_json);
                    let font = style.get("font").and_then(Font::from_json);
                    Some(Style { name, format, locked, align, font })
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
        let given = self.format.0.trim().to_string();
        let code = match crate::formatnames::resolve(&given) {
            Ok(code) => code,
            Err(refusal) => {
                return Err(EngineError::Failed {
                    id,
                    message: crate::formatnames::message(&given, &refusal),
                });
            }
        };
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("the style `{name}` is already in the workbook"),
            });
        }
        let mut styles = styles.iter().map(Style::to_json).collect::<Vec<_>>();
        styles.push(
            Style { name: name.clone(), format: code.clone(), locked: None, align: None, font: None }
                .to_json(),
        );
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
        let given = self.format.0.trim().to_string();
        let code = match crate::formatnames::resolve(&given) {
            Ok(code) => code,
            Err(refusal) => {
                return Err(EngineError::Failed {
                    id,
                    message: crate::formatnames::message(&given, &refusal),
                });
            }
        };
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if !styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        // The **lock** of the style is kept: the format is what this
        // command changes, and the lock is the style's own.
        let styles: Vec<_> = styles
            .into_iter()
            .map(|style| {
                if style.name.eq_ignore_ascii_case(&name) {
                    Style { format: code.clone(), ..style }.to_json()
                } else {
                    style.to_json()
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

/// The **lock** of a style is changed.
///
/// The lock is the tree's **protection**: the cells that name the style
/// are locked when the sheet is protected, and a style that says
/// nothing leaves the cells to the sheet.
///
/// The lock is **derived**: the cells that name the style are locked or
/// unlocked because the style says so, and changing the style changes
/// every cell that names it.
#[derive(Command)]
#[command(id = "grid.style.lock", area = "grid", layer = "grid", icon = "lock", mutates, undoable)]
pub struct LockStyle {
    /// The workbook the style belongs to.
    pub workbook: NodeRef,
    /// The name of the style.
    pub name: Text,
    /// Whether the cells that name it are locked.
    pub locked: Bool,
}

impl LockStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.style.lock".to_string();
        let workbook = the_workbook(session, self.workbook.0, &id)?;
        let name = self.name.0.trim().to_string();
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if !styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        let locked = self.locked.0;
        let styles: Vec<_> = styles
            .into_iter()
            .map(|style| {
                if style.name.eq_ignore_ascii_case(&name) {
                    Style { locked: Some(locked), ..style }.to_json()
                } else {
                    style.to_json()
                }
            })
            .collect();
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["styles"] = json!(styles);
        }
        Ok(Outcome::changed(format!("the style `{name}` is {}", if locked { "locked" } else { "unlocked" }))
            .with_data(json!({ "workbook": workbook, "name": name, "locked": locked })))
    }
}

/// The **alignment** of a style is changed.
///
/// The tree's alignment is a style's, and here a style may carry one:
/// the cells that name the style and say nothing of their own are
/// aligned by it.
///
/// The alignment is **derived**: the cells that name the style are
/// aligned because the style says so, and changing the style changes
/// every cell that names it.
#[derive(Command)]
#[command(id = "grid.style.align", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct AlignStyle {
    /// The workbook the style belongs to.
    pub workbook: NodeRef,
    /// The name of the style.
    pub name: Text,
    /// How the text lies **across** the cell: general, left, center,
    /// right, fill, justify or center-across.
    pub horizontal: Option<Text>,
    /// How the text lies **down** the cell: top, middle or bottom.
    pub vertical: Option<Text>,
    /// How far the text is indented.
    pub indent: Option<Number>,
    /// Whether the text wraps.
    pub wrap: Option<Bool>,
}

impl AlignStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.style.align".to_string();
        let workbook = the_workbook(session, self.workbook.0, &id)?;
        let name = self.name.0.trim().to_string();
        let parts = crate::align::AlignParts::parse(
            &id,
            self.horizontal.as_ref(),
            self.vertical.as_ref(),
            self.indent,
            self.wrap,
        )?;
        if parts.is_empty() {
            return Err(EngineError::Failed {
                id,
                message: "name what to align: the alignment across the cell, down it, the indent or the wrap"
                    .to_string(),
            });
        }
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if !styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        // The **alignment** of the style: the tree's own rules, and
        // naming the alignment that is there takes it back to general.
        let styles: Vec<_> = styles
            .into_iter()
            .map(|style| {
                if style.name.eq_ignore_ascii_case(&name) {
                    let align = parts.apply(style.align.unwrap_or_else(Align::plain));
                    Style { align: Some(align), ..style }.to_json()
                } else {
                    style.to_json()
                }
            })
            .collect();
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["styles"] = json!(styles);
        }
        Ok(Outcome::changed(format!("the style `{name}` is aligned")).with_data(json!({
            "workbook": workbook,
            "name": name,
        })))
    }
}

/// The **font** of a style is changed.
///
/// The tree's font is a style's, and here a style may carry one: the
/// cells that name the style and say nothing of their own are written
/// with it.
///
/// The font is **derived**: the cells that name the style are written
/// with the font because the style says so, and changing the style
/// changes every cell that names it.
#[derive(Command)]
#[command(id = "grid.style.font", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct FontStyle {
    /// The workbook the style belongs to.
    pub workbook: NodeRef,
    /// The name of the style.
    pub name: Text,
    /// The **name** of the face. It is the face's name and not the
    /// style's, so it is called `face` here.
    pub face: Option<Text>,
    /// The **size** of the text.
    pub size: Option<Number>,
    /// The **underline**: none, single, double, single accounting or
    /// double accounting.
    pub underline: Option<Text>,
    /// Where the text sits **above or below** its line: baseline,
    /// superscript or subscript.
    pub vert: Option<Text>,
}

impl FontStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.style.font".to_string();
        let workbook = the_workbook(session, self.workbook.0, &id)?;
        let name = self.name.0.trim().to_string();
        let parts = crate::font::FontParts::parse(
            &id,
            self.face.as_ref(),
            self.size,
            self.underline.as_ref(),
            self.vert.as_ref(),
        )?;
        if parts.is_empty() {
            return Err(EngineError::Failed {
                id,
                message: "name what to set: the face, the size, the underline or the place of the text"
                    .to_string(),
            });
        }
        let styles = session.document().node(workbook).map(styles_of_book).unwrap_or_default();
        if !styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        // The **font** of the style: the tree's own rules, and naming
        // the underline that is there takes it away.
        let styles: Vec<_> = styles
            .into_iter()
            .map(|style| {
                if style.name.eq_ignore_ascii_case(&name) {
                    let font = parts.apply(style.font.unwrap_or_else(Font::plain));
                    Style { font: Some(font), ..style }.to_json()
                } else {
                    style.to_json()
                }
            })
            .collect();
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["styles"] = json!(styles);
        }
        Ok(Outcome::changed(format!("the style `{name}` has a font")).with_data(json!({
            "workbook": workbook,
            "name": name,
        })))
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
        if !styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
            return Err(EngineError::Failed {
                id,
                message: format!("there is no style `{name}` in the workbook"),
            });
        }
        let styles: Vec<_> = styles
            .into_iter()
            .filter(|style| !style.name.eq_ignore_ascii_case(&name))
            .map(|style| style.to_json())
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
            if !styles.iter().any(|style| style.name.eq_ignore_ascii_case(&name)) {
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
        crate::protection::refuse_format(node, &id)?;
        let mut cells = crate::sheet_cells_mut(node);
        cells.set_style(cell, if name.is_empty() { None } else { Some(name.clone()) });
        node.data["cells"] = crate::cells_into(&cells);
        Ok(Outcome::changed(format!("the cell {} names the style `{name}`", cell.to_text()))
            .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "style": name })))
    }
}
