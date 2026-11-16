//! The **clipboard** of a workbook.

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

/// **Cells may be copied and pasted, and they go where they are told.**
#[test]
fn cells_may_be_copied_and_pasted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A5" }))
        .expect("the cells are pasted");
    assert_eq!(cell_of(&mut session, sheet, "A5")["value"], json!(1.0), "the first cell is pasted");
    assert_eq!(cell_of(&mut session, sheet, "B5")["value"], json!(2.0), "and the second one");
    assert_eq!(
        cell_of(&mut session, sheet, "A1")["value"],
        json!(1.0),
        "and the ones they came from are still there"
    );
}

/// **A pasted formula is shifted: a relative reference moves with the
/// cell, and an absolute one stays.**
#[test]
fn a_pasted_formula_is_shifted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_formula(&mut session, sheet, "B1", "A1*10");
    set_formula(&mut session, sheet, "C1", "$A$1*10");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "B1:C1" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "B5" }))
        .expect("the cells are pasted");
    assert_eq!(
        formula_of(&mut session, sheet, "B5"),
        Some("A5 * 10".to_string()),
        "the relative reference moved with the cell"
    );
    assert_eq!(
        formula_of(&mut session, sheet, "C5"),
        Some("$A$1 * 10".to_string()),
        "and the absolute one stayed"
    );
}

/// **Cells may be cut, and they are taken away from where they were.**
#[test]
fn cells_may_be_cut_and_they_are_taken_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session.execute("grid.range.cut", &json!({ "sheet": sheet, "range": "A1" })).expect("the cell is cut");
    assert_eq!(cell_of(&mut session, sheet, "A1")["value"], Json::Null, "the cell is gone");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C3" }))
        .expect("the cells are pasted");
    assert_eq!(cell_of(&mut session, sheet, "C3")["value"], json!(1.0), "and it is in the clipboard");
}

/// **The clipboard is the workbook's: cells may be pasted into another
/// sheet.**
#[test]
fn the_clipboard_is_the_workbooks() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let other = another_sheet(&mut session, workbook);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session.execute("grid.range.paste", &json!({ "sheet": other, "at": "A1" })).expect("the cell is pasted");
    assert_eq!(cell_of(&mut session, other, "A1")["value"], json!(1.0), "the cell is in the other sheet");
}

/// **A paste over cells that hold something is refused, unless the
/// paste is told to overwrite.**
#[test]
fn a_paste_over_a_cell_that_holds_something_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "A2", 2.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    let error = session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A2" }))
        .expect_err("the paste is refused");
    assert!(error.to_string().contains("holds something"), "the refusal says why: {error}");
    assert_eq!(cell_of(&mut session, sheet, "A2")["value"], json!(2.0), "and the cell is as it was");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A2", "overwrite": true }))
        .expect("the paste overwrites");
    assert_eq!(cell_of(&mut session, sheet, "A2")["value"], json!(1.0), "and the cell holds what was pasted");
}

/// **A workbook with nothing copied has nothing to paste.**
#[test]
fn an_empty_clipboard_has_nothing_to_paste() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A1" }))
        .expect_err("the paste is refused");
    assert!(error.to_string().contains("clipboard is empty"), "the refusal says why: {error}");
}

/// A paste of **values** drops the formulas and pastes what they give.
#[test]
fn a_paste_of_values_drops_the_formulas() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 2.0);
    set_formula(&mut session, sheet, "A2", "A1 * 3");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:A2" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "values": true }))
        .expect("the values are pasted");
    let cell = cell_of(&mut session, sheet, "C2");
    assert_eq!(cell["value"], json!(6.0), "the value is what the formula gives");
    assert_eq!(cell["formula"], Json::Null, "and the formula is not there");
}

/// The value is computed in the sheet the cells were **copied from**.
#[test]
fn the_value_is_computed_in_the_sheet_the_cells_were_copied_from() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let other = another_sheet(&mut session, workbook);
    set_number(&mut session, sheet, "A1", 4.0);
    set_formula(&mut session, sheet, "A2", "A1 * 5");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A2" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": other, "at": "B1", "values": true }))
        .expect("the value is pasted");
    let cell = cell_of(&mut session, other, "B1");
    assert_eq!(cell["value"], json!(20.0), "the value is the one of the sheet it came from");
    assert_eq!(cell["formula"], Json::Null, "and there is no formula");
}

/// The pasted cell keeps its **look**.
#[test]
fn the_pasted_cell_keeps_its_look() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 2.0);
    set_formula(&mut session, sheet, "A2", "A1 * 3");
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A2", "on": true }))
        .expect("the font is bold");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A2" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "values": true }))
        .expect("the value is pasted");
    let cell = cell_of(&mut session, sheet, "C1");
    assert_eq!(cell["value"], json!(6.0), "the value is there");
    assert_eq!(cell["font"]["bold"], json!(true), "and the cell is still bold");
}

/// A **transposed** paste puts the rows of the clipboard into the
/// columns.
#[test]
fn a_transposed_paste_puts_the_rows_into_the_columns() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    set_number(&mut session, sheet, "A2", 3.0);
    set_number(&mut session, sheet, "B2", 4.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "D1", "transpose": true }))
        .expect("the cells are pasted");
    assert_eq!(cell_of(&mut session, sheet, "D1")["value"], json!(1.0), "the first cell is in the corner");
    assert_eq!(cell_of(&mut session, sheet, "E1")["value"], json!(3.0), "the row becomes a column");
    assert_eq!(cell_of(&mut session, sheet, "D2")["value"], json!(2.0), "and the column becomes a row");
    assert_eq!(
        cell_of(&mut session, sheet, "E2")["value"],
        json!(4.0),
        "and the last cell is in the far corner"
    );
}

/// A transposed paste moves the **formula** with its cell.
#[test]
fn a_transposed_paste_moves_the_formula_with_its_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_formula(&mut session, sheet, "B2", "A1 + 1");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A5", "transpose": true }))
        .expect("the cells are pasted");
    assert_eq!(cell_of(&mut session, sheet, "A5")["value"], json!(1.0), "the number is pasted");
    assert_eq!(
        formula_of(&mut session, sheet, "B6"),
        Some("A5 + 1".to_string()),
        "the formula moved with its cell"
    );
}

/// A paste that would land **off the sheet** is refused.
#[test]
fn a_paste_that_would_land_off_the_sheet_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "B1", 2.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the cells are copied");
    let outcome =
        session.execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A1048576", "transpose": true }));
    assert!(outcome.is_err(), "a cell may not be pushed off the sheet");
}
