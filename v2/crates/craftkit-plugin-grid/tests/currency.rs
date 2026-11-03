//! The **currency** of a cell.
//!
//! A currency is a number with a symbol on it. The tree writes the
//! symbol in quotes, because a symbol is a literal and not a
//! placeholder.

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

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the text is read");
    outcome.data["text"].as_str().expect("the text is a string").to_string()
}

/// A cell with a **value** and a **format**.
fn a_cell(session: &mut Session, sheet: u64, cell: &str, value: f64, format: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": format }))
        .expect("the format is set");
}

/// A currency is the **symbol** and the number.
#[test]
fn a_currency_is_the_symbol_and_the_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1234.5, "\"$\"#,##0.00");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$1,234.50", "the symbol is in front of the number");
}

/// The symbol may be **behind** the number.
#[test]
fn the_symbol_may_be_behind_the_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1234.5, "#,##0.00\" kr\"");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1,234.50 kr", "the symbol is behind the number");
}

/// A currency may have **no decimals**.
#[test]
fn a_currency_may_have_no_decimals() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1234.5, "\"$\"#,##0");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$1,235", "the number is whole");
}

/// A **negative** currency keeps its sign, in front of the symbol.
#[test]
fn a_negative_currency_keeps_its_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -5.0, "\"$\"#,##0.00");
    assert_eq!(text_of(&mut session, sheet, "A1"), "-$5.00", "the sign is in front of the symbol");
}

/// The thousands of a currency are **separated**.
#[test]
fn the_thousands_of_a_currency_are_separated() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1234567.0, "\"$\"#,##0");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$1,234,567", "the thousands are separated");
}

/// A symbol with **no number** is refused: a currency is a number with a
/// symbol on it.
#[test]
fn a_symbol_with_no_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect("the value is set");
    let outcome =
        session.execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "\"$\"" }));
    assert!(outcome.is_err(), "there is no number in the format");
}

/// A symbol with a number that is **not** a number is refused.
#[test]
fn a_symbol_with_a_number_that_is_not_a_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect("the value is set");
    let outcome =
        session.execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "\"$\"abc" }));
    assert!(outcome.is_err(), "the number is refused");
}
