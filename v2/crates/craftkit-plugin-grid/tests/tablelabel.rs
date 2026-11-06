//! The **label** of the total row of a table.

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

/// A **table** on a sheet, with a header row.
fn a_table(session: &mut Session, sheet: u64) {
    a_text(session, sheet, "A1", "who");
    a_text(session, sheet, "B1", "amount");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "spendings", "range": "A1:B3" }))
        .expect("the table is created");
}

/// The **tables** of a sheet, as the answer holds them.
fn tables_of(session: &mut Session, sheet: u64) -> Vec<Json> {
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    outcome.data["tables"].as_array().expect("the tables are a list").clone()
}

/// A table that gains a total row shows a **label** in it.
#[test]
fn a_table_that_gains_a_total_row_shows_a_label() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "spendings", "totals": true }))
        .expect("the total row is said");
    let tables = tables_of(&mut session, sheet);
    assert_eq!(tables[0]["label"], json!("Total"), "the label is the tree's own word");
}

/// The **label** may be said.
#[test]
fn the_label_may_be_said() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "spendings", "totals": true }))
        .expect("the total row is said");
    let outcome = session
        .execute("grid.table.label", &json!({ "sheet": sheet, "name": "spendings", "label": "Sum" }))
        .expect("the label is said");
    assert_eq!(outcome.data["label"], json!("Sum"), "the label is the one that was said");
}

/// An **empty** label takes the label away.
#[test]
fn an_empty_label_takes_the_label_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "spendings", "totals": true }))
        .expect("the total row is said");
    let outcome = session
        .execute("grid.table.label", &json!({ "sheet": sheet, "name": "spendings", "label": "" }))
        .expect("the label is taken away");
    assert_eq!(outcome.data["label"], Json::Null, "the label is gone");
}

/// A table with **no total row** has no label, and saying one is
/// refused.
#[test]
fn a_label_on_a_table_with_no_total_row_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let outcome =
        session.execute("grid.table.label", &json!({ "sheet": sheet, "name": "spendings", "label": "Sum" }));
    assert!(outcome.is_err(), "there is no row for the label to sit in");
}

/// A table that is **not there** is refused.
#[test]
fn a_table_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.table.label", &json!({ "sheet": sheet, "name": "nothing", "label": "Sum" }));
    assert!(outcome.is_err(), "there is no such table");
}
