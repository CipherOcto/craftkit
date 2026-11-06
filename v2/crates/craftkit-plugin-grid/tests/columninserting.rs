//! The **column that is inserted**.

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

/// **A column may be inserted, and the cells move right.**
#[test]
fn a_column_may_be_inserted_and_the_cells_move_right() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], Json::Null, "the first column is empty");
    assert_eq!(cell_of(&mut session, sheet, "B1")["value"], json!(1.0), "the first number moved right");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!(2.0), "and the second one too");
}

/// **More than one column may be inserted.**
#[test]
fn more_than_one_column_may_be_inserted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "C1", 3.0);
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A", "count": 2 }))
        .expect("the columns are inserted");
    assert_eq!(cell_of(&mut session, sheet, "E1")["value"], json!(3.0), "the number moved right two columns");
}

/// The **formula** of a cell that moves is written again.
#[test]
fn the_formula_of_a_cell_that_moves_is_written_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "B2", 2.0);
    set_formula(&mut session, sheet, "A1", "=B2");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    assert_eq!(
        cell_of(&mut session, sheet, "B1")["formula"],
        json!("=C2"),
        "the formula names the cell it moved to"
    );
}

/// The formula of a cell that **stays** is written again too.
#[test]
fn the_formula_of_a_cell_that_stays_is_written_again_too() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "C1", 5.0);
    set_formula(&mut session, sheet, "A1", "=C1");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "B" }))
        .expect("the column is inserted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["formula"], json!("=D1"), "the reference follows the cell");
}

/// A reference that names **another sheet** is not moved.
#[test]
fn a_reference_that_names_another_sheet_is_not_moved() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    set_number(&mut session, other, "B2", 2.0);
    set_formula(&mut session, sheet, "A1", "=Sheet2!B2");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    assert_eq!(
        cell_of(&mut session, sheet, "B1")["formula"],
        json!("=Sheet2!B2"),
        "the columns of the other sheet are not the columns that were inserted"
    );
}

/// An **anchored** column does not move.
#[test]
fn an_anchored_column_does_not_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_formula(&mut session, sheet, "C1", "=$A5");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    assert_eq!(
        cell_of(&mut session, sheet, "D1")["formula"],
        json!("=$A5"),
        "the anchored column does not move"
    );
}

/// A **merge** moves with the columns it covers.
#[test]
fn a_merge_moves_with_the_columns_it_covers() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    // The merge is now B1:C2, so the cell C2 is covered by it.
    let refused = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C2", "value": 1.0 }))
        .expect_err("the covered cell is refused");
    assert!(refused.to_string().contains("merge"), "the refusal says the merge that covers it");
}

/// A **note** moves with its cell.
#[test]
fn a_note_moves_with_its_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "B2", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "C2" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], json!("check this"), "the note moved with the cell");
}

/// The **width** of a column moves with the column.
#[test]
fn the_width_of_a_column_moves_with_the_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "B", "width": 20.0 }))
        .expect("the width is set");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["columns"]["C"], json!(20.0), "the width moved with the column");
}

/// A **hidden column** moves with the columns.
#[test]
fn a_hidden_column_moves_with_the_columns() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.hide", &json!({ "sheet": sheet, "column": "C" }))
        .expect("the column is hidden");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let outcome = session.execute("grid.sheet.get", &json!({ "sheet": sheet })).expect("the sheet is read");
    assert_eq!(outcome.data["hidden_columns"], json!(["D"]), "the hidden column moved right");
}

/// The **outline** of a column moves with the column.
#[test]
fn the_outline_of_a_column_moves_with_the_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "B:C" }))
        .expect("the columns are grouped");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["column_outline"]["C"], json!(1), "the level moved with the column");
    assert_eq!(payload["column_outline"]["D"], json!(1), "and so did the other one");
}

/// The **filter** moves with the column it filters.
#[test]
fn the_filter_moves_with_the_column_it_filters() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "B", "op": ">", "value": "10" }))
        .expect("the filter is set");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let payload = payload(&session, sheet);
    assert_eq!(payload["filter"]["column"], json!("C"), "the filter moved with the column");
}

/// The **area** a sheet prints moves with the columns.
#[test]
fn the_area_a_sheet_prints_moves_with_the_columns() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.print.area", &json!({ "sheet": sheet, "range": "A1:C10" }))
        .expect("the area is set");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the print is read");
    assert_eq!(outcome.data["area"], json!("B1:D10"), "the area moved right");
}

/// The **titles** of a print move with the columns.
#[test]
fn the_titles_of_a_print_move_with_the_columns() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.print.titles", &json!({ "sheet": sheet, "columns": "B:B" }))
        .expect("the titles are set");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the print is read");
    assert_eq!(outcome.data["title_columns"], json!(["C"]), "the title moved right");
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
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect_err("the insert is refused");
    assert!(refused.to_string().contains("chart"), "the refusal says what it holds");
}

/// A **protected** sheet refuses.
#[test]
fn a_protected_sheet_refuses() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let refused = session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect_err("the insert is refused");
    assert!(refused.to_string().contains("protected"), "the refusal says why");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let refused = session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A1" }))
        .expect_err("the insert is refused");
    assert!(refused.to_string().contains("not a column"), "the refusal says why");
}
