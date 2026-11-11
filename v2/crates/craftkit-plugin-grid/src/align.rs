//! The **alignment** of a cell: how its text lies across it and how it
//! lies down it, and how far it is indented and whether it wraps.
//!
//! The tree's rules:
//!
//! - the **horizontal** alignment is general, left, center, right,
//!   fill, justify or center-across: the tree's own names;
//! - the **vertical** alignment is top, middle or bottom: the tree's
//!   own names;
//! - **naming the alignment a cell already has returns it to general**
//!   — the tree's own rule: clicking the alignment that is active
//!   again is clicking the general one;
//! - the **indent** is from zero to two hundred and fifty: the tree
//!   clamps it, and here it is refused;
//! - an **indent** above zero makes the horizontal alignment **left**
//!   when it is general, center, fill, justify or center-across: the
//!   tree's own rule, because an indented text is a text that starts
//!   at the left;
//! - the **wrap** is a yes or a no.
//!
//! # What the port found
//!
//! - **The tree's alignment is a style and this one's is the cell's
//!   own** (§271): the alignment of a cell is the cell's own, as its
//!   format is, and a hundred cells with one alignment carry a
//!   hundred alignments.
//! - **The tree's alignment is a range and this one's is a cell**:
//!   the tree's button aligns the selection, and here the cell is
//!   named.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{Align, AlignH, AlignV};

/// The most **indent** a cell may have: the tree's own bound.
const MAX_INDENT: u8 = 250;

/// **Align** a cell: how its text lies across it and how it lies down
/// it, and how far it is indented and whether it wraps.
#[derive(Command)]
#[command(id = "grid.cell.align", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetAlign {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
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

impl SetAlign {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.align".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        if self.horizontal.is_none()
            && self.vertical.is_none()
            && self.indent.is_none()
            && self.wrap.is_none()
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "name what to align: the alignment across the cell, down it, the indent or the wrap"
                    .to_string(),
            });
        }
        let horizontal = match &self.horizontal {
            Some(horizontal) => Some(AlignH::parse(&horizontal.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not an alignment: it is general, left, center, right, fill, justify or center-across",
                    horizontal.0
                ),
            })?),
            None => None,
        };
        let vertical = match &self.vertical {
            Some(vertical) => Some(AlignV::parse(&vertical.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not an alignment down the cell: it is top, middle or bottom",
                    vertical.0
                ),
            })?),
            None => None,
        };
        let indent = match self.indent {
            Some(indent) => {
                let number = indent.0;
                if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > MAX_INDENT as f64
                {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the indent of a cell is from zero to {MAX_INDENT}, and {number} is not one of them"
                        ),
                    });
                }
                Some(number as u8)
            }
            None => None,
        };
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
        crate::protection::refuse_format(node, &id)?;
        let cells = crate::cells_of(node);
        let mut align = cells.align(cell).copied().unwrap_or_else(Align::plain);
        if let Some(horizontal) = horizontal {
            // **Naming the alignment a cell already has returns it to
            // general**: the tree's own rule.
            align.horizontal = if align.horizontal == horizontal { AlignH::General } else { horizontal };
        }
        if let Some(vertical) = vertical {
            align.vertical = vertical;
        }
        if let Some(indent) = indent {
            align.indent = indent;
            // An indented text is a text that starts at the left: the
            // tree's own rule.
            if indent > 0
                && matches!(
                    align.horizontal,
                    AlignH::General | AlignH::Center | AlignH::Fill | AlignH::Justify | AlignH::CenterAcross
                )
            {
                align.horizontal = AlignH::Left;
            }
        }
        if let Some(wrap) = self.wrap {
            align.wrap = wrap.0;
        }
        let mut cells = crate::sheet_cells_mut(node);
        cells.set_align(cell, Some(align));
        node.data["cells"] = crate::cells_into(&cells);
        Ok(Outcome::changed(format!("the cell {} is aligned", cell.to_text())).with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "horizontal": align.horizontal.to_text(),
            "vertical": align.vertical.to_text(),
            "indent": align.indent,
            "wrap": align.wrap,
        })))
    }
}
