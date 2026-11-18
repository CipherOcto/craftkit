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

/// A **top-ten** filter keeps the rows whose number is at the
/// threshold or above it, and hides the rest.
#[test]
fn a_top_ten_filter_keeps_the_highest_rows() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 9.0);
    session
        .execute("grid.filter.top10", &json!({ "sheet": sheet, "column": "A", "count": 2 }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1], "the lowest row is hidden");
}

/// A top-ten filter may keep the **lowest** rows, and its count may
/// be a **percent** of the numbers of the column.
#[test]
fn a_top_ten_filter_may_keep_the_lowest_rows() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 9.0);
    session
        .execute("grid.filter.top10", &json!({ "sheet": sheet, "column": "A", "count": 1, "bottom": true }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![2, 3], "the two highest rows are hidden");
    session
        .execute("grid.filter.top10", &json!({ "sheet": sheet, "column": "A", "count": 50, "percent": true }))
        .expect("the sheet is filtered");
    // Half of three rows is two of them, rounded up: the threshold is
    // the second highest, and the lowest row is hidden.
    assert_eq!(hidden_rows(&mut session, sheet), vec![1], "the lowest row is hidden");
}

/// A filter may keep the rows that are **above** the average of a
/// column, and the rows that are below it.
#[test]
fn a_filter_may_keep_the_rows_above_the_average() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 9.0);
    session
        .execute("grid.filter.average", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1, 2], "the rows at the average and below are hidden");
    session
        .execute("grid.filter.average", &json!({ "sheet": sheet, "column": "A", "above": false }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![2, 3], "and the rows above it are hidden");
}

/// The number a filter's criterion is compared with is read by the
/// tree's own rule of numbers, so a date is a number.
#[test]
fn a_filter_criterion_is_read_by_the_trees_own_rule_of_numbers() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 43831.0);
    a_number(&mut session, sheet, "A2", 44000.0);
    session
        .execute(
            "grid.filter.set",
            &json!({ "sheet": sheet, "column": "A", "op": ">=", "value": "1/1/2020" }),
        )
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), Vec::<u64>::new(), "every row is after the date");
    // The first of June 2020 is between the two numbers.
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "6/1/2020" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1], "and the first row is before it");
}

/// A filter may look for the **words** a cell begins with, and the
/// words it holds.
#[test]
fn a_filter_may_look_for_the_words_a_cell_holds() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the cat sat" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": "the dog sat" }))
        .expect("the cell is set");
    session
        .execute(
            "grid.filter.set",
            &json!({ "sheet": sheet, "column": "A", "op": "beginsWith", "value": "the ca" }),
        )
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![2], "the row that does not begin with it is hidden");
    session
        .execute(
            "grid.filter.set",
            &json!({ "sheet": sheet, "column": "A", "op": "contains", "value": "dog" }),
        )
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1], "and the row that does not hold it");
}

/// A filter may judge the **colour** of the cells: the fill behind
/// their text, or the colour of their text.
#[test]
fn a_filter_may_judge_the_colour_of_the_cells() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": 2 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.fill", &json!({ "sheet": sheet, "cell": "A1", "colour": "#ff0000" }))
        .expect("the cell is filled");
    session
        .execute("grid.filter.colour", &json!({ "sheet": sheet, "column": "A", "colour": "#ff0000" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![2], "the row that is not red is hidden");
    session
        .execute(
            "grid.filter.colour",
            &json!({ "sheet": sheet, "column": "A", "colour": "#ff0000", "font": true }),
        )
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1, 2], "and no cell has a red text");
}

/// A filter may keep the rows whose cell is **empty**.
#[test]
fn a_filter_may_keep_the_rows_whose_cell_is_empty() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A3", 3.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "isEmpty" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![1, 3], "the rows that are not empty are hidden");
}

/// A filter may keep the rows whose cell holds an **error**.
#[test]
fn a_filter_may_keep_the_rows_whose_cell_holds_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": { "error": "#DIV/0!" } }))
        .expect("the cell is set");
    a_number(&mut session, sheet, "A2", 5.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "isError" }))
        .expect("the sheet is filtered");
    assert_eq!(hidden_rows(&mut session, sheet), vec![2], "the row that holds no error is hidden");
}

/// A value with an operation that compares with **nothing** is refused.
#[test]
fn a_value_with_an_operation_that_compares_with_nothing_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let failure = session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": "isEmpty", "value": "x" }))
        .expect_err("the filter is refused");
    let message = failure.to_string();
    assert!(message.contains("compares with nothing"), "the refusal says why: {message}");
}

/// A comparison with **no value** is refused: the tree does not treat
/// an absent value as an empty one.
#[test]
fn a_comparison_with_no_value_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let failure = session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">" }))
        .expect_err("the filter is refused");
    let message = failure.to_string();
    assert!(message.contains("compares with a value"), "the refusal says why: {message}");
}
