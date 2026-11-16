//! The **cells that are inserted** and the **cells that are deleted**.

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

/// The **formula** of a cell.
fn formula_of(session: &mut Session, sheet: u64, cell: &str) -> Option<String> {
    cell_of(session, sheet, cell)["formula"].as_str().map(|text| text.to_string())
}

/// **Cells may be inserted, and the cells beside them move — within the
/// rows of the range.**
#[test]
fn cells_may_be_inserted_and_the_cells_beside_them_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    set_number(&mut session, sheet, "C1", 3.0);
    set_number(&mut session, sheet, "A2", 4.0);
    set_number(&mut session, sheet, "B2", 5.0);
    session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "B1", "shift": "right" }))
        .expect("the cell is inserted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], json!(1.0), "the cell before the band stays");
    assert_eq!(cell_of(&mut session, sheet, "B1")["value"], Json::Null, "and the band is empty");
    assert_eq!(
        cell_of(&mut session, sheet, "C1")["value"],
        json!(2.0),
        "the cell that was in the band moved right"
    );
    assert_eq!(cell_of(&mut session, sheet, "D1")["value"], json!(3.0), "and the one after it moved too");
    assert_eq!(
        cell_of(&mut session, sheet, "B2")["value"],
        json!(5.0),
        "the row below the band did not move"
    );
}

/// **Cells may be inserted downwards, and only the columns of the range
/// move.**
#[test]
fn cells_may_be_inserted_downwards() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "A2", 2.0);
    set_number(&mut session, sheet, "B1", 3.0);
    session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "A1", "shift": "down" }))
        .expect("the cell is inserted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], Json::Null, "the band is empty");
    assert_eq!(
        cell_of(&mut session, sheet, "A2")["value"],
        json!(1.0),
        "the cell that was in the band moved down"
    );
    assert_eq!(
        cell_of(&mut session, sheet, "B1")["value"],
        json!(3.0),
        "the column beside the band did not move"
    );
}

/// **A formula within the band follows the cells it names.**
#[test]
fn a_formula_within_the_band_follows_the_cells() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "B1", 2.0);
    set_formula(&mut session, sheet, "A1", "B1*10");
    session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "B1", "shift": "right" }))
        .expect("the cell is inserted");
    assert_eq!(
        formula_of(&mut session, sheet, "A1"),
        Some("C1 * 10".to_string()),
        "the formula names the cell's new place"
    );
}

/// **A formula that reaches outside the band does not move: the cells
/// it names are not the cells that moved.**
#[test]
fn a_formula_that_reaches_outside_the_band_does_not_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "B2", 2.0);
    set_formula(&mut session, sheet, "A1", "B2*10");
    session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "B1", "shift": "right" }))
        .expect("the cell is inserted");
    assert_eq!(
        formula_of(&mut session, sheet, "A1"),
        Some("B2*10".to_string()),
        "the formula names the cell it named before, and it is written as it was"
    );
}

/// **Cells may be deleted, and the cells beside them move back.**
#[test]
fn cells_may_be_deleted_and_the_cells_beside_them_move_back() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    set_number(&mut session, sheet, "C1", 3.0);
    set_number(&mut session, sheet, "B2", 4.0);
    session
        .execute("grid.cell.delete", &json!({ "sheet": sheet, "range": "B1", "shift": "left" }))
        .expect("the cell is deleted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], json!(1.0), "the cell before the band stays");
    assert_eq!(cell_of(&mut session, sheet, "B1")["value"], json!(3.0), "the cell after the band moved back");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], Json::Null, "and the one after it moved too");
    assert_eq!(
        cell_of(&mut session, sheet, "B2")["value"],
        json!(4.0),
        "the row below the band did not move"
    );
}

/// **A formula that names a cell that is deleted is the error
/// `#REF!`.**
#[test]
fn a_formula_that_names_a_deleted_cell_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "B1*10");
    session
        .execute("grid.cell.delete", &json!({ "sheet": sheet, "range": "B1", "shift": "left" }))
        .expect("the cell is deleted");
    assert_eq!(formula_of(&mut session, sheet, "A1"), Some("#REF! * 10".to_string()), "the formula says so");
}

/// **A protected sheet may not have a cell inserted in it.**
#[test]
fn a_protected_sheet_refuses_the_insert() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let error = session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "B1", "shift": "right" }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("protected"), "the refusal says why: {error}");
}

/// **A way that is not a way is refused, and so is a range that is not
/// a range.**
#[test]
fn a_way_that_is_not_a_way_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "B1", "shift": "sideways" }))
        .expect_err("the shift is refused");
    assert!(error.to_string().contains("not a way"), "the refusal says why: {error}");
    let error = session
        .execute("grid.cell.insert", &json!({ "sheet": sheet, "range": "one", "shift": "right" }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("not a range of cells"), "the refusal says why: {error}");
}
