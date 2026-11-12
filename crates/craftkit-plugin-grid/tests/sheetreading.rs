//! **Reading** a sheet.

use craftkit_sdk::serde_json::{Value as Json, json};
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
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **answer** of a read.
fn read(session: &mut Session, sheet: u64) -> Json {
    let outcome = session.execute("grid.sheet.get", &json!({ "sheet": sheet })).expect("the sheet is read");
    outcome.data.clone()
}

/// A sheet may be **read whole**.
#[test]
fn a_sheet_may_be_read_whole() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 7 }))
        .expect("the cell is set");
    let answer = read(&mut session, sheet);
    assert_eq!(answer["id"], json!(sheet), "the sheet is named by its id");
    assert_eq!(answer["name"], Json::String("Sheet1".to_string()), "and by its name");
    assert_eq!(answer["workbook"], json!(workbook), "and it is in a workbook");
    assert_eq!(answer["cells"], json!(1), "and one cell holds something");
    assert_eq!(answer["used_range"], Json::String("B2".to_string()), "and the used range is said");
    assert_eq!(answer["date_system"], Json::String("1900".to_string()), "and the calendar is said");
    assert_eq!(answer["protection"], Json::Null, "and it is not protected");
}

/// A sheet that holds **nothing** has no used range.
#[test]
fn a_sheet_that_holds_nothing_has_no_used_range() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let answer = read(&mut session, sheet);
    assert_eq!(answer["used_range"], Json::Null, "there is nothing to read");
    assert_eq!(answer["cells"], json!(0), "and no cell holds something");
}

/// The **view** of a sheet is said by the answer.
#[test]
fn the_view_of_a_sheet_is_said_by_the_answer() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "gridlines": false }))
        .expect("the view is set");
    session
        .execute("grid.sheet.freeze", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the panes are frozen");
    session.execute("grid.sheet.zoom", &json!({ "sheet": sheet, "percent": 150 })).expect("the zoom is set");
    let answer = read(&mut session, sheet);
    assert_eq!(answer["view"]["gridlines"], json!(false), "the gridlines are not drawn");
    assert_eq!(answer["view"]["headings"], json!(true), "and the headings are");
    assert_eq!(answer["view"]["frozen_rows"], json!(1), "the row above the cell is frozen");
    assert_eq!(answer["view"]["frozen_columns"], json!(1), "and the column to its left");
    assert_eq!(answer["zoom"], json!(150.0), "and the zoom is said");
}

/// The rows and the columns that are **hidden** are said by the answer.
#[test]
fn the_hidden_rows_and_columns_are_said_by_the_answer() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "B" }))
        .expect("the column is hidden");
    let answer = read(&mut session, sheet);
    assert_eq!(answer["hidden_rows"], json!([3]), "the hidden row is said");
    assert_eq!(answer["hidden_columns"], json!(["B"]), "and the hidden column");
}

/// The **protection** of a sheet is said by the answer.
#[test]
fn the_protection_of_a_sheet_is_said_by_the_answer() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "sort": true }))
        .expect("the sheet is protected");
    let answer = read(&mut session, sheet);
    assert_eq!(answer["protection"]["sort"], json!(true), "the sorting is still allowed");
    assert_eq!(answer["protection"]["filter"], json!(false), "and the filtering is not");
    assert_eq!(answer["protection"]["locked"], json!(true), "and the cells are locked");
}

/// A sheet that is **not there** is refused, and so is a node that is
/// not a sheet.
#[test]
fn a_sheet_that_is_not_there_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error =
        session.execute("grid.sheet.get", &json!({ "sheet": 999 })).expect_err("the sheet is refused");
    assert!(error.to_string().contains("there is no sheet `999`"), "the refusal says why: {error}");
    let error = session
        .execute("grid.sheet.get", &json!({ "sheet": workbook }))
        .expect_err("the workbook is not a sheet");
    assert!(error.to_string().contains("not a sheet"), "the refusal says why: {error}");
}
