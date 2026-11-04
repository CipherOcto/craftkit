//! **Reading** a name.
//!
//! A name is a word that stands for a range of cells, and a name may
//! be read: the range it stands for and the values of the cells of it.

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

/// The **answer** of a read.
fn read(session: &mut Session, workbook: u64, name: &str) -> Json {
    let outcome = session
        .execute("grid.name.get", &json!({ "workbook": workbook, "name": name }))
        .expect("the name is read");
    outcome.data.clone()
}

/// A name may be **read**: the range it stands for and the values of
/// the cells of it.
#[test]
fn a_name_may_be_read() {
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
    let answer = read(&mut session, workbook, "Sales");
    assert_eq!(answer["name"], Json::String("Sales".to_string()), "the name is said");
    assert_eq!(answer["refers_to"], Json::String("Sheet1!A1:A2".to_string()), "and what it stands for");
    assert_eq!(answer["sheet"], json!(sheet), "and the sheet it is in");
    assert_eq!(answer["range"], Json::String("A1:A2".to_string()), "and the range itself");
    assert_eq!(answer["values"], json!([[10.0], [20.0]]), "and the values of the cells");
}

/// The values of a name are **derived**: a cell that holds a formula
/// has the value the formula computes.
#[test]
fn the_values_of_a_name_are_derived() {
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
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A2", "formula": "=A1*2" }))
        .expect("the formula is set");
    let answer = read(&mut session, workbook, "Sales");
    assert_eq!(answer["values"], json!([[10.0], [20.0]]), "the formula cell has the value it computes");
}

/// A name that is **not there** is refused.
#[test]
fn a_name_that_is_not_there_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.get", &json!({ "workbook": workbook, "name": "Sales" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("there is no name `Sales`"), "the refusal says why: {error}");
}

/// A **workbook** that is not there is refused, and so is a node that
/// is not a workbook.
#[test]
fn a_workbook_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.name.get", &json!({ "workbook": 999, "name": "Sales" }))
        .expect_err("the workbook is refused");
    assert!(error.to_string().contains("there is no node `999`"), "the refusal says why: {error}");
    let error = session
        .execute("grid.name.get", &json!({ "workbook": sheet, "name": "Sales" }))
        .expect_err("the sheet is not a workbook");
    assert!(error.to_string().contains("not a workbook"), "the refusal says why: {error}");
}

/// A name whose **sheet is gone** is refused: the name stands for a
/// range of a sheet that is not there.
#[test]
fn a_name_whose_sheet_is_gone_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1" }))
        .expect("the name is written");
    // A workbook must keep one sheet, so another one is added first.
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the second sheet is added");
    session.execute("grid.sheet.remove", &json!({ "sheet": sheet })).expect("the sheet is taken away");
    let error = session
        .execute("grid.name.get", &json!({ "workbook": workbook, "name": "Sales" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("there is no sheet `Sheet1`"), "the refusal says why: {error}");
}
