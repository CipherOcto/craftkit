//! The **number format** of a cell.
//!
//! The tree's format is a whole language; this tree has a subset of
//! it, and a format that is not one of the subset is refused.

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

/// A number is written with a **fixed number of decimals**.
#[test]
fn a_number_is_written_with_its_decimals() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.2345 })).expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.23", "the number has two decimals");
}

/// The thousands are **separated**.
#[test]
fn the_thousands_are_separated() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1234567 }))
        .expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "#,##0" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1,234,567", "the thousands are separated");
}

/// A percentage is **multiplied by a hundred**.
#[test]
fn a_percentage_is_multiplied_by_a_hundred() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0.25 })).expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00%" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "25.00%", "the value is a percentage");
}

/// A number is **rounded away from zero**, as the tree rounds it.
#[test]
fn a_number_is_rounded_away_from_zero() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0.125 })).expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "0.13", "the number is rounded away from zero");
}

/// A cell with **no format** shows its value as it is.
#[test]
fn a_cell_with_no_format_shows_its_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "7", "the value is as it is");
}

/// A **text** is not changed by a format.
#[test]
fn a_text_is_not_changed_by_a_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "on backorder" }))
        .expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "on backorder", "the text is the text");
}

/// A **formula** cell shows the value the formula computes.
#[test]
fn a_formula_cell_shows_what_it_computes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 2.5 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=B1*2" }))
        .expect("the formula is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.00", "the formula is computed and written");
}

/// A format that is **not one of ours** is refused.
#[test]
fn a_format_that_is_not_one_of_ours_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "yyyy-mm-dd" }))
        .expect_err("the format is refused");
    assert!(error.to_string().contains("is not a format this tree has"), "the error says why: {error}");
}

/// A format that is **too long** is refused: the tree's own bound.
#[test]
fn a_format_that_is_too_long_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let long = "0".repeat(300);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": long }))
        .expect_err("the format is refused");
    assert!(error.to_string().contains("at most 255 characters"), "the error says why: {error}");
}

/// A cell may be formatted **before it has a value**.
#[test]
fn a_cell_may_be_formatted_before_it_has_a_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "", "the cell shows nothing");
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.5 })).expect("set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "and the format is still there");
}

/// The fill copies the **format** of the source, as the tree copies the
/// style.
#[test]
fn the_fill_copies_the_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2", "mode": "copy" }))
        .expect("the range is filled");
    assert_eq!(text_of(&mut session, sheet, "A2"), "7.00", "the cell that was filled has the format");
}

/// A sheet that is **not there** is refused.
#[test]
fn a_sheet_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": 1_000_000, "cell": "A1", "format": "0.00" }))
        .expect_err("the format is refused");
    assert!(error.to_string().contains("there is no sheet"), "the error says why: {error}");
}
