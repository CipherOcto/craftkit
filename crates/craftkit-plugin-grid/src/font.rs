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

use craftkit_derive::commands;

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

commands! {
    /// **Set** the face, the size, the underline or the place of the text
    /// of a cell.
    ///
    /// The tree's Font, Font Size, Underline and the two places of the
    /// text.
    grid.cell.font {
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
            /// The **name** of the face.

            name: Option<Text>,
            /// The **size** of the text.

            size: Option<Number>,
            /// The **underline**: none, single, double, single accounting or
            /// double accounting.

            underline: Option<Text>,
            /// Where the text sits **above or below** its line: baseline,
            /// superscript or subscript.

            vert: Option<Text>,
        },
        run: {

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
            let parts = FontParts::parse(
                &id,
                self.name.as_ref(),
                self.size,
                self.underline.as_ref(),
                self.vert.as_ref(),
            )?;
            let font = font_of(session, sheet, cell);
            let font = parts.apply(font);
            write_font(session, sheet, cell, font, &id)

        }
    }
    /// **Bold** the text of a cell, or take the bold away.
    ///
    /// The tree's Bold: a button, and a button pressed twice does the
    /// opposite.
    grid.cell.font.bold {
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
            /// Whether the text is bold: the opposite of what it is when this
            /// is not named.

            on: Option<Bool>,
        },
        run: {

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
    grid.cell.font.italic {
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
            /// Whether the text is italic: the opposite of what it is when
            /// this is not named.

            on: Option<Bool>,
        },
        run: {

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
    grid.cell.font.strike {
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
            /// Whether the text is struck through: the opposite of what it is
            /// when this is not named.

            on: Option<Bool>,
        },
        run: {

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
}

/// The **parts** of a font: what the parameters name.
///
/// The tree's font is a style's, and a cell here carries one of its
/// own, so the parts are read once and are used by both.
#[derive(Debug, Clone, Default)]
pub(crate) struct FontParts {
    /// The **name** of the face.
    pub name: Option<String>,
    /// The **size** of the text.
    pub size: Option<f64>,
    /// The **underline**.
    pub underline: Option<Underline>,
    /// Where the text sits **above or below** its line.
    pub vert: Option<Vert>,
}

impl FontParts {
    /// The parts the parameters name, and the error that says they do
    /// not.
    pub(crate) fn parse(
        id: &str,
        name: Option<&Text>,
        size: Option<Number>,
        underline: Option<&Text>,
        vert: Option<&Text>,
    ) -> Result<FontParts, EngineError> {
        let underline = match underline {
            Some(underline) => Some(Underline::parse(&underline.0).ok_or_else(|| EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "`{}` is not an underline: it is none, single, double, single accounting or double accounting",
                    underline.0
                ),
            })?),
            None => None,
        };
        let vert = match vert {
            Some(vert) => Some(Vert::parse(&vert.0).ok_or_else(|| EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "`{}` is not a place for the text: it is baseline, superscript or subscript",
                    vert.0
                ),
            })?),
            None => None,
        };
        let name = match name {
            Some(name) => {
                let name = name.0.trim().to_string();
                if name.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: "a face has a name".to_string(),
                    });
                }
                Some(name)
            }
            None => None,
        };
        let size = match size {
            Some(size) => {
                let number = size.0;
                if !number.is_finite() || !(MIN_SIZE..=MAX_SIZE).contains(&number) {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
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
        Ok(FontParts { name, size, underline, vert })
    }

    /// Whether the parts say **nothing** at all.
    pub(crate) fn is_empty(&self) -> bool {
        self.name.is_none() && self.size.is_none() && self.underline.is_none() && self.vert.is_none()
    }

    /// The font these parts make of the one that is there.
    ///
    /// The tree's own rules: naming the underline that is there takes
    /// it away, and naming the place that is there takes the text back
    /// to the baseline.
    pub(crate) fn apply(&self, mut font: Font) -> Font {
        if let Some(underline) = self.underline {
            // Naming the underline a cell already has takes it away:
            // the tree's own rule.
            font.underline = if underline == Underline::None || font.underline == underline {
                Underline::None
            } else {
                underline
            };
        }
        if let Some(vert) = self.vert {
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
        if let Some(name) = &self.name {
            font.name = Some(name.clone());
        }
        if let Some(size) = self.size {
            font.size = Some(size);
        }
        font
    }
}
