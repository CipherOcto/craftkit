//! The **column that moves**.

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

/// A **second sheet** in the workbook.
fn another_sheet(session: &mut Session, workbook: u64) -> u64 {
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
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

/// The **formula** of a cell.
fn formula_of(session: &mut Session, sheet: u64, cell: &str) -> Option<String> {
    cell_of(session, sheet, cell)["formula"].as_str().map(|text| text.to_string())
}

/// The **payload** of the sheet: the facts it holds.
fn payload(session: &Session, sheet: u64) -> Json {
    session.document().node(sheet).expect("the sheet is there").data.clone()
}

/// **A column may be moved, and the cells go with it.**
#[test]
fn a_column_may_be_moved_and_the_cells_go_with_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    set_number(&mut session, sheet, "C1", 3.0);
    session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect("the column is moved");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], json!(1.0), "the first column stays");
    assert_eq!(
        cell_of(&mut session, sheet, "B1")["value"],
        json!(3.0),
        "the column that was third is second"
    );
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!(2.0), "and the one that moved is third");
    assert_eq!(cell_of(&mut session, sheet, "D1")["value"], Json::Null, "and the fourth is empty");
}

/// **A formula follows the cell it names.**
#[test]
fn a_formula_follows_the_cell_it_names() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "B1", 2.0);
    set_formula(&mut session, sheet, "E1", "B1*10");
    session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect("the column is moved");
    assert_eq!(
        formula_of(&mut session, sheet, "E1"),
        Some("C1 * 10".to_string()),
        "the formula names the cell's new place"
    );
}

/// **A formula in another sheet follows too.**
#[test]
fn a_formula_in_another_sheet_follows_too() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let other = another_sheet(&mut session, workbook);
    set_number(&mut session, sheet, "B1", 2.0);
    set_formula(&mut session, other, "A1", "Sheet1!B1*10");
    session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect("the column is moved");
    assert_eq!(
        formula_of(&mut session, other, "A1"),
        Some("Sheet1!C1 * 10".to_string()),
        "the formula of the other sheet names the cell's new place"
    );
}

/// **A range that lies wholly in the moved column follows it.**
#[test]
fn a_range_that_lies_in_the_moved_column_follows_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_formula(&mut session, sheet, "E1", "SUM(B1:B5)");
    session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect("the column is moved");
    assert_eq!(
        formula_of(&mut session, sheet, "E1"),
        Some("SUM(C1:C5)".to_string()),
        "the range names the column's new place"
    );
}

/// **A range with an edge on the moved column names a column that is
/// not there any more: it is the error `#REF!`.**
#[test]
fn a_range_with_an_edge_on_the_moved_column_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_formula(&mut session, sheet, "E1", "SUM(A1:B5)");
    session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect("the column is moved");
    assert_eq!(formula_of(&mut session, sheet, "E1"), Some("SUM(#REF!)".to_string()), "the formula says so");
}

/// **A fact that names the moved column cannot be written, and the move
/// is refused.**
#[test]
fn a_merge_that_names_the_moved_column_refuses_the_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.across", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the cells are merged");
    let error = session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect_err("the merge cannot be written");
    assert!(error.to_string().contains("A1:B1"), "the refusal says what it is: {error}");
    assert_eq!(
        payload(&session, sheet)["merges"],
        json!(["A1:B1".to_string()] as [String; 1]),
        "and the sheet is as it was"
    );
}

/// **The widths of the columns move with them.**
#[test]
fn the_widths_of_the_columns_move_with_them() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "B", "width": 20.0 }))
        .expect("the width is set");
    session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect("the column is moved");
    let payload = payload(&session, sheet);
    assert_eq!(payload["columns"]["C"], json!(20.0), "the width went with the column");
    assert!(payload["columns"].get("B").is_none(), "and it is not on the column it was");
}

/// **A column may not go before itself: a move that moves nothing is
/// not a move.**
#[test]
fn a_column_may_not_go_before_itself() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "B" }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("already before"), "the refusal says why: {error}");
}

/// **A protected sheet may not have a column moved in it.**
#[test]
fn a_protected_sheet_refuses_the_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let error = session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "B", "before": "D" }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("protected"), "the refusal says why: {error}");
}

/// **A column that is not a column is refused.**
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.column.move", &json!({ "sheet": sheet, "column": "1", "before": "D" }))
        .expect_err("the column is refused");
    assert!(error.to_string().contains("not a column"), "the refusal says why: {error}");
}
