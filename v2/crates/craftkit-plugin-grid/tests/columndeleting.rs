//! The **column that is deleted**.

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

/// The **payload** of the sheet: the facts it holds.
fn payload(session: &Session, sheet: u64) -> Json {
    session.document().node(sheet).expect("the sheet is there").data.clone()
}

/// **A column may be deleted, and the cells move left.**
#[test]
fn a_column_may_be_deleted_and_the_cells_move_left() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "C1", 3.0);
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect("the column is deleted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], json!(1.0), "the first number stayed");
    assert_eq!(cell_of(&mut session, sheet, "B1")["value"], json!(3.0), "the second one moved left");
}

/// **More than one column may be deleted.**
#[test]
fn more_than_one_column_may_be_deleted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "E1", 5.0);
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B", "count": 2 }))
        .expect("the columns are deleted");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!(5.0), "the number moved left two columns");
}

/// The **formula** of a cell that moves is written again.
#[test]
fn the_formula_of_a_cell_that_moves_is_written_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "D1", 4.0);
    set_formula(&mut session, sheet, "C1", "=D1");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is deleted");
    assert_eq!(
        cell_of(&mut session, sheet, "B1")["formula"],
        json!("=C1"),
        "the formula names the cell it moved to"
    );
}

/// A **formula that names a deleted column** refuses the delete.
#[test]
fn a_formula_that_names_a_deleted_column_refuses_the_delete() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("columns that are deleted"), "the refusal says why");
}

/// The **note** of a deleted cell goes with the cell.
#[test]
fn the_note_of_a_deleted_cell_goes_with_the_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "B2", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect("the column is deleted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the cell is read");
    assert_eq!(outcome.data["note"], Json::Null, "the note went with the cell");
}

/// The **note** of a cell to the right moves left.
#[test]
fn the_note_of_a_cell_to_the_right_moves_left() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "C2", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is deleted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], json!("check this"), "the note moved left with the cell");
}

/// A **merge** that covers a deleted column refuses the delete.
#[test]
fn a_merge_that_covers_a_deleted_column_refuses_the_delete() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:C2" }))
        .expect("the range is merged");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("merge"), "the refusal says the merge that covers it");
}

/// The **width** of a column moves left, and the width of a deleted
/// column goes with it.
#[test]
fn the_width_of_a_column_moves_left() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "B", "width": 20.0 }))
        .expect("the width is set");
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "C", "width": 30.0 }))
        .expect("the width is set");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is deleted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["columns"]["A"], json!(20.0), "the width moved left with the column");
    assert_eq!(payload["columns"]["B"], json!(30.0), "and so did the other one");
}

/// A **hidden column** moves left.
#[test]
fn a_hidden_column_moves_left() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "C" }))
        .expect("the column is hidden");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is deleted");
    let outcome = session.execute("grid.sheet.get", &json!({ "sheet": sheet })).expect("the sheet is read");
    assert_eq!(outcome.data["hidden_columns"], json!(["B"]), "the hidden column moved left");
}

/// The **area** a sheet prints moves left.
#[test]
fn the_area_a_sheet_prints_moves_left() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.print.area", &json!({ "sheet": sheet, "range": "B1:D10" }))
        .expect("the area is set");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is deleted");
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the print is read");
    assert_eq!(outcome.data["area"], json!("A1:C10"), "the area moved left");
}

/// The **filter** of a deleted column refuses the delete.
#[test]
fn the_filter_of_a_deleted_column_refuses_the_delete() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "B", "op": ">", "value": "10" }))
        .expect("the filter is set");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("filter"), "the refusal says the filter");
}

/// A sheet that holds something this tree **cannot move** is refused.
#[test]
fn a_sheet_that_holds_something_this_tree_cannot_move_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": "A1:C5" }))
        .expect("the chart is added");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("chart"), "the refusal says what it holds");
}

/// A **protected** sheet refuses.
#[test]
fn a_protected_sheet_refuses() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "A" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("protected"), "the refusal says why");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let refused = session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "1" }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("not a column"), "the refusal says why");
}
