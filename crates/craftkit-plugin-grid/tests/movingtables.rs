//! The **table** and the **pivot** that move.

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
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **row** a range begins on: the header of a table or a pivot is
/// the first row of its range.
fn first_row(range: &str) -> u64 {
    let first = range.trim().split(':').next().unwrap_or(range.trim());
    let digits: String = first.chars().filter(|character| character.is_ascii_digit()).collect();
    digits.parse().unwrap_or(1)
}

/// A **table** on a range, with a header of texts.
fn set_table(session: &mut Session, sheet: u64, name: &str, range: &str) {
    let row = first_row(range);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": format!("A{row}"), "value": "name" }))
        .expect("the header is written");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": format!("B{row}"), "value": "size" }))
        .expect("the header is written");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": name, "range": range }))
        .expect("the table is made");
}

/// A **pivot** on a range, with three fields.
fn set_pivot(session: &mut Session, sheet: u64, name: &str, range: &str) {
    let row = first_row(range);
    for (column, text) in [("A", "name"), ("B", "size"), ("C", "kind")] {
        session
            .execute(
                "grid.cell.set",
                &json!({ "sheet": sheet, "cell": format!("{column}{row}"), "value": text }),
            )
            .expect("the header is written");
    }
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": name,
                "range": range,
                "row": "A",
                "column": "B",
                "value": "C",
                "aggregate": "sum",
            }),
        )
        .expect("the pivot is made");
}

/// The **payload** of the sheet: the facts it holds.
fn payload(session: &Session, sheet: u64) -> Json {
    session.document().node(sheet).expect("the sheet is there").data.clone()
}

/// A **table** moves with the rows.
#[test]
fn a_table_moves_with_the_rows() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_table(&mut session, sheet, "the table", "A1:B5");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["tables"][0]["range"], json!("A2:B6"), "the range moved down");
}

/// A **table** moves with the columns.
#[test]
fn a_table_moves_with_the_columns() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_table(&mut session, sheet, "the table", "A1:B5");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["tables"][0]["range"], json!("B1:C5"), "the range moved right");
}

/// A **table** that is deleted goes away.
#[test]
fn a_table_that_is_deleted_goes_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_table(&mut session, sheet, "the table", "A2:B4");
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2, "count": 3 }))
        .expect("the rows are deleted");
    let payload = payload(&session, sheet);
    assert!(payload["tables"].as_array().is_some_and(|list| list.is_empty()), "the table went with the rows");
}

/// A **table** that covers a deleted row refuses.
#[test]
fn a_table_that_covers_a_deleted_row_refuses() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_table(&mut session, sheet, "the table", "A1:B5");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("row that is deleted"), "the refusal says why");
}

/// A **pivot** moves with the rows.
#[test]
fn a_pivot_moves_with_the_rows() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_pivot(&mut session, sheet, "the pivot", "A1:C5");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["pivots"][0]["range"], json!("A2:C6"), "the range moved down");
    assert_eq!(payload["pivots"][0]["row"], json!("A"), "the fields are columns, and they stayed");
}

/// A **pivot** moves with the columns, and its fields move with it.
#[test]
fn a_pivot_moves_with_the_columns() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_pivot(&mut session, sheet, "the pivot", "A1:C5");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["pivots"][0]["range"], json!("B1:D5"), "the range moved right");
    assert_eq!(payload["pivots"][0]["row"], json!("B"), "the field moved right with the range");
    assert_eq!(payload["pivots"][0]["column"], json!("C"), "and so did the other one");
    assert_eq!(payload["pivots"][0]["value"], json!("D"), "and the last one");
}

/// A **pivot** that gathers by a deleted column refuses.
#[test]
fn a_pivot_that_gathers_by_a_deleted_column_refuses() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_pivot(&mut session, sheet, "the pivot", "A1:C5");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "C" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("gathers by the column C"), "the refusal says why: {refused}");
}

/// A **pivot** that is deleted goes away.
#[test]
fn a_pivot_that_is_deleted_goes_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_pivot(&mut session, sheet, "the pivot", "A2:C4");
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2, "count": 3 }))
        .expect("the rows are deleted");
    let payload = payload(&session, sheet);
    assert!(payload["pivots"].as_array().is_some_and(|list| list.is_empty()), "the pivot went with the rows");
}
