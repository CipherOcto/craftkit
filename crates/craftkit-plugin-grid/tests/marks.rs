//! The **marks** of a print: the gridlines and the headings.

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

/// A column of **numbers**, one in each row from one.
fn a_column(session: &mut Session, sheet: u64, rows: u32) {
    for row in 1..=rows {
        a_number(session, sheet, &format!("A{row}"), f64::from(row));
    }
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

/// **The gridlines may be printed, and the fact is read back.**
#[test]
fn the_gridlines_may_be_printed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(setup_of(&mut session, sheet)["gridlines"], json!(false), "the tree prints none");
    session
        .execute("grid.print.gridlines", &json!({ "sheet": sheet, "on": true }))
        .expect("the gridlines are printed");
    assert_eq!(setup_of(&mut session, sheet)["gridlines"], json!(true));
    session
        .execute("grid.print.gridlines", &json!({ "sheet": sheet, "on": false }))
        .expect("the gridlines are not printed");
    assert_eq!(setup_of(&mut session, sheet)["gridlines"], json!(false));
}

/// **The headings may be printed, and the fact is read back.**
#[test]
fn the_headings_may_be_printed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(setup_of(&mut session, sheet)["headings"], json!(false), "the tree prints none");
    session
        .execute("grid.print.headings", &json!({ "sheet": sheet, "on": true }))
        .expect("the headings are printed");
    assert_eq!(setup_of(&mut session, sheet)["headings"], json!(true));
}

/// **The pages say which marks they carry.**
#[test]
fn the_pages_say_their_marks() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.print.gridlines", &json!({ "sheet": sheet, "on": true }))
        .expect("the gridlines are printed");
    session
        .execute("grid.print.headings", &json!({ "sheet": sheet, "on": true }))
        .expect("the headings are printed");
    let pages = pages_of(&mut session, sheet);
    assert_eq!(pages[0]["gridlines"], json!(true));
    assert_eq!(pages[0]["headings"], json!(true));
}

/// **The headings take room from the page.**
#[test]
fn the_headings_take_room() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_column(&mut session, sheet, 45);
    assert_eq!(pages_of(&mut session, sheet).len(), 1, "forty-five rows are one page");
    session
        .execute("grid.print.headings", &json!({ "sheet": sheet, "on": true }))
        .expect("the headings are printed");
    assert_eq!(pages_of(&mut session, sheet).len(), 2, "the headings take the room of a row");
}

/// **The gridlines take no room: they are drawn between the cells.**
#[test]
fn the_gridlines_take_no_room() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_column(&mut session, sheet, 45);
    session
        .execute("grid.print.gridlines", &json!({ "sheet": sheet, "on": true }))
        .expect("the gridlines are printed");
    assert_eq!(pages_of(&mut session, sheet).len(), 1, "the gridlines take no room");
}

/// **The marks of a print are not the marks of the view.**
#[test]
fn the_marks_of_a_print_are_not_the_marks_of_the_view() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "gridlines": false }))
        .expect("the view is set");
    session
        .execute("grid.print.gridlines", &json!({ "sheet": sheet, "on": true }))
        .expect("the gridlines are printed");
    let outcome = session.execute("grid.sheet.get", &json!({ "sheet": sheet })).expect("the sheet is read");
    assert_eq!(outcome.data["view"]["gridlines"], json!(false), "the view is its own fact");
    assert_eq!(setup_of(&mut session, sheet)["gridlines"], json!(true), "the print is its own fact");
}
