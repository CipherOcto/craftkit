//! The **colours** of a cell: the colour of its text and the colour
//! behind it.
//!
//! The tree's rules:
//!
//! - a colour is `#rrggbb` — six digits of hex, and the tree's own
//!   rule for a colour of this tree;
//! - **none** takes the colour away, and the text takes the colour of
//!   the sheet: the tree's own rule, and the tree's colour may also
//!   be a theme colour with a tint, which this tree has not.
//!
//! # What the port found
//!
//! - **The tree's fill and its font colour are in the style of the
//!   cell, and here they are the cell's own**: as the format, the
//!   alignment and the font are (§816, §819).
//! - **The tree's colour may be a theme colour with a tint**, and this
//!   tree has no theme: a colour is six digits of hex or it is
//!   refused.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

/// The **colour** of a name: `#rrggbb`, or **none** to take the colour
/// away.
///
/// `None` when the name is not a colour.
pub(crate) fn colour_of(given: &str) -> Option<Option<String>> {
    let text = given.trim();
    if text.eq_ignore_ascii_case("none") {
        return Some(None);
    }
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    Some(Some(format!("#{}", hex.to_ascii_lowercase())))
}

/// The **channels** of a colour: the red, the green and the blue, each
/// of them from 0 to 255.
///
/// A colour that is not `#rrggbb` has no channels.
pub(crate) fn channels_of(given: &str) -> Option<(u8, u8, u8)> {
    let text = given.trim();
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((red, green, blue))
}

/// The **mix** of two colours: the colour that lies between them, in
/// the proportion given.
///
/// The proportion is from zero to one, and each channel of the mix is
/// the channel of the first colour plus that part of the way to the
/// channel of the second.
pub(crate) fn mix_of(first: (u8, u8, u8), second: (u8, u8, u8), part: f64) -> String {
    let channel = |one: u8, other: u8| -> u8 {
        let value = f64::from(one) + (f64::from(other) - f64::from(one)) * part;
        value.round().clamp(0.0, 255.0) as u8
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(first.0, second.0),
        channel(first.1, second.1),
        channel(first.2, second.2)
    )
}

commands! {
    /// **Colour** the text of a cell.
    ///
    /// The tree's Font Color: the text of the cell takes a colour, and
    /// **none** takes it away.
    grid.cell.font.colour {
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
            /// The colour: `#rrggbb`, or `none` to take it away.
            colour: Text,
        },
        run: {
            let id = "grid.cell.font.colour".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let Some(colour) = colour_of(&self.colour.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a colour: a colour is `#rrggbb`, or `none`",
                        self.colour.0.trim()
                    ),
                });
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
            let mut font = cells.font(cell).cloned().unwrap_or_else(crate::formula::Font::plain);
            font.colour = colour.clone();
            let mut cells = crate::sheet_cells_mut(node);
            cells.set_font(cell, Some(font));
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("the text of the cell {} is coloured", cell.to_text())).with_data(
                json!({
                    "sheet": sheet,
                    "cell": cell.to_text(),
                    "colour": match &colour {
                        Some(colour) => colour.clone(),
                        None => String::new(),
                    },
                }),
            ))

        }
    }

    /// **Fill** a cell with a colour.
    ///
    /// The tree's Fill Color: the colour behind the text of the cell, and
    /// **none** takes it away.
    grid.cell.fill {
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
            /// The colour: `#rrggbb`, or `none` to take it away.
            colour: Text,
        },
        run: {
            let id = "grid.cell.fill".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let Some(colour) = colour_of(&self.colour.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a colour: a colour is `#rrggbb`, or `none`",
                        self.colour.0.trim()
                    ),
                });
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
            let mut cells = crate::sheet_cells_mut(node);
            cells.set_fill(cell, colour.clone());
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("the cell {} is filled", cell.to_text())).with_data(json!({
                "sheet": sheet,
                "cell": cell.to_text(),
                "colour": match &colour {
                    Some(colour) => colour.clone(),
                    None => String::new(),
                },
            })))

        }
    }
}
