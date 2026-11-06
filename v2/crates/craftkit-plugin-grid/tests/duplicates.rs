//! The **duplicates** of a range.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **values** of a range, as the answer holds them.
fn values_of(session: &mut Session, sheet: u64, range: &str) -> Vec<Json> {
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": range }))
        .expect("the range is read");
    outcome.data["values"].as_array().expect("the values are a list").clone()
}

/// The rows that hold **what an earlier row holds** are emptied, and
/// the first one is kept.
#[test]
fn the_duplicate_rows_are_emptied() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "A2", json!("one"));
    value(&mut session, sheet, "B2", json!(1));
    value(&mut session, sheet, "A3", json!("two"));
    value(&mut session, sheet, "B3", json!(2));
    let outcome = session
        .execute("grid.duplicates.remove", &json!({ "sheet": sheet, "range": "A1:B3" }))
        .expect("the duplicates are removed");
    assert_eq!(outcome.data["duplicates"], json!(1), "one row is a duplicate");
    let values = values_of(&mut session, sheet, "A1:B3");
    assert_eq!(values[0][0], json!("one"), "the first row is kept");
    assert_eq!(values[1][0], Json::Null, "the second row is empty");
    assert_eq!(values[2][0], json!("two"), "and the third row is kept");
}

/// The **key** may be made of some of the columns only.
#[test]
fn the_key_may_be_made_of_some_columns_only() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "A2", json!("one"));
    value(&mut session, sheet, "B2", json!(2));
    let outcome = session
        .execute("grid.duplicates.remove", &json!({ "sheet": sheet, "range": "A1:B2", "columns": "A" }))
        .expect("the duplicates are removed");
    assert_eq!(outcome.data["duplicates"], json!(1), "the two rows hold the same key");
}

/// The **header** stays where it is and is not part of the key.
#[test]
fn the_header_stays_where_it_is() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("name"));
    value(&mut session, sheet, "A2", json!("one"));
    value(&mut session, sheet, "A3", json!("one"));
    let outcome = session
        .execute("grid.duplicates.remove", &json!({ "sheet": sheet, "range": "A1:A3", "header": true }))
        .expect("the duplicates are removed");
    assert_eq!(outcome.data["duplicates"], json!(1), "the second name is a duplicate");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], json!("name"), "the header is still there");
}

/// Two cells that show the same thing are the **same thing**.
#[test]
fn two_cells_that_show_the_same_thing_are_the_same_thing() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(1.0));
    let outcome = session
        .execute("grid.duplicates.remove", &json!({ "sheet": sheet, "range": "A1:A2" }))
        .expect("the duplicates are removed");
    assert_eq!(outcome.data["duplicates"], json!(1), "the two numbers are the same number");
}

/// A row whose key is **empty** is left alone.
#[test]
fn a_row_whose_key_is_empty_is_left_alone() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    let outcome = session
        .execute("grid.duplicates.remove", &json!({ "sheet": sheet, "range": "A1:A3" }))
        .expect("the duplicates are removed");
    assert_eq!(outcome.data["rows"], json!(1), "only the row that holds something is examined");
    assert_eq!(outcome.data["duplicates"], json!(0), "and nothing is a duplicate");
}

/// A **column** that is not in the range is refused.
#[test]
fn a_column_that_is_not_in_the_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    let outcome = session
        .execute("grid.duplicates.remove", &json!({ "sheet": sheet, "range": "A1:A2", "columns": "C" }));
    assert!(outcome.is_err(), "the column is not in the range");
}
