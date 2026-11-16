//! The **range** a pivot reads.

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

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A sheet of three **spendings**, and a second block beside them.
fn a_sheet_with_two_blocks(session: &mut Session) -> u64 {
    let (_, sheet) = a_sheet(session);
    a_text(session, sheet, "A1", "who");
    a_text(session, sheet, "B1", "amount");
    a_text(session, sheet, "A2", "ann");
    a_number(session, sheet, "B2", 3.0);
    a_text(session, sheet, "A3", "bob");
    a_number(session, sheet, "B3", 5.0);
    a_text(session, sheet, "D1", "who");
    a_text(session, sheet, "E1", "amount");
    a_text(session, sheet, "D2", "ann");
    a_number(session, sheet, "E2", 10.0);
    a_text(session, sheet, "G1", "who");
    a_text(session, sheet, "H1", "count");
    a_text(session, sheet, "G2", "ann");
    a_number(session, sheet, "H2", 7.0);
    a_text(session, sheet, "J1", "person");
    a_text(session, sheet, "J2", "ann");
    sheet
}

/// The **rows** of a pivot.
fn rows(session: &mut Session, sheet: u64, name: &str) -> Vec<Json> {
    let outcome = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": name }))
        .expect("the pivot is read");
    outcome.data["rows"].as_array().expect("the rows are a list").clone()
}

/// The pivot reads the **new range** after the source is changed.
#[test]
fn the_pivot_reads_the_new_range_after_the_source_is_changed() {
    let mut session = session();
    let sheet = a_sheet_with_two_blocks(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:B3", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    session
        .execute("grid.pivot.source", &json!({ "sheet": sheet, "name": "by who", "range": "D1:E2" }))
        .expect("the source is changed");
    let rows = rows(&mut session, sheet, "by who");
    assert_eq!(rows.len(), 1, "the new range has one row of spendings");
    assert_eq!(rows[0]["key"], "ann", "and it is ann");
    assert_eq!(rows[0]["value"], 10.0, "and the amount is ten");
}

/// A field that is **not in the new range** is taken away.
#[test]
fn a_field_that_is_not_in_the_new_range_is_taken_away() {
    let mut session = session();
    let sheet = a_sheet_with_two_blocks(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:B3", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let outcome =
        session.execute("grid.pivot.source", &json!({ "sheet": sheet, "name": "by who", "range": "G1:H2" }));
    assert!(outcome.is_err(), "the way of putting numbers together needs its column");
}

/// The column a pivot gathers the rows by may **not** be taken away.
#[test]
fn the_column_a_pivot_gathers_the_rows_by_may_not_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet_with_two_blocks(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:B3", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let outcome =
        session.execute("grid.pivot.source", &json!({ "sheet": sheet, "name": "by who", "range": "J1:J2" }));
    assert!(outcome.is_err(), "a pivot without the field it gathers the rows by is not a pivot");
}

/// A range of **one cell** is refused.
#[test]
fn a_range_of_one_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_two_blocks(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:B3", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let outcome =
        session.execute("grid.pivot.source", &json!({ "sheet": sheet, "name": "by who", "range": "D1" }));
    assert!(outcome.is_err(), "a pivot reads more than one cell");
}

/// A pivot that is **not there** is refused.
#[test]
fn a_pivot_that_is_not_there_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_two_blocks(&mut session);
    let outcome =
        session.execute("grid.pivot.source", &json!({ "sheet": sheet, "name": "nowhere", "range": "A1:B3" }));
    assert!(outcome.is_err(), "there is no pivot called `nowhere`");
}
