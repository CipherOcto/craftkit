//! The **filter** of a sheet.
//!
//! A filter is a column and a criterion, and the rows it hides are
//! derived from the cells.

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

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **rows** a sheet's filter hides.
fn hidden_rows(session: &mut Session, sheet: u64) -> Vec<u64> {
    let outcome = session.execute("grid.filter.rows", &json!({ "sheet": sheet })).expect("the rows are read");
    outcome.data["rows"]
        .as_array()
        .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect())
        .unwrap_or_default()
}

/// The **value** of a cell, worked out.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// A filter hides the rows whose cell does **not** match.
#[test]
fn a_filter_hides_the_rows_that_do_not_match() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 9.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "4" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1], "the row that does not match is hidden");
}

/// The rows a filter hides are **derived**: they are worked out from
/// the cells, and they are not written down.
#[test]
fn the_rows_are_derived() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "4" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1], "the row is hidden");
    // The cell changes and the rows are worked out again.
    a_number(&mut session, sheet, "A1", 7.0);
    assert!(hidden_rows(&mut session, sheet).is_empty(), "and the row is shown again");
}

/// The tree's `*` is a **wildcard**: `*abc*` is "contains abc".
#[test]
fn the_wildcard_is_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the sales figures" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": "the costs" }))
        .expect("the cell is set");
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "=", "value": "*sales*" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![2], "the row that does not match is hidden");
}

/// The text of a cell is compared **without case**.
#[test]
fn the_text_is_compared_without_case() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "Sales" }))
        .expect("the cell is set");
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "=", "value": "sales" }))
        .expect("the sheet is filtered");
    assert!(hidden_rows(&mut session, sheet).is_empty(), "the row matches");
}

/// A subtotal leaves the rows a filter hides out, whatever the function
/// number: that is the tree's rule.
#[test]
fn a_subtotal_leaves_the_filtered_rows_out() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 9.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "4" }))
        .expect("the sheet is filtered");
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUBTOTAL(9,A1:A3)" }),
        )
        .expect("the formula is set");
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "B2", "formula": "=SUBTOTAL(109,A1:A3)" }),
        )
        .expect("the formula is set");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(14.0), "the plain form leaves the row out");
    assert_eq!(value_of(&mut session, sheet, "B2"), json!(14.0), "and so does the hundred form");
}

/// A **sum** does not leave the filtered rows out: only a subtotal does.
#[test]
fn a_sum_does_not_leave_the_filtered_rows_out() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "4" }))
        .expect("the sheet is filtered");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUM(A1:A2)" }))
        .expect("the formula is set");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(6.0), "the sum is of every cell");
}

/// The filter may be **taken away**, and then no row is hidden by it.
#[test]
fn the_filter_may_be_taken_away() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "4" }))
        .expect("the sheet is filtered");
    session.execute("grid.filter.clear", &json!({ "sheet": sheet })).expect("the filter is taken away");
    assert!(hidden_rows(&mut session, sheet).is_empty(), "and no row is hidden by it");
}

/// A sheet that is **not filtered** may not have its filter taken away.
#[test]
fn a_sheet_that_is_not_filtered_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error =
        session.execute("grid.filter.clear", &json!({ "sheet": sheet })).expect_err("the filter is refused");
    assert!(error.to_string().contains("is not filtered"), "the error says so: {error}");
}

/// An operation that is **not one of the tree's own** is refused, and
/// the refusal says what they are.
#[test]
fn an_operation_that_is_not_one_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "~=", "value": "4" }))
        .expect_err("the filter is refused");
    assert!(error.to_string().contains("there is no operation `~=`"), "the error says so: {error}");
    assert!(error.to_string().contains(">="), "and it says what the operations are");
}

/// A comparison that needs a **number** and is given text is refused.
#[test]
fn a_comparison_with_text_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "abc" }))
        .expect_err("the filter is refused");
    assert!(error.to_string().contains("compares numbers"), "the error says so: {error}");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "3", "op": "=", "value": "4" }))
        .expect_err("the filter is refused");
    assert!(error.to_string().contains("is not a column"), "the error says so: {error}");
}

/// The filter is **saved**: it is in the document after it is done.
#[test]
fn the_filter_is_saved() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "B", "op": ">", "value": "10" }))
        .expect("the sheet is filtered");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    assert_eq!(node.data["filter"]["column"].as_str(), Some("B"), "the column is there");
    assert_eq!(node.data["filter"]["op"].as_str(), Some(">"), "and the operation");
    assert_eq!(node.data["filter"]["value"].as_str(), Some("10"), "and the value");
}
