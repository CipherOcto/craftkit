//! The **total row** of a table.

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

/// A **sheet** in a workbook.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **cell** of a sheet, holding a value.
fn a_cell(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **table** over the range, with its header row of text.
fn a_table(session: &mut Session, sheet: u64, name: &str, range: &str) {
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": name, "range": range }))
        .expect("the table is created");
}

/// The **fields** of a table, as the command reads them.
fn fields_of(session: &mut Session, sheet: u64, name: &str) -> Json {
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": name }))
        .expect("the fields are read");
    outcome.data.clone()
}

/// Say whether the table has a **total row**.
fn totals(session: &mut Session, sheet: u64, name: &str, totals: bool) {
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": name, "totals": totals }))
        .expect("the total row is said");
}

/// A table with a **total row** says that it has one.
#[test]
fn a_table_with_a_total_row_says_that_it_has_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "B1", json!("who"));
    a_table(&mut session, sheet, "the table", "A1:B4");
    totals(&mut session, sheet, "the table", true);
    let fields = fields_of(&mut session, sheet, "the table");
    assert_eq!(fields["totals"], json!(true), "the table says that it has one");
}

/// The **totals** are the sums of the columns.
#[test]
fn the_totals_are_the_sums_of_the_columns() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "B1", json!("who"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    a_cell(&mut session, sheet, "A3", json!(32.0));
    a_table(&mut session, sheet, "the table", "A1:B4");
    totals(&mut session, sheet, "the table", true);
    let fields = fields_of(&mut session, sheet, "the table");
    let sums = fields["sums"].as_array().expect("the sums are a list");
    assert_eq!(sums[0]["name"], json!("amount"), "the column is named");
    assert_eq!(sums[0]["total"], json!(42.0), "and the total is the sum");
}

/// The **last row** of a table with a total row is the total row.
#[test]
fn the_last_row_of_a_table_with_a_total_row_is_the_total_row() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    a_cell(&mut session, sheet, "A3", json!(32.0));
    a_cell(&mut session, sheet, "A4", json!(999.0));
    a_table(&mut session, sheet, "the table", "A1:A4");
    totals(&mut session, sheet, "the table", true);
    let fields = fields_of(&mut session, sheet, "the table");
    let sums = fields["sums"].as_array().expect("the sums are a list");
    assert_eq!(sums[0]["total"], json!(42.0), "the last row is not counted in the total");
}

/// A table with **no total row** has no totals.
#[test]
fn a_table_with_no_total_row_has_no_totals() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    a_table(&mut session, sheet, "the table", "A1:A3");
    let fields = fields_of(&mut session, sheet, "the table");
    assert_eq!(fields["totals"], json!(false), "the table has no total row");
    assert_eq!(fields["sums"], json!([]), "and no totals");
}

/// The total row may be **taken away**.
#[test]
fn the_total_row_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    a_table(&mut session, sheet, "the table", "A1:A3");
    totals(&mut session, sheet, "the table", true);
    totals(&mut session, sheet, "the table", false);
    let fields = fields_of(&mut session, sheet, "the table");
    assert_eq!(fields["totals"], json!(false), "the table has no total row");
    assert_eq!(fields["sums"], json!([]), "and no totals");
}

/// A column with **no numbers** has no total.
#[test]
fn a_column_with_no_numbers_has_no_total() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "B1", json!("who"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    a_cell(&mut session, sheet, "B2", json!("the first"));
    a_table(&mut session, sheet, "the table", "A1:B4");
    totals(&mut session, sheet, "the table", true);
    let fields = fields_of(&mut session, sheet, "the table");
    let sums = fields["sums"].as_array().expect("the sums are a list");
    assert_eq!(sums[0]["total"], json!(10.0), "the column of numbers has a total");
    assert_eq!(sums[1]["total"], Json::Null, "and the column of text has none");
}

/// The **totals** are derived: they follow the cells.
#[test]
fn the_totals_are_derived_and_they_follow_the_cells() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    a_table(&mut session, sheet, "the table", "A1:A3");
    totals(&mut session, sheet, "the table", true);
    let before = fields_of(&mut session, sheet, "the table");
    let sums = before["sums"].as_array().expect("the sums are a list");
    assert_eq!(sums[0]["total"], json!(10.0), "the total is the sum");
    a_cell(&mut session, sheet, "A2", json!(20.0));
    let after = fields_of(&mut session, sheet, "the table");
    let sums = after["sums"].as_array().expect("the sums are a list");
    assert_eq!(sums[0]["total"], json!(20.0), "and the total follows the cell");
}

/// A table that is **not there** is refused.
#[test]
fn a_table_that_is_not_there_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "the table", "totals": true }))
        .expect_err("the total row is refused");
    assert!(refused.to_string().contains("no table"), "the refusal says why: {refused}");
}
