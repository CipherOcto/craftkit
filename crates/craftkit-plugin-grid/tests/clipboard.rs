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

/// The **borders** of a cell, as the answer holds them.
fn borders_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.cell.border.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the borders are read");
    outcome.data["borders"].clone()
}

/// The **borders** of the copied cells are pasted with them.
#[test]
fn the_borders_of_the_copied_cells_are_pasted_with_them() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "A1", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session.execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1" })).expect("the cell is pasted");
    let borders = borders_of(&mut session, sheet, "C1");
    assert_eq!(borders["top"]["style"], json!("thin"), "the border of the first cell is on the second");
}

/// A cell that is pasted over **loses** the border it had.
#[test]
fn a_cell_that_is_pasted_over_loses_the_border_it_had() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "C1", 2.0);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "C1", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "overwrite": true }))
        .expect("the cell is pasted");
    let borders = borders_of(&mut session, sheet, "C1");
    assert_eq!(borders["top"], Json::Null, "the border of the cell that was there is gone");
}

/// A paste of the **formats** writes the look and keeps what the cell
/// holds.
#[test]
fn a_paste_of_the_formats_writes_the_look_and_keeps_what_the_cell_holds() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the font is bold");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    set_number(&mut session, sheet, "C1", 42.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "formats": true }))
        .expect("the formats are pasted");
    let cell = cell_of(&mut session, sheet, "C1");
    assert_eq!(cell["value"], json!(42.0), "the cell still holds what it held");
    assert_eq!(cell["font"]["bold"], json!(true), "and it is bold now");
    assert_eq!(cell["format"], json!("0.00"), "and it has the format");
}

/// A paste of the **formats** may not be given with the values.
#[test]
fn a_paste_of_the_formats_may_not_be_given_with_the_values() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    let outcome = session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "formats": true, "values": true }));
    assert!(outcome.is_err(), "a paste is the values or the formats, and not both");
}

/// A paste of the **formats** writes the borders too.
#[test]
fn a_paste_of_the_formats_writes_the_borders_too() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "A1", "edge": "left", "style": "thick" }),
        )
        .expect("the border is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "formats": true }))
        .expect("the formats are pasted");
    let borders = borders_of(&mut session, sheet, "C1");
    assert_eq!(borders["left"]["style"], json!("thick"), "the border is on the cell");
}

/// The **comment** on a cell, as the answer holds it.
fn comment_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the comment is read");
    outcome.data["comment"].clone()
}

/// The **note** on a cell, as the answer holds it.
fn note_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the note is read");
    outcome.data["note"].clone()
}

/// The **comment** of a copied cell is pasted with it.
#[test]
fn the_comment_of_a_copied_cell_is_pasted_with_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute(
            "grid.cell.comment.set",
            &json!({ "sheet": sheet, "cell": "A1", "text": "a thread", "author": "ada" }),
        )
        .expect("the comment is written");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session.execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1" })).expect("the cell is pasted");
    let comment = comment_of(&mut session, sheet, "C1");
    assert_eq!(comment["text"], json!("a thread"), "the comment is on the cell");
    assert_eq!(comment["author"], json!("ada"), "and so is its author");
}

/// The **note** of a copied cell is pasted with it.
#[test]
fn the_note_of_a_copied_cell_is_pasted_with_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session.execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1" })).expect("the cell is pasted");
    assert_eq!(note_of(&mut session, sheet, "C1")["text"], json!("check this"), "the note is on the cell");
}

/// A cell that is pasted over **loses** the comment it had.
#[test]
fn a_cell_that_is_pasted_over_loses_the_comment_it_had() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_number(&mut session, sheet, "C1", 2.0);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "C1", "text": "a thread" }))
        .expect("the comment is written");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "overwrite": true }))
        .expect("the cell is pasted");
    assert_eq!(
        comment_of(&mut session, sheet, "C1"),
        Json::Null,
        "the comment of the cell that was there is gone"
    );
}

/// A paste of the **values** does not carry the comment.
#[test]
fn a_paste_of_the_values_does_not_carry_the_comment() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "a thread" }))
        .expect("the comment is written");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "values": true }))
        .expect("the values are pasted");
    assert_eq!(comment_of(&mut session, sheet, "C1"), Json::Null, "the comment did not travel");
}

/// A cell of the clipboard that holds **nothing** writes nothing over
/// the cell it lands on.
#[test]
fn a_blank_of_the_clipboard_writes_nothing_over_the_cell_it_lands_on() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // A cell that holds a look and no value: it is a blank that is
    // shown.
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    set_number(&mut session, sheet, "C1", 5.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute(
            "grid.range.paste",
            &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "skip_blanks": true }),
        )
        .expect("the cells are pasted");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!(5.0), "the cell still holds what it held");
}

/// Without the **skip**, a blank of the clipboard writes over the cell
/// it lands on.
#[test]
fn without_the_skip_a_blank_of_the_clipboard_writes_over_the_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    set_number(&mut session, sheet, "C1", 5.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "overwrite": true }))
        .expect("the cells are pasted");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], Json::Null, "the cell holds nothing now");
}

/// The number of the clipboard is put together with the number that is
/// **there**.
#[test]
fn the_number_of_the_clipboard_is_put_together_with_the_number_that_is_there() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 10.0);
    set_number(&mut session, sheet, "C1", 5.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute(
            "grid.range.paste",
            &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "operation": "add" }),
        )
        .expect("the numbers are put together");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!(15.0), "the numbers are added");
}

/// The **multiply** puts the numbers together as well.
#[test]
fn the_multiply_puts_the_numbers_together_as_well() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 10.0);
    set_number(&mut session, sheet, "C1", 5.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute(
            "grid.range.paste",
            &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "operation": "multiply" }),
        )
        .expect("the numbers are put together");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!(50.0), "the numbers are multiplied");
}

/// A cell that holds a **text** keeps it.
#[test]
fn a_cell_that_holds_a_text_keeps_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 10.0);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C1", "value": "five" }))
        .expect("the cell is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute(
            "grid.range.paste",
            &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "operation": "add" }),
        )
        .expect("the numbers are put together");
    assert_eq!(cell_of(&mut session, sheet, "C1")["value"], json!("five"), "the text is still there");
}

/// A cell that holds a **formula** takes the operation into the formula.
#[test]
fn a_cell_that_holds_a_formula_takes_the_operation_into_the_formula() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 2.0);
    set_number(&mut session, sheet, "B1", 3.0);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "C1", "formula": "A1 * 2" }))
        .expect("the formula is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "B1" }))
        .expect("the cell is copied");
    session
        .execute(
            "grid.range.paste",
            &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "operation": "add" }),
        )
        .expect("the numbers are put together");
    assert_eq!(
        formula_of(&mut session, sheet, "C1"),
        Some("(A1 * 2)+3".to_string()),
        "the formula takes the number"
    );
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    assert_eq!(outcome.data["cells"]["C1"], json!(7.0), "and the value is worked out");
}

/// A division by **zero** is an error.
#[test]
fn a_division_by_zero_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 0.0);
    set_number(&mut session, sheet, "C1", 5.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    session
        .execute(
            "grid.range.paste",
            &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "operation": "divide" }),
        )
        .expect("the numbers are put together");
    assert_eq!(
        cell_of(&mut session, sheet, "C1")["value"]["error"],
        json!("#DIV/0!"),
        "the division by zero is an error"
    );
}

/// An **operation** that is not one of the tree's is refused.
#[test]
fn an_operation_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 10.0);
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the cell is copied");
    let outcome = session.execute(
        "grid.range.paste",
        &json!({ "sheet": sheet, "at": "C1", "overwrite": true, "operation": "modulo" }),
    );
    assert!(outcome.is_err(), "an operation is one of the tree's");
}

/// The **validation** of the copied cells is pasted with them.
#[test]
fn the_validation_of_the_copied_cells_is_pasted_with_them() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "kind": "whole",
                "operator": "between",
                "formula1": "1",
                "formula2": "10",
            }),
        )
        .expect("the rule is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:A2" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1" }))
        .expect("the cells are pasted");
    // The rule moved to the cells it landed on.
    let outcome = session
        .execute("grid.validation.check", &json!({ "sheet": sheet, "cell": "C1", "input": "50" }))
        .expect("the cell is checked");
    assert_eq!(outcome.data["ok"], json!(false), "the rule is on the pasted cell");
}

/// A rule that reaches out of the copied range does **not** travel.
#[test]
fn a_rule_that_reaches_out_of_the_copied_range_does_not_travel() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "whole",
                "operator": "between",
                "formula1": "1",
                "formula2": "10",
            }),
        )
        .expect("the rule is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:A2" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1" }))
        .expect("the cells are pasted");
    let outcome = session
        .execute("grid.validation.check", &json!({ "sheet": sheet, "cell": "C1", "input": "50" }))
        .expect("the cell is checked");
    assert_eq!(outcome.data["ok"], json!(true), "the rule did not travel");
}

/// A paste of the **values** does not carry the validation.
#[test]
fn a_paste_of_the_values_does_not_carry_the_validation() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "kind": "whole",
                "operator": "between",
                "formula1": "1",
                "formula2": "10",
            }),
        )
        .expect("the rule is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1:A2" }))
        .expect("the cells are copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1", "values": true }))
        .expect("the values are pasted");
    let outcome = session
        .execute("grid.validation.check", &json!({ "sheet": sheet, "cell": "C1", "input": "50" }))
        .expect("the cell is checked");
    assert_eq!(outcome.data["ok"], json!(true), "the validation did not travel");
}
