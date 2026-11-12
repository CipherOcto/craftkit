//! The **sign** of a number that rounds away.
//!
//! A negative number that rounds to zero is written without its sign:
//! the sign of a number that is not there is not written.

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
fn a_cell(session: &mut Session, sheet: u64, value: f64, format: &str) -> String {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": value }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": format }))
        .expect("the format is set");
    text_of(session, sheet, "A1")
}

/// A negative number that **rounds to zero** shows no sign.
#[test]
fn a_negative_number_that_rounds_to_zero_shows_no_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(a_cell(&mut session, sheet, -0.0033, "0.00"), "0.00", "the sign is not there");
}

/// A negative number that **rounds away from zero** shows its sign.
#[test]
fn a_negative_number_that_rounds_away_from_zero_shows_its_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(a_cell(&mut session, sheet, -0.005, "0.00"), "-0.01", "the sign is there");
}

/// A whole number that rounds to zero shows no sign.
#[test]
fn a_whole_number_that_rounds_to_zero_shows_no_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(a_cell(&mut session, sheet, -0.4, "0"), "0", "the sign is not there");
}

/// A whole number that rounds away shows its sign.
#[test]
fn a_whole_number_that_rounds_away_shows_its_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(a_cell(&mut session, sheet, -0.5, "0"), "-1", "the sign is there");
}

/// A negative number that is **not** near zero keeps its sign.
#[test]
fn a_negative_number_that_is_not_near_zero_keeps_its_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(a_cell(&mut session, sheet, -2.5, "0.00"), "-2.50", "the sign is there");
}

/// A **currency** that rounds to zero shows no sign either.
#[test]
fn a_currency_that_rounds_to_zero_shows_no_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(a_cell(&mut session, sheet, -0.0033, "\"$\"0.00"), "$0.00", "the sign is not there");
}
