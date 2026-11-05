//! The **row that is deleted**.

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

/// A **number** in a cell.
fn set_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the number is set");
}

/// A **formula** in a cell.
fn set_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// What a cell **holds**.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// **A row may be deleted, and the cells below move up.**
#[test]
fn a_row_may_be_deleted_and_the_cells_below_move_up() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "A2", 2.0);
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1 })).expect("the row is deleted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], json!(2.0), "the second number moved up");
    assert_eq!(cell_of(&mut session, sheet, "A2")["value"], Json::Null, "and the second row is empty");
}

/// **More than one row may be deleted.**
#[test]
fn more_than_one_row_may_be_deleted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A5", 5.0);
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1, "count": 3 }))
        .expect("the rows are deleted");
    assert_eq!(cell_of(&mut session, sheet, "A2")["value"], json!(5.0), "the number moved up three rows");
}

/// The **formula** of a cell that moves is written again.
#[test]
fn the_formula_of_a_cell_that_moves_is_written_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A5", 5.0);
    set_formula(&mut session, sheet, "B2", "=A5");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1 })).expect("the row is deleted");
    assert_eq!(
        cell_of(&mut session, sheet, "B1")["formula"],
        json!("=A4"),
        "the formula names the cell it moved to"
    );
}

/// A formula that names a **deleted** row is refused.
#[test]
fn a_formula_that_names_a_deleted_row_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A2", 2.0);
    set_formula(&mut session, sheet, "B1", "=A2");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("names a cell"), "the refusal says why");
}

/// A **note** of a deleted cell goes with the cell.
#[test]
fn a_note_of_a_deleted_cell_goes_with_the_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "B2", "text": "check this" }))
        .expect("the note is written");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 })).expect("the row is deleted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the note is read");
    assert!(outcome.data["note"].is_null(), "the note went with the cell");
}

/// A **note** of a cell below moves up.
#[test]
fn a_note_of_a_cell_below_moves_up() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "B5", "text": "check this" }))
        .expect("the note is written");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1 })).expect("the row is deleted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "B4" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], json!("check this"), "the note moved with the cell");
}

/// A **merge** that covers a deleted row is refused.
#[test]
fn a_merge_that_covers_a_deleted_row_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("merge"), "the refusal says the merge that covers it");
}

/// A **protected** sheet refuses.
#[test]
fn a_protected_sheet_refuses() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("protected"), "the refusal says why");
}

/// A row that is **not a row** is refused.
#[test]
fn a_row_that_is_not_a_row_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 0 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("counted from one"), "the refusal says why");
}
