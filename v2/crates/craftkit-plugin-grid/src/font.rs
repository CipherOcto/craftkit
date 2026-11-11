//! The **font** of a cell: the face of its text, the size of it, and
//! the marks that are put on it.
//!
//! The tree's rules:
//!
//! - the **face** is the name of a face;
//! - the **size** is from one to four hundred and nine, and it is
//!   **rounded** to the nearest half: the tree's own rule;
//! - the **underline** is none, single, double, single accounting or
//!   double accounting, and naming the underline a cell already has
//!   takes it away: the tree's own rule;
//! - the **vertical place** is baseline, superscript or subscript, and
//!   naming the place a cell already has takes it back to the
//!   baseline: the tree's own rule;
//! - **bold**, **italic** and **strike** are buttons, and a button
//!   pressed twice does the opposite: naming no value takes the
//!   opposite of what the cell has, and naming a value takes that
//!   value — the tree's own rule.
//!
//! # What the port found
//!
//! - **The tree's font is a style and this one's is the cell's own**
//!   (§271): the font of a cell is the cell's own, as its format and
//!   its alignment are.
//! - **The tree's font is a range and this one's is a cell**: the
//!   tree's button takes the selection, and here the cell is named.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{CellRef, Font, Underline, Vert};

/// The most **size** a font may have: the tree's own bound.
const MAX_SIZE: f64 = 409.0;

/// The least **size** a font may have: the tree's own bound.
const MIN_SIZE: f64 = 1.0;

/// The **font** of a cell, as it is now.
fn font_of(session: &Session, sheet: u64, cell: CellRef) -> Font {
    let Some(node) = session.document().node(sheet) else { return Font::plain() };
    crate::cells_of(node).font(cell).cloned().unwrap_or_else(Font::plain)
}

/// **Write** the font of a cell, and say what it is.
fn write_font(
    session: &mut Session,
    sheet: u64,
    cell: CellRef,
    font: Font,
    id: &str,
) -> Result<Outcome, EngineError> {
    let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
    crate::protection::refuse_format(node, id)?;
    let mut cells = crate::sheet_cells_mut(node);
    cells.set_font(cell, Some(font.clone()));
    node.data["cells"] = crate::cells_into(&cells);
    Ok(Outcome::changed(format!("the font of the cell {} is set", cell.to_text())).with_data(json!({
        "sheet": sheet,
        "cell": cell.to_text(),
        "name": match &font.name {
            Some(name) => name.clone(),
            None => String::new(),
        },
        "size": font.size,
        "bold": font.bold,
        "italic": font.italic,
        "strike": font.strike,
        "underline": font.underline.to_text(),
        "vert": font.vert.to_text(),
    })))
}

/// **Set** the face, the size, the underline or the place of the text
/// of a cell.
///
/// The tree's Font, Font Size, Underline and the two places of the
/// text.
#[derive(Command)]
#[command(id = "grid.cell.font", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetFont {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The **name** of the face.
    pub name: Option<Text>,
    /// The **size** of the text.
    pub size: Option<Number>,
    /// The **underline**: none, single, double, single accounting or
    /// double accounting.
    pub underline: Option<Text>,
    /// Where the text sits **above or below** its line: baseline,
    /// superscript or subscript.
    pub vert: Option<Text>,
}

impl SetFont {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.font".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        if self.name.is_none() && self.size.is_none() && self.underline.is_none() && self.vert.is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "name what to set: the face, the size, the underline or the place of the text"
                    .to_string(),
            });
        }
        let underline = match &self.underline {
            Some(underline) => Some(Underline::parse(&underline.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not an underline: it is none, single, double, single accounting or double accounting",
                    underline.0
                ),
            })?),
            None => None,
        };
        let vert = match &self.vert {
            Some(vert) => Some(Vert::parse(&vert.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not a place for the text: it is baseline, superscript or subscript",
                    vert.0
                ),
            })?),
            None => None,
        };
        let name = match &self.name {
            Some(name) => {
                let name = name.0.trim().to_string();
                if name.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a face has a name".to_string(),
                    });
                }
                Some(name)
            }
            None => None,
        };
        let size = match self.size {
            Some(size) => {
                let number = size.0;
                if !number.is_finite() || number < MIN_SIZE || number > MAX_SIZE {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the size of a font is from {MIN_SIZE} to {MAX_SIZE}, and {number} is not one of them"
                        ),
                    });
                }
                // The size is **rounded** to the nearest half: the
                // tree's own rule.
                Some((number * 2.0).round() / 2.0)
            }
            None => None,
        };
        let mut font = font_of(session, sheet, cell);
        if let Some(underline) = underline {
            // Naming the underline a cell already has takes it away:
            // the tree's own rule.
            font.underline = if underline == Underline::None || font.underline == underline {
                Underline::None
            } else {
                underline
            };
        }
        if let Some(vert) = vert {
            // Naming the place a cell already has takes it back to the
            // baseline: the tree's own rule.
            font.vert = match vert {
                Vert::Baseline => Vert::Baseline,
                other => {
                    if font.vert == other {
                        Vert::Baseline
                    } else {
                        other
                    }
                }
            };
        }
        if let Some(name) = name {
            font.name = Some(name);
        }
        if let Some(size) = size {
            font.size = Some(size);
        }
        write_font(session, sheet, cell, font, &id)
    }
}

/// **Bold** the text of a cell, or take the bold away.
///
/// The tree's Bold: a button, and a button pressed twice does the
/// opposite.
#[derive(Command)]
#[command(id = "grid.cell.font.bold", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetBold {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// Whether the text is bold: the opposite of what it is when this
    /// is not named.
    pub on: Option<Bool>,
}

impl SetBold {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.font.bold".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let mut font = font_of(session, sheet, cell);
        font.bold = match self.on {
            Some(on) => on.0,
            None => !font.bold,
        };
        write_font(session, sheet, cell, font, &id)
    }
}

/// **Italic** the text of a cell, or take the italic away.
///
/// The tree's Italic: a button, and a button pressed twice does the
/// opposite.
#[derive(Command)]
#[command(id = "grid.cell.font.italic", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetItalic {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// Whether the text is italic: the opposite of what it is when
    /// this is not named.
    pub on: Option<Bool>,
}

impl SetItalic {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.font.italic".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let mut font = font_of(session, sheet, cell);
        font.italic = match self.on {
            Some(on) => on.0,
            None => !font.italic,
        };
        write_font(session, sheet, cell, font, &id)
    }
}

/// **Strike** the text of a cell through, or take the strike away.
///
/// The tree's Strikethrough: a button, and a button pressed twice does
/// the opposite.
#[derive(Command)]
#[command(id = "grid.cell.font.strike", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetStrike {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// Whether the text is struck through: the opposite of what it is
    /// when this is not named.
    pub on: Option<Bool>,
}

impl SetStrike {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.font.strike".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let mut font = font_of(session, sheet, cell);
        font.strike = match self.on {
            Some(on) => on.0,
            None => !font.strike,
        };
        write_font(session, sheet, cell, font, &id)
    }
}
