//! The **marks** of a print: the gridlines and the headings.
//!
//! A printed page may carry the **gridlines** — the lines between the
//! cells — and the **headings**, which are the row numbers and the
//! column letters beside them.
//!
//! The headings are not free: they take room from the page, so a sheet
//! that is printed with its headings has fewer rows and fewer columns
//! on a page than one printed without them. The room they take is the
//! tree's own: the width of a column of headings is ten points plus
//! seven for every digit of the last row, and their height is the
//! height of a row.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session};

use crate::printing::sheet_of;

/// The **room** the headings take at the side of the page, in points:
/// the tree's own.
pub const HEADING_MARGIN: f64 = 10.0;

/// The **width** of a digit of a heading, in points: the tree's own.
pub const HEADING_DIGIT: f64 = 7.0;

/// Whether the **gridlines** are printed.
pub fn gridlines_of(node: &Node) -> bool {
    node.data.get("print_gridlines").and_then(|flag| flag.as_bool()).unwrap_or(false)
}

/// Whether the **headings** are printed.
pub fn headings_of(node: &Node) -> bool {
    node.data.get("print_headings").and_then(|flag| flag.as_bool()).unwrap_or(false)
}

/// The **room** the headings take from the page, in points: the width
/// they take from the columns and the height they take from the rows.
///
/// A sheet that does not print its headings takes no room.
pub fn room_of(node: &Node, digits: u32) -> (f64, f64) {
    if !headings_of(node) {
        return (0.0, 0.0);
    }
    let height = crate::printing::DEFAULT_ROW_HEIGHT.clamp(10.0, 60.0);
    (HEADING_MARGIN + HEADING_DIGIT * digits as f64, height)
}

commands! {
    /// **Print** the gridlines of a sheet, or do not.
    grid.print.gridlines {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// Whether the gridlines are printed.

            on: Bool,
        },
        run: {

            set(session, "grid.print.gridlines", "print_gridlines", self.sheet.0, self.on.0)

        }
    }
    /// **Print** the headings of a sheet, or do not.
    ///
    /// The headings take room from the page, so the pages of a sheet with
    /// its headings are not the pages of a sheet without them.
    grid.print.headings {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// Whether the headings are printed.

            on: Bool,
        },
        run: {

            set(session, "grid.print.headings", "print_headings", self.sheet.0, self.on.0)

        }
    }
}

/// **Set** a mark of a print.
fn set(session: &mut Session, id: &str, name: &str, sheet: u64, on: bool) -> Result<Outcome, EngineError> {
    sheet_of(session, id, sheet)?;
    if let Some(node) = session.document_mut()?.node_mut(sheet) {
        node.data[name] = json!(on);
    }
    Ok(Outcome::changed(format!("the {name} of the sheet is set")).with_data(json!({
        "sheet": sheet,
        "on": on,
    })))
}
