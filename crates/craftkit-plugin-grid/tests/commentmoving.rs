//! The **comments** of a sheet move with the cells they are on.

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

/// A **workbook** with a sheet in it, and their ids.
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

/// The **comment** on a cell, as the answer holds it.
fn comment_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the comment is read");
    outcome.data["comment"].clone()
}

/// A column that is **inserted** moves the comment of the cell to its
/// right.
#[test]
fn an_inserted_column_moves_the_comment_of_the_cell_to_its_right() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "B1", "text": "mine" }))
        .expect("the comment is written");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A", "count": 1 }))
        .expect("the column is inserted");
    let comment = comment_of(&mut session, sheet, "C1");
    assert_eq!(comment["text"], json!("mine"), "the comment moved right: {comment:?}");
    let comment = comment_of(&mut session, sheet, "B1");
    assert_eq!(comment, json!(null), "and it is not where it was: {comment:?}");
}

/// The comment of a **deleted** cell goes with the cell.
#[test]
fn the_comment_of_a_deleted_cell_goes_with_the_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "mine" }))
        .expect("the comment is written");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A", "count": 1 }))
        .expect("the column is deleted");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment, json!(null), "the comment is gone: {comment:?}");
}

/// A **sort** moves the comment with the row it describes.
#[test]
fn a_sort_moves_the_comment_with_the_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "b" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": "a" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A2", "text": "mine" }))
        .expect("the comment is written");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "A" }))
        .expect("the sheet is sorted");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment["text"], json!("mine"), "the comment moved up: {comment:?}");
    let comment = comment_of(&mut session, sheet, "A2");
    assert_eq!(comment, json!(null), "and it is not where it was: {comment:?}");
}
