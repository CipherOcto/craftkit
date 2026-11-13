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

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

use crate::formula::{Align, AlignH, AlignV};

/// The most **indent** a cell may have: the tree's own bound.
const MAX_INDENT: u8 = 250;

/// The **parts** of an alignment: what the parameters name.
///
/// The tree's alignment is a style's, and a cell here carries one of
/// its own, so the parts are read once and are used by both.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AlignParts {
    /// How the text lies **across** the cell.
    pub horizontal: Option<AlignH>,
    /// How the text lies **down** the cell.
    pub vertical: Option<AlignV>,
    /// How far the text is indented.
    pub indent: Option<u8>,
    /// Whether the text wraps.
    pub wrap: Option<bool>,
}

impl AlignParts {
    /// The parts the parameters name, and the error that says they do
    /// not.
    pub(crate) fn parse(
        id: &str,
        horizontal: Option<&Text>,
        vertical: Option<&Text>,
        indent: Option<Number>,
        wrap: Option<Bool>,
    ) -> Result<AlignParts, EngineError> {
        let horizontal = match horizontal {
            Some(horizontal) => Some(AlignH::parse(&horizontal.0).ok_or_else(|| EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "`{}` is not an alignment: it is general, left, center, right, fill, justify or center-across",
                    horizontal.0
                ),
            })?),
            None => None,
        };
        let vertical = match vertical {
            Some(vertical) => Some(AlignV::parse(&vertical.0).ok_or_else(|| EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "`{}` is not an alignment down the cell: it is top, middle or bottom",
                    vertical.0
                ),
            })?),
            None => None,
        };
        let indent = match indent {
            Some(indent) => {
                let number = indent.0;
                if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > MAX_INDENT as f64
                {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: format!(
                            "the indent of a cell is from zero to {MAX_INDENT}, and {number} is not one of them"
                        ),
                    });
                }
                Some(number as u8)
            }
            None => None,
        };
        let wrap = wrap.map(|wrap| wrap.0);
        Ok(AlignParts { horizontal, vertical, indent, wrap })
    }

    /// Whether the parts say **nothing** at all.
    pub(crate) fn is_empty(&self) -> bool {
        self.horizontal.is_none() && self.vertical.is_none() && self.indent.is_none() && self.wrap.is_none()
    }

    /// The alignment these parts make of the one that is there.
    ///
    /// The tree's own rules: naming the alignment that is there takes
    /// it back to general, and an indented text lies at the left.
    pub(crate) fn apply(&self, mut align: Align) -> Align {
        if let Some(horizontal) = self.horizontal {
            // **Naming the alignment a cell already has returns it to
            // general**: the tree's own rule.
            align.horizontal = if align.horizontal == horizontal { AlignH::General } else { horizontal };
        }
        if let Some(vertical) = self.vertical {
            align.vertical = vertical;
        }
        if let Some(indent) = self.indent {
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
            align.wrap = wrap;
        }
        align
    }
}

commands! {
    /// **Align** a cell: how its text lies across it and how it lies down
    /// it, and how far it is indented and whether it wraps.
    grid.cell.align {
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
            /// How the text lies **across** the cell: general, left, center,
            /// right, fill, justify or center-across.
            horizontal: Option<Text>,
            /// How the text lies **down** the cell: top, middle or bottom.
            vertical: Option<Text>,
            /// How far the text is indented.
            indent: Option<Number>,
            /// Whether the text wraps.
            wrap: Option<Bool>,
        },
        run: {
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
            let parts =
                AlignParts::parse(&id, self.horizontal.as_ref(), self.vertical.as_ref(), self.indent, self.wrap)?;
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
            let align = parts.apply(cells.align(cell).copied().unwrap_or_else(Align::plain));
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
}
