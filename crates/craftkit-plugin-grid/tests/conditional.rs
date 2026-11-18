//! The **conditional formats** of a sheet.
//!
//! A conditional format is a range, a criterion and a format, and the
//! cells that match are written with it.

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

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell has text").to_string()
}

/// The **cells** a conditional format holds for.
fn matching(session: &mut Session, sheet: u64, range: &str) -> Vec<String> {
    let outcome = session
        .execute("grid.conditional.check", &json!({ "sheet": sheet, "range": range }))
        .expect("the cells are read");
    outcome.data["cells"]
        .as_array()
        .map(|cells| {
            cells.iter().map(|cell| cell["cell"].as_str().expect("a cell is named").to_string()).collect()
        })
        .unwrap_or_default()
}

/// A conditional format writes the cells that **match** with its
/// format.
#[test]
fn a_conditional_format_writes_the_cells_that_match() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 9.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(text_of(&mut session, sheet, "A2"), "9.00", "the cell that matches is written");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1", "and the one that does not is not");
}

/// The cells a conditional format holds for are **derived**.
#[test]
fn the_cells_are_derived() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 9.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(matching(&mut session, sheet, "A1:A2"), vec!["A2".to_string()], "the cell matches");
    a_number(&mut session, sheet, "A2", 2.0);
    assert!(matching(&mut session, sheet, "A1:A2").is_empty(), "and now it does not");
}

/// The conditional format is **on top** of the cell's own format: a
/// rule is on top of a fact.
#[test]
fn the_conditional_format_is_on_top() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 9.0);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0" }))
        .expect("the cell is formatted");
    assert_eq!(text_of(&mut session, sheet, "A1"), "9", "the cell's own format is used");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "9.00", "and the rule is on top");
}

/// A cell **outside** the range is not written by the format.
#[test]
fn a_cell_outside_the_range_is_not_written() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 9.0);
    a_number(&mut session, sheet, "B1", 9.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "9.00", "the cell in the range is written");
    assert_eq!(text_of(&mut session, sheet, "B1"), "9", "and the one outside is not");
}

/// A range may have **one** conditional format: a second one for the
/// same range replaces the first.
#[test]
fn a_second_format_for_a_range_replaces_the_first() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 9.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "5", "format": "0" }),
        )
        .expect("the conditional format is set again");
    assert_eq!(text_of(&mut session, sheet, "A1"), "9", "the second format is the one");
    assert_eq!(matching(&mut session, sheet, "A1:A1").len(), 1, "and there is only one");
}

/// A conditional format may be **taken away**.
#[test]
fn a_conditional_format_may_be_taken_away() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 9.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    session
        .execute("grid.conditional.clear", &json!({ "sheet": sheet, "range": "A1:A1" }))
        .expect("the conditional format is taken away");
    assert_eq!(text_of(&mut session, sheet, "A1"), "9", "and the cell is written as it was");
}

/// A range with **no** conditional format may not have one taken away.
#[test]
fn a_range_with_no_conditional_format_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.conditional.clear", &json!({ "sheet": sheet, "range": "A1:A1" }))
        .expect_err("the format is refused");
    assert!(
        error.to_string().contains("there is no conditional format of A1:A1"),
        "the error says so: {error}"
    );
}

/// A range that is **not a range** is refused.
#[test]
fn a_range_that_is_not_a_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "the top", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect_err("the format is refused");
    assert!(error.to_string().contains("is not a range of cells"), "the error says so: {error}");
}

/// An operation that is **not one of the tree's own** is refused.
#[test]
fn an_operation_that_is_not_one_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": "~", "value": "5", "format": "0.00" }),
        )
        .expect_err("the format is refused");
    assert!(error.to_string().contains("there is no operation `~`"), "the error says so: {error}");
}

/// A format that is **not a format** is refused.
#[test]
fn a_format_that_is_not_a_format_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "5", "format": "purple" }),
        )
        .expect_err("the format is refused");
    assert!(error.to_string().contains("is not a format this tree has"), "the error says so: {error}");
}

/// The conditional format is **saved**: it is in the document after it
/// is done.
#[test]
fn the_conditional_format_is_saved() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    let list = node.data["conditionals"].as_array().expect("the list is there");
    assert_eq!(list.len(), 1, "there is one conditional format");
    assert_eq!(list[0]["range"].as_str(), Some("A1:A2"), "the range is there");
    assert_eq!(list[0]["format"].as_str(), Some("0.00"), "and the format");
}

/// A conditional format may judge the cells by a **formula**, and the
/// formula is written as it is in the top-left cell of the range.
#[test]
fn a_conditional_format_may_judge_the_cells_by_a_formula() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 9.0);
    session
        .execute(
            "grid.conditional.formula",
            &json!({ "sheet": sheet, "range": "A1:A2", "formula": "A1>5", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(text_of(&mut session, sheet, "A2"), "9.00", "the cell that matches is written");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1", "and the one that does not is not");
    assert_eq!(matching(&mut session, sheet, "A1:A2"), vec!["A2".to_string()], "the cell matches");
}

/// A conditional format may judge the **empty** cells.
#[test]
fn a_conditional_format_may_judge_the_empty_cells() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": "isEmpty", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(matching(&mut session, sheet, "A1:A2"), vec!["A2".to_string()], "the empty cell matches");
}

/// A conditional format may judge the cells that hold an **error**.
#[test]
fn a_conditional_format_may_judge_the_cells_that_hold_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": { "error": "#DIV/0!" } }))
        .expect("the cell is set");
    a_number(&mut session, sheet, "A2", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": "isError", "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(matching(&mut session, sheet, "A1:A2"), vec!["A1".to_string()], "the error matches");
}

/// A value with an operation that compares with **nothing** is refused.
#[test]
fn a_value_with_an_operation_that_compares_with_nothing_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let failure = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": "isEmpty", "value": "x" }),
        )
        .expect_err("the conditional format is refused");
    let message = failure.to_string();
    assert!(message.contains("compares with nothing"), "the refusal says why: {message}");
}

/// A conditional format may judge the **top** of a range: the cells
/// whose number is among the highest of it.
#[test]
fn a_conditional_format_may_judge_the_top_of_a_range() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    a_number(&mut session, sheet, "A3", 3.0);
    session
        .execute(
            "grid.conditional.top10",
            &json!({ "sheet": sheet, "range": "A1:A3", "count": 1, "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(matching(&mut session, sheet, "A1:A3"), vec!["A3".to_string()], "the highest matches");
}

/// The **lowest** values of a range may be the ones the rule holds for.
#[test]
fn the_lowest_values_of_a_range_may_be_the_ones() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    a_number(&mut session, sheet, "A3", 3.0);
    session
        .execute(
            "grid.conditional.top10",
            &json!({ "sheet": sheet, "range": "A1:A3", "count": 1, "bottom": true, "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(matching(&mut session, sheet, "A1:A3"), vec!["A1".to_string()], "the lowest matches");
}

/// The count may be a **percent** of the numbers of the range, and it is
/// rounded **up**.
#[test]
fn the_count_may_be_a_percent_of_the_numbers_of_the_range() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    a_number(&mut session, sheet, "A3", 3.0);
    a_number(&mut session, sheet, "A4", 4.0);
    session
        .execute(
            "grid.conditional.top10",
            &json!({ "sheet": sheet, "range": "A1:A4", "count": 50, "percent": true, "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(
        matching(&mut session, sheet, "A1:A4"),
        vec!["A3".to_string(), "A4".to_string()],
        "the highest half matches"
    );
}

/// A cell that does not hold a number is not one of the top.
#[test]
fn a_cell_that_does_not_hold_a_number_is_not_one_of_the_top() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    session
        .execute(
            "grid.conditional.top10",
            &json!({ "sheet": sheet, "range": "A1:A3", "count": 1, "format": "0.00" }),
        )
        .expect("the conditional format is set");
    assert_eq!(matching(&mut session, sheet, "A1:A3"), vec!["A2".to_string()], "the highest matches");
}

/// A count that is not a whole number of values is refused, and so is a
/// percent over a hundred.
#[test]
fn a_count_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let failure = session
        .execute(
            "grid.conditional.top10",
            &json!({ "sheet": sheet, "range": "A1:A3", "count": 1.5, "format": "0.00" }),
        )
        .expect_err("the conditional format is refused");
    let message = failure.to_string();
    assert!(message.contains("whole number"), "the refusal says why: {message}");
    let failure = session
        .execute(
            "grid.conditional.top10",
            &json!({ "sheet": sheet, "range": "A1:A3", "count": 150, "percent": true, "format": "0.00" }),
        )
        .expect_err("the conditional format is refused");
    let message = failure.to_string();
    assert!(message.contains("a hundred at most"), "the refusal says why: {message}");
}
