//! The **names** of a workbook.
//!
//! A name is a word that stands for a range of cells, and a formula may
//! use the name where it would use the range.

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

/// A **cell** with a number in it.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** of a cell, worked out.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// A name may be **written**, and a formula may use it.
#[test]
fn a_name_may_be_written() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "A2", 20.0);
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1:A2" }),
        )
        .expect("the name is written");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUM(Sales)" }))
        .expect("the formula is set");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(30.0), "the formula sums the range");
}

/// The name is a **range**: a formula that reads it reads the cells.
#[test]
fn a_name_is_a_range() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "A2", 20.0);
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1:A2" }),
        )
        .expect("the name is written");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUM(Sales)" }))
        .expect("the formula is set");
    let outcome = session
        .execute("grid.precedents", &json!({ "sheet": sheet, "cell": "B1" }))
        .expect("the precedents are read");
    let precedents = outcome.data["precedents"].as_array().expect("the precedents are a list");
    assert_eq!(precedents.len(), 1, "the range is one precedent");
    assert_eq!(precedents[0]["area"], "A1:A2", "and it is the range the name stands for");
    assert_eq!(precedents[0]["sheet"], "Sheet1", "in the sheet it is in");
}

/// A name that is **not there** is an error in the cell, and the error
/// says which name is missing.
#[test]
fn a_name_that_is_not_there_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUM(Sales)" }))
        .expect("the formula is set");
    let value = value_of(&mut session, sheet, "B1");
    assert_eq!(value, json!({ "error": "there is no name `Sales`" }), "the cell says so");
}

/// A name may be **taken away**, and the formulas that used it say so.
#[test]
fn a_name_may_be_taken_away() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1" }))
        .expect("the name is written");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUM(Sales)" }))
        .expect("the formula is set");
    session
        .execute("grid.name.delete", &json!({ "workbook": workbook, "name": "Sales" }))
        .expect("the name is taken away");
    assert_eq!(
        value_of(&mut session, sheet, "B1"),
        json!({ "error": "there is no name `Sales`" }),
        "and the formula says so"
    );
}

/// A name that is **not there** may not be taken away.
#[test]
fn a_name_that_is_not_there_may_not_be_taken_away() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.delete", &json!({ "workbook": workbook, "name": "Sales" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("there is no name `Sales`"), "the error says so: {error}");
}

/// A name may be **written again**, and it stands for the new range.
#[test]
fn a_name_may_be_written_again() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "A2", 20.0);
    session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1" }))
        .expect("the name is written");
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1:A2" }),
        )
        .expect("the name is written again");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=SUM(Sales)" }))
        .expect("the formula is set");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(30.0), "the formula sums the new range");
}

/// A name that looks like a **cell** is refused: it would be a cell.
#[test]
fn a_name_that_looks_like_a_cell_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "A1", "refers_to": "Sheet1!A1" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("looks like a cell"), "the error says so: {error}");
}

/// A name that looks like an **R1C1 reference** is refused.
#[test]
fn a_name_that_looks_like_r1c1_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "R1C1", "refers_to": "Sheet1!A1" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("R1C1"), "the error says so: {error}");
}

/// A name that is one of the **constants** is refused.
#[test]
fn a_name_that_is_a_constant_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "TRUE", "refers_to": "Sheet1!A1" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("constants"), "the error says so: {error}");
}

/// A name that does not **begin** with a letter is refused.
#[test]
fn a_name_that_does_not_begin_with_a_letter_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "1Sales", "refers_to": "Sheet1!A1" }),
        )
        .expect_err("the name is refused");
    assert!(error.to_string().contains("begins with"), "the error says so: {error}");
}

/// A range that does not name its **sheet** is refused.
#[test]
fn a_range_that_does_not_name_its_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "A1:A2" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("names its sheet"), "the error says so: {error}");
}

/// A range in a sheet that is **not there** is refused, and the refusal
/// says what the sheets are.
#[test]
fn a_range_in_a_sheet_that_is_not_there_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet2!A1" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("there is no sheet `Sheet2`"), "the error says so: {error}");
    assert!(error.to_string().contains("Sheet1"), "and it says what the sheets are");
}

/// A node that is **not a workbook** is refused.
#[test]
fn a_node_that_is_not_a_workbook_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.set", &json!({ "workbook": sheet, "name": "Sales", "refers_to": "Sheet1!A1" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("not a workbook"), "the error says so: {error}");
}

/// A name is **workbook**-wide: a formula in another sheet may use it.
#[test]
fn a_name_is_workbook_wide() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the other sheet is added");
    let other = outcome.data["id"].as_u64().expect("the other sheet has an id");
    a_number(&mut session, sheet, "A1", 7.0);
    session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1" }))
        .expect("the name is written");
    session
        .execute("grid.formula.set", &json!({ "sheet": other, "cell": "A1", "formula": "=Sales" }))
        .expect("the formula is set");
    assert_eq!(value_of(&mut session, other, "A1"), json!("1 values"), "the other sheet reads the name");
}

/// The name is **saved**: it is in the document after it is done.
#[test]
fn the_name_is_saved() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1:B2" }),
        )
        .expect("the name is written");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(workbook).expect("the workbook is in the document");
    assert_eq!(node.data["names"]["Sales"], "Sheet1!A1:B2", "and the name is there");
}
