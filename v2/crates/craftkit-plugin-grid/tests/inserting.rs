//! The **row that is inserted**.

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

/// **A row may be inserted, and the cells move down.**
#[test]
fn a_row_may_be_inserted_and_the_cells_move_down() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "A2", 2.0);
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], Json::Null, "the first row is empty");
    assert_eq!(cell_of(&mut session, sheet, "A2")["value"], json!(1.0), "the first number moved down");
    assert_eq!(cell_of(&mut session, sheet, "A3")["value"], json!(2.0), "and the second one too");
}

/// **More than one row may be inserted.**
#[test]
fn more_than_one_row_may_be_inserted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A5", 5.0);
    session
        .execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1, "count": 3 }))
        .expect("the rows are inserted");
    assert_eq!(cell_of(&mut session, sheet, "A4")["value"], Json::Null, "the rows are empty");
    assert_eq!(cell_of(&mut session, sheet, "A5")["value"], Json::Null, "and so is this one");
    assert_eq!(
        cell_of(&mut session, sheet, "A8")["value"],
        json!(5.0),
        "and the number moved down three rows"
    );
}

/// The **formula** of a cell that moves is written again.
#[test]
fn the_formula_of_a_cell_that_moves_is_written_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A2", 2.0);
    set_formula(&mut session, sheet, "B1", "=A2");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    assert_eq!(
        cell_of(&mut session, sheet, "B2")["formula"],
        json!("=A3"),
        "the formula names the cell it moved to"
    );
}

/// The formula of a cell that **stays** is written again too.
#[test]
fn the_formula_of_a_cell_that_stays_is_written_again_too() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A5", 5.0);
    set_formula(&mut session, sheet, "B1", "=A5");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 2 })).expect("the row is inserted");
    assert_eq!(cell_of(&mut session, sheet, "B1")["formula"], json!("=A6"), "the reference follows the cell");
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
    set_number(&mut session, other, "A2", 2.0);
    set_formula(&mut session, sheet, "B1", "=Sheet2!A2");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    assert_eq!(
        cell_of(&mut session, sheet, "B2")["formula"],
        json!("=Sheet2!A2"),
        "the rows of the other sheet are not the rows that were inserted"
    );
}

/// An **anchored** reference is not moved.
#[test]
fn an_anchored_reference_is_not_moved() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_formula(&mut session, sheet, "B1", "=$A$5");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    assert_eq!(
        cell_of(&mut session, sheet, "B2")["formula"],
        json!("=$A$5"),
        "the anchored row does not move"
    );
}

/// A **merge** moves with the rows it covers.
#[test]
fn a_merge_moves_with_the_rows_it_covers() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    // The merge is now A2:B3, so the cell B3 is covered by it.
    let refused = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 1.0 }))
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
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "B3" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], json!("check this"), "the note moved with the cell");
}

/// A **hidden row** moves with the rows.
#[test]
fn a_hidden_row_moves_with_the_rows() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 5 })).expect("the row is hidden");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    // The row that was hidden is the sixth now, and hiding it again is
    // refused because it is hidden already.
    let refused = session
        .execute("grid.row.hide", &json!({ "sheet": sheet, "row": 6 }))
        .expect_err("it is hidden already");
    assert!(refused.to_string().contains("already hidden"), "the refusal says why");
}

/// A sheet that holds something this tree **cannot move** is refused.
#[test]
fn a_sheet_that_holds_something_this_tree_cannot_move_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.conditional.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A10",
                "op": ">",
                "value": "5",
                "format": "general",
            }),
        )
        .expect("the condition is set");
    let refused = session
        .execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 }))
        .expect_err("the insert is refused");
    assert!(refused.to_string().contains("conditionals"), "the refusal says what it holds");
}

/// A **protected** sheet refuses.
#[test]
fn a_protected_sheet_refuses() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let refused = session
        .execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 }))
        .expect_err("the insert is refused");
    assert!(refused.to_string().contains("protected"), "the refusal says why");
}

/// A row that is **not a row** is refused.
#[test]
fn a_row_that_is_not_a_row_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let refused = session
        .execute("grid.row.insert", &json!({ "sheet": sheet, "row": 0 }))
        .expect_err("the insert is refused");
    assert!(refused.to_string().contains("counted from one"), "the refusal says why");
}
