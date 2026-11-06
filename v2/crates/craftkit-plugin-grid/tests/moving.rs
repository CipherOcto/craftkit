//! The **facts that move**: the conditional formats, the validations
//! and the sparklines.

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

/// A **conditional format** on a range.
fn set_conditional(session: &mut Session, sheet: u64, range: &str) {
    session
        .execute(
            "grid.conditional.set",
            &json!({
                "sheet": sheet,
                "range": range,
                "op": ">",
                "value": "5",
                "format": "general",
            }),
        )
        .expect("the condition is set");
}

/// A **validation** on a range.
fn set_validation(session: &mut Session, sheet: u64, range: &str) {
    session
        .execute(
            "grid.validation.set",
            &json!({ "sheet": sheet, "range": range, "kind": "whole", "operator": "greaterThan", "formula1": "10" }),
        )
        .expect("the rule is set");
}

/// A **sparkline** in a cell, reading a range.
fn set_sparkline(session: &mut Session, sheet: u64, cell: &str, range: &str) {
    session
        .execute(
            "grid.sparkline.set",
            &json!({ "sheet": sheet, "cell": cell, "range": range, "kind": "line" }),
        )
        .expect("the sparkline is set");
}

/// The **payload** of the sheet: the facts it holds.
fn payload(session: &Session, sheet: u64) -> Json {
    session.document().node(sheet).expect("the sheet is there").data.clone()
}

/// A **conditional format** moves with the rows.
#[test]
fn a_conditional_format_moves_with_the_rows() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "A1:A10");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["conditionals"][0]["range"], json!("A2:A11"), "the range moved down");
}

/// A conditional format **grows** when the row falls inside it.
#[test]
fn a_conditional_format_grows_when_the_row_falls_inside_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "A1:A10");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 5 })).expect("the row is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["conditionals"][0]["range"], json!("A1:A11"), "the range grew by the row");
}

/// A **conditional format** moves with the columns.
#[test]
fn a_conditional_format_moves_with_the_columns() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "A1:C10");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["conditionals"][0]["range"], json!("B1:D10"), "the range moved right");
}

/// A **validation** moves with the rows.
#[test]
fn a_validation_moves_with_the_rows() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_validation(&mut session, sheet, "A1:A9");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["validations"][0]["range"], json!("A2:A10"), "the range moved down");
}

/// A **validation** moves with the columns.
#[test]
fn a_validation_moves_with_the_columns() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_validation(&mut session, sheet, "A1:A9");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["validations"][0]["range"], json!("B1:B9"), "the range moved right");
}

/// A **sparkline** moves with its cell.
#[test]
fn a_sparkline_moves_with_its_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_sparkline(&mut session, sheet, "B2", "C2:F2");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let payload = payload(&session, sheet);
    assert!(payload["sparklines"].get("B3").is_some(), "the sparkline moved down with its cell");
    assert_eq!(payload["sparklines"]["B3"]["range"], json!("C3:F3"), "and the range it reads moved with it");
}

/// A **sparkline** moves with its column.
#[test]
fn a_sparkline_moves_with_its_column() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_sparkline(&mut session, sheet, "B2", "C2:F2");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert!(payload["sparklines"].get("C2").is_some(), "the sparkline moved right with its cell");
    assert_eq!(payload["sparklines"]["C2"]["range"], json!("D2:G2"), "and the range it reads moved with it");
}

/// A conditional format that is **deleted** goes away.
#[test]
fn a_conditional_format_that_is_deleted_goes_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "A2:A4");
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2, "count": 3 }))
        .expect("the rows are deleted");
    let payload = payload(&session, sheet);
    assert!(
        payload["conditionals"].as_array().is_some_and(|list| list.is_empty()),
        "the format went with the rows"
    );
}

/// A conditional format that **covers** a deleted row refuses.
#[test]
fn a_conditional_format_that_covers_a_deleted_row_refuses() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "A1:A10");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 5 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("row that is deleted"), "the refusal says why");
}

/// A conditional format that is deleted **with a column** goes away.
#[test]
fn a_conditional_format_that_is_deleted_with_a_column_goes_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "B1:B10");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect("the column is deleted");
    let payload = payload(&session, sheet);
    assert!(
        payload["conditionals"].as_array().is_some_and(|list| list.is_empty()),
        "the format went with the column"
    );
}

/// A conditional format that **covers** a deleted column refuses.
#[test]
fn a_conditional_format_that_covers_a_deleted_column_refuses() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_conditional(&mut session, sheet, "A1:C10");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("column that is deleted"), "the refusal says why");
}

/// The **sparkline** of a deleted cell goes away with the cell.
#[test]
fn the_sparkline_of_a_deleted_cell_goes_away_with_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_sparkline(&mut session, sheet, "B2", "C2:F2");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 })).expect("the row is deleted");
    let payload = payload(&session, sheet);
    assert!(payload["sparklines"].get("B2").is_none(), "the sparkline went with the row");
}

/// A **validation** that is deleted goes away.
#[test]
fn a_validation_that_is_deleted_goes_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_validation(&mut session, sheet, "A2:A4");
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2, "count": 3 }))
        .expect("the rows are deleted");
    let payload = payload(&session, sheet);
    assert!(
        payload["validations"].as_array().is_some_and(|list| list.is_empty()),
        "the rule went with the rows"
    );
}
