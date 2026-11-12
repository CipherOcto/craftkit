//! The **area** a sheet prints and the **breaks** in it.

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

/// The **pages** of a sheet.
fn pages_of(session: &mut Session, sheet: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.print.pages", &json!({ "sheet": sheet })).expect("the pages are read");
    outcome.data["pages"].as_array().expect("the pages are a list").clone()
}

/// The **setup** of a sheet.
fn setup_of(session: &mut Session, sheet: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the setup is read");
    outcome.data.clone()
}

/// **The area that is printed may be set.**
#[test]
fn the_area_that_is_printed_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.print.area", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the area is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["area"], json!("A1:B2"));
}

/// **The area of one cell is refused.**
#[test]
fn the_area_of_one_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.area", &json!({ "sheet": sheet, "range": "A1" }))
        .expect_err("the area is refused");
    let message = failure.to_string();
    assert!(message.contains("an area of one cell is a cell"), "the refusal says why: {message}");
}

/// **The pages are of the area that is printed.**
#[test]
fn the_pages_are_of_the_area() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        a_number(&mut session, sheet, &format!("{column}1"), 1.0);
    }
    session
        .execute("grid.print.area", &json!({ "sheet": sheet, "range": "A1:C1" }))
        .expect("the area is set");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 1, "the area is one page wide: {pages:?}");
    assert_eq!(pages[0]["columns"], json!(["A", "C"]));
}

/// **The area may be taken away, and the sheet prints what it uses.**
#[test]
fn the_area_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        a_number(&mut session, sheet, &format!("{column}1"), 1.0);
    }
    session
        .execute("grid.print.area", &json!({ "sheet": sheet, "range": "A1:C1" }))
        .expect("the area is set");
    session.execute("grid.print.area", &json!({ "sheet": sheet })).expect("the area is taken away");
    let setup = setup_of(&mut session, sheet);
    assert!(setup["area"].is_null(), "and there is no area: {setup:?}");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the sheet prints what it uses: {pages:?}");
}

/// **A break is a place where a page begins.**
#[test]
fn a_break_is_a_place_where_a_page_begins() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=20 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session.execute("grid.print.break", &json!({ "sheet": sheet, "row": 10 })).expect("the break is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["row_breaks"], json!([10]));
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the break makes two pages: {pages:?}");
    assert_eq!(pages[0]["rows"], json!([1, 9]));
    assert_eq!(pages[1]["rows"], json!([10, 20]));
}

/// **A break may be taken away.**
#[test]
fn a_break_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for row in 1..=20 {
        a_number(&mut session, sheet, &format!("A{row}"), 1.0);
    }
    session.execute("grid.print.break", &json!({ "sheet": sheet, "row": 10 })).expect("the break is set");
    session
        .execute("grid.print.break", &json!({ "sheet": sheet, "row": 10, "clear": true }))
        .expect("the break is taken away");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["row_breaks"], json!([]));
}

/// **A break is a row or a column and not both.**
#[test]
fn a_break_is_a_row_or_a_column() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute("grid.print.break", &json!({ "sheet": sheet, "row": 10, "column": "A" }))
        .expect_err("the break is refused");
    let message = failure.to_string();
    assert!(message.contains("a row or a column and not both"), "the refusal says why: {message}");
}

/// **A break that is neither is refused.**
#[test]
fn a_break_that_is_neither_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure =
        session.execute("grid.print.break", &json!({ "sheet": sheet })).expect_err("the break is refused");
    let message = failure.to_string();
    assert!(message.contains("a break is a row or a column"), "the refusal says why: {message}");
}

/// **A column break breaks the columns.**
#[test]
fn a_column_break_breaks_the_columns() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for column in ["A", "B", "C"] {
        a_number(&mut session, sheet, &format!("{column}1"), 1.0);
    }
    session.execute("grid.print.break", &json!({ "sheet": sheet, "column": "C" })).expect("the break is set");
    let setup = setup_of(&mut session, sheet);
    assert_eq!(setup["column_breaks"], json!(["C"]));
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages.len(), 2, "the break makes two pages: {pages:?}");
    assert_eq!(pages[0]["columns"], json!(["A", "B"]));
    assert_eq!(pages[1]["columns"], json!(["C", "C"]));
}
