//! The **accounting** and the **comma** formats: the two that fill the
//! cell.
//!
//! The tree's own: the symbol at the edge of the cell, the number at
//! the other edge, and the space between them.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the grid battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A number in a cell.
fn set(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **format** of a cell.
fn format(session: &mut Session, sheet: u64, cell: &str, format: &str) {
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": format }))
        .expect("the format is set");
}

/// The **width** of a column, in characters.
fn width(session: &mut Session, sheet: u64, column: &str, width: f64) {
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": column, "width": width }))
        .expect("the width is set");
}

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the text is read");
    outcome.data["text"].as_str().expect("the text is a string").to_string()
}

/// The **accounting** format puts the symbol at the left and the number
/// at the right.
#[test]
fn the_accounting_format_fills_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    width(&mut session, sheet, "A", 14.0);
    set(&mut session, sheet, "A1", 1234.5);
    format(&mut session, sheet, "A1", "accounting");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$    1,234.50 ", "the cell is filled");
}

/// The **comma** format separates the thousands and fills the cell.
#[test]
fn the_comma_format_fills_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    width(&mut session, sheet, "A", 14.0);
    set(&mut session, sheet, "A1", 1234.5);
    format(&mut session, sheet, "A1", "comma");
    assert_eq!(text_of(&mut session, sheet, "A1"), "     1,234.50 ", "the cell is filled");
}

/// A **negative** number is written in parentheses: the tree's own rule.
#[test]
fn a_negative_number_is_written_in_parentheses() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    width(&mut session, sheet, "A", 14.0);
    set(&mut session, sheet, "A1", -1234.5);
    format(&mut session, sheet, "A1", "accounting");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$  (1,234.50) ", "the number is in brackets");
}

/// A **zero** is written as a dash: the tree's own rule.
#[test]
fn a_zero_is_written_as_a_dash() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    width(&mut session, sheet, "A", 14.0);
    set(&mut session, sheet, "A1", 0.0);
    format(&mut session, sheet, "A1", "accounting");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$           - ", "the zero is a dash");
}

/// The **symbol** may be chosen.
#[test]
fn the_symbol_may_be_chosen() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    width(&mut session, sheet, "A", 14.0);
    set(&mut session, sheet, "A1", 1234.5);
    format(&mut session, sheet, "A1", "accounting \"L\"");
    assert_eq!(
        text_of(&mut session, sheet, "A1"),
        "L    1,234.50 ",
        "the symbol is the one that was asked for"
    );
}

/// A column with **no width** has no fill.
#[test]
fn a_column_with_no_width_has_no_fill() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set(&mut session, sheet, "A1", 1234.5);
    format(&mut session, sheet, "A1", "accounting");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$ 1,234.50", "the text is written as it is");
}

/// A text that is **longer** than the column is written as it is.
#[test]
fn a_text_longer_than_the_column_is_written_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    width(&mut session, sheet, "A", 6.0);
    set(&mut session, sheet, "A1", 1234.5);
    format(&mut session, sheet, "A1", "accounting");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$ 1,234.50", "the text does not fit and is not cut");
}
