//! The **column beyond Z**: `AA`, `AB` and the rest of the letters of
//! a column.

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

/// The **value** of a cell.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// **A cell beyond Z may be set, and it is the cell it says.**
#[test]
fn a_cell_beyond_z_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "AA1", 7.0);
    a_number(&mut session, sheet, "AB1", 8.0);
    assert_eq!(value_of(&mut session, sheet, "AA1"), json!(7.0));
    assert_eq!(value_of(&mut session, sheet, "AB1"), json!(8.0));
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(null), "the first column is its own");
}

/// **The used range reaches the column beyond Z.**
#[test]
fn the_used_range_reaches_beyond_z() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "AD1", 2.0);
    let outcome = session.execute("grid.range.read", &json!({ "sheet": sheet })).expect("the range is read");
    assert_eq!(outcome.data["range"], json!("A1:AD1"));
}

/// **A formula may read the columns beyond Z.**
#[test]
fn a_formula_may_read_beyond_z() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "AA1", 2.0);
    a_number(&mut session, sheet, "AB1", 3.0);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "AC1", "formula": "SUM(AA1:AB1)" }))
        .expect("the formula is set");
    assert_eq!(value_of(&mut session, sheet, "AC1"), json!(5.0));
}

/// **A range of columns beyond Z holds the values it holds.**
#[test]
fn a_range_beyond_z_holds_its_values() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "AA1", 1.0);
    a_number(&mut session, sheet, "AB1", 2.0);
    a_number(&mut session, sheet, "AC1", 3.0);
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": "AA1:AC1" }))
        .expect("the range is read");
    assert_eq!(outcome.data["values"], json!([[1.0, 2.0, 3.0]]));
}

/// **A sheet may be sorted by a column beyond Z.**
#[test]
fn a_sheet_may_be_sorted_beyond_z() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "AA1", 3.0);
    a_number(&mut session, sheet, "AA2", 1.0);
    a_number(&mut session, sheet, "AA3", 2.0);
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "AA1:AA3", "column": "AA" }))
        .expect("the sheet is sorted");
    assert_eq!(value_of(&mut session, sheet, "AA1"), json!(1.0));
    assert_eq!(value_of(&mut session, sheet, "AA2"), json!(2.0));
    assert_eq!(value_of(&mut session, sheet, "AA3"), json!(3.0));
}

/// **The column beyond Z is the one after Z.**
#[test]
fn the_column_beyond_z_is_the_one_after_z() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "AA1", 2.0);
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": "A1:AA1" }))
        .expect("the range is read");
    let row = outcome.data["values"][0].as_array().expect("the row is a list").clone();
    assert_eq!(row.len(), 27, "the range from A to AA is twenty-seven columns wide");
    assert_eq!(row[0], json!(1.0), "the first column holds its own value");
    assert_eq!(row[26], json!(2.0), "the value is in the column after Z");
    let outcome = session.execute("grid.sheet.get", &json!({ "sheet": sheet })).expect("the sheet is read");
    assert_eq!(outcome.data["used_range"], json!("A1:AA1"), "the range is two columns wide");
}
