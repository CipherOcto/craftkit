//! The **titles** of a sheet: the rows and the columns that are
//! printed on every page.

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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **setup** of a sheet.
fn setup_of(session: &mut Session, sheet: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the setup is read");
    outcome.data.clone()
}

/// The **pages** of a sheet.
fn pages_of(session: &mut Session, sheet: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.print.pages", &json!({ "sheet": sheet })).expect("the pages are read");
    outcome.data["pages"].as_array().expect("the pages are a list").clone()
}

/// **The titles may be set, and they are read back.**
#[test]
fn the_titles_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "rows": "1:2", "columns": "A:B" }))
        .expect("the titles are set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["title_rows"], json!([1, 2]));
    assert_eq!(setup["title_columns"], json!(["A", "B"]));
}

/// **The titles are printed on every page.**
#[test]
fn the_titles_are_printed_on_every_page() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=60 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "rows": "1:2" }))
        .expect("the titles are set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the sheet is two pages: {pages:?}");
    assert_eq!(pages[0]["title_rows"], json!([1, 2]));
    assert_eq!(pages[1]["title_rows"], json!([1, 2]), "and the second page has them too");
}

/// **The titles are not part of the pages themselves.**
#[test]
fn the_titles_are_not_part_of_the_pages() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=60 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "rows": "1:2" }))
        .expect("the titles are set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages[0]["rows"], json!([3, 47]), "the pages begin after the titles: {pages:?}");
}

/// **The title columns are left out of the pages too.**
#[test]
fn the_title_columns_are_left_out() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        a_number(&mut session, sheet, &format!("{column}1"), 1.0);
    }
    session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "columns": "A:B" }))
        .expect("the titles are set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages[0]["columns"], json!(["C", "H"]), "the pages begin after the titles: {pages:?}");
    assert_eq!(pages[0]["title_columns"], json!(["A", "B"]));
}

/// **A range that is not a range of rows is refused.**
#[test]
fn a_range_that_is_not_a_range_of_rows_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "rows": "1" }))
        .expect_err("the titles are refused");
    let message = failure.to_string();
    assert!(message.contains("is not a range of rows"), "the refusal says why: {message}");
}

/// **A row that is not a row is refused.**
#[test]
fn a_row_that_is_not_a_row_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "rows": "one:two" }))
        .expect_err("the titles are refused");
    let message = failure.to_string();
    assert!(message.contains("is not a row"), "the refusal says why: {message}");
}

/// **A column that is not a column is refused.**
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "columns": "one" }))
        .expect_err("the titles are refused");
    let message = failure.to_string();
    assert!(message.contains("is not a range of columns"), "the refusal says why: {message}");
}
