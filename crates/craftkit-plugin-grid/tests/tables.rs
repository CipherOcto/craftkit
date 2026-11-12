//! The **table** of a sheet: a named range whose first row names the
//! fields.

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

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A sheet whose first row names **three fields**.
fn a_sheet_with_a_header(session: &mut Session) -> u64 {
    let (_, sheet) = a_workbook(session);
    a_text(session, sheet, "A1", "who");
    a_text(session, sheet, "B1", "amount");
    a_text(session, sheet, "C1", "when");
    a_text(session, sheet, "A2", "ann");
    a_text(session, sheet, "B2", "3");
    sheet
}

/// **A table is made and read back.**
#[test]
fn a_table_is_made_and_read_back() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    let outcome = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    assert_eq!(outcome.data["name"], "spendings");
    assert_eq!(outcome.data["range"], "A1:C2");

    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0]["name"], "spendings");
    assert_eq!(tables[0]["range"], "A1:C2");
}

/// **The fields of a table are the text of its header.**
#[test]
fn the_fields_are_the_text_of_the_header() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the fields are read");
    let fields = outcome.data["fields"].as_array().expect("the fields are a list");
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0]["name"], "who");
    assert_eq!(fields[0]["column"], "A");
    assert_eq!(fields[1]["name"], "amount");
    assert_eq!(fields[2]["cell"], "C1");
    assert_eq!(outcome.data["rows"], 1);
}

/// **A header cell that holds a number does not name a field.**
#[test]
fn a_header_that_holds_a_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 7 }))
        .expect("the cell is set");
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(message.contains("B1"), "the refusal says which cell: {message}");
    assert!(message.contains("a field is named by text"), "the refusal says why: {message}");
}

/// **A table of one cell is a cell.**
#[test]
fn a_table_of_one_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "one", "range": "A1" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(message.contains("a table of one cell is a cell"), "the refusal says why: {message}");
}

/// **Two tables may not share a cell.**
#[test]
fn two_tables_that_overlap_are_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "one", "range": "A1:C2" }))
        .expect("the table is made");
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "two", "range": "B1:C3" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(
        message.contains("a cell that is in two tables is in none of them"),
        "the refusal says why: {message}"
    );
}

/// **A name that looks like a cell is not a name.**
#[test]
fn a_name_that_looks_like_a_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    let failure = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "A1", "range": "A1:C2" }))
        .expect_err("the table is refused");
    let message = failure.to_string();
    assert!(message.contains("looks like a cell"), "the refusal says why: {message}");
}

/// **A table is renamed.**
#[test]
fn a_table_is_renamed() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute("grid.table.rename", &json!({ "sheet": sheet, "name": "spendings", "to": "outgoings" }))
        .expect("the table is renamed");
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert_eq!(tables[0]["name"], "outgoings");
}

/// **A table is resized.**
#[test]
fn a_table_is_resized() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute("grid.table.resize", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is resized");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the fields are read");
    assert_eq!(outcome.data["range"], "A1:C2");
}

/// **A table is taken away, and the cells stay.**
#[test]
fn a_table_is_taken_away() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    session
        .execute("grid.table.remove", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the table is taken away");
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert!(tables.is_empty());
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A2" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], "ann");
}

/// **The fields are derived: a header that is edited names another
/// field.**
#[test]
fn the_fields_are_derived_from_the_header() {
    let mut session = session();
    let sheet = a_sheet_with_a_header(&mut session);
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:C2" }))
        .expect("the table is made");
    a_text(&mut session, sheet, "B1", "sum");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "spendings" }))
        .expect("the fields are read");
    let fields = outcome.data["fields"].as_array().expect("the fields are a list");
    assert_eq!(fields[1]["name"], "sum");
}
