//! The **hiding** of a row and a column.
//!
//! A hidden row is still a row: its cells are still there and they are
//! still worked out.

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
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **rows** of a sheet that are hidden.
fn hidden_rows(session: &Session, sheet: u64) -> Vec<u64> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["hidden_rows"]
        .as_array()
        .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect())
        .unwrap_or_default()
}

/// The **columns** of a sheet that are hidden.
fn hidden_columns(session: &Session, sheet: u64) -> Vec<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["hidden_columns"]
        .as_array()
        .map(|columns| {
            columns.iter().filter_map(|column| column.as_str().map(|column| column.to_string())).collect()
        })
        .unwrap_or_default()
}

/// A row may be **hidden**.
#[test]
fn a_row_may_be_hidden() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    assert_eq!(hidden_rows(&session, sheet), vec![3], "the sheet says so");
}

/// A hidden row is still a row: its cells are still there and they are
/// still worked out.
#[test]
fn a_hidden_row_is_still_a_row() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A3", "value": 10 }))
        .expect("the cell is set");
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    assert_eq!(
        outcome.data["cells"].as_object().map(|cells| cells.len()),
        Some(0),
        "the value is worked out"
    );
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": "A3" }))
        .expect("the cell is read");
    assert!(outcome.data["text"].is_string(), "and the cell is still there");
}

/// A row that is **already hidden** may not be hidden again: a command
/// that does nothing did not do anything.
#[test]
fn a_row_that_is_already_hidden_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    let error = session
        .execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 }))
        .expect_err("the row is refused");
    assert!(error.to_string().contains("is already hidden"), "the error says so: {error}");
}

/// A hidden row may be **shown** again.
#[test]
fn a_hidden_row_may_be_shown() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    session.execute("grid.row.show", &json!({ "sheet": sheet, "row": 3 })).expect("the row is shown");
    assert!(hidden_rows(&session, sheet).is_empty(), "the sheet says so");
}

/// A row that is **not hidden** may not be shown.
#[test]
fn a_row_that_is_not_hidden_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.show", &json!({ "sheet": sheet, "row": 3 }))
        .expect_err("the row is refused");
    assert!(error.to_string().contains("is not hidden"), "the error says so: {error}");
}

/// A column may be **hidden** and **shown**.
#[test]
fn a_column_may_be_hidden_and_shown() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "C" }))
        .expect("the column is hidden");
    assert_eq!(hidden_columns(&session, sheet), vec!["C".to_string()], "the sheet says so");
    session
        .execute("grid.column.show", &json!({ "sheet": sheet, "column": "C" }))
        .expect("the column is shown");
    assert!(hidden_columns(&session, sheet).is_empty(), "and it is shown again");
}

/// A column that is **already hidden** may not be hidden again.
#[test]
fn a_column_that_is_already_hidden_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "C" }))
        .expect("the column is hidden");
    let error = session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "C" }))
        .expect_err("the column is refused");
    assert!(error.to_string().contains("is already hidden"), "the error says so: {error}");
}

/// A column that is **not hidden** may not be shown.
#[test]
fn a_column_that_is_not_hidden_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.column.show", &json!({ "sheet": sheet, "column": "C" }))
        .expect_err("the column is refused");
    assert!(error.to_string().contains("is not hidden"), "the error says so: {error}");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "3" }))
        .expect_err("the column is refused");
    assert!(error.to_string().contains("is not a column"), "the error says so: {error}");
}

/// A row that is **not there** is refused.
#[test]
fn a_row_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.hide", &json!({ "sheet": sheet, "row": 0 }))
        .expect_err("the row is refused");
    assert!(error.to_string().contains("counted from one"), "the error says so: {error}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.row.hide", &json!({ "sheet": workbook, "row": 3 }))
        .expect_err("the row is refused");
    assert!(error.to_string().contains("is not a sheet"), "the error says so: {error}");
}

/// The hiding is **saved**: it is in the document after it is done.
#[test]
fn the_hiding_is_saved() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "C" }))
        .expect("the column is hidden");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    assert_eq!(node.data["hidden_rows"][0].as_u64(), Some(3), "the row is there");
    assert_eq!(node.data["hidden_columns"][0].as_str(), Some("C"), "and the column is there");
}
