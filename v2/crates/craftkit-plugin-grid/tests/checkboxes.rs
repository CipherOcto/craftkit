//! The **checkbox** of a cell.
//!
//! The tree's own rules: an empty cell starts unchecked, a box is
//! toggled, and delete on a checkbox works in two passes.

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

/// A **sheet** in a workbook, and its id.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **cell** a command reads, as the answer holds it.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// A cell that is made a **checkbox** keeps what it holds, and an
/// empty cell starts unchecked.
#[test]
fn a_cell_is_made_a_checkbox() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is a checkbox");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!(false), "the empty cell starts unchecked: {cell:?}");
    assert_eq!(cell["text"], json!("\u{2610}"), "and it shows an empty box: {cell:?}");
    // A cell that holds a **text** keeps it: the tree says that a
    // value that is not a boolean shows as it is.
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": "hello" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "B1" }))
        .expect("the cell is a checkbox");
    let cell = cell_of(&mut session, sheet, "B1");
    assert_eq!(cell["value"], json!("hello"), "the value is kept: {cell:?}");
    assert_eq!(cell["text"], json!("hello"), "and it shows as it is: {cell:?}");
}

/// A checkbox is **toggled**: `TRUE` becomes `FALSE`, `FALSE` becomes
/// `TRUE`, and an empty checkbox cell becomes `TRUE`.
#[test]
fn a_checkbox_is_toggled() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is a checkbox");
    session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the box is toggled");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!(true), "the box is ticked: {cell:?}");
    assert_eq!(cell["text"], json!("\u{2611}"), "and it shows a ticked box: {cell:?}");
    session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the box is toggled");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!(false), "and it is unticked again: {cell:?}");
}

/// A cell that is **not a checkbox** is left as it is, and a cell that
/// holds a formula is not toggled.
#[test]
fn a_cell_that_is_not_a_checkbox_is_left_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "hello" }))
        .expect("the cell is set");
    let outcome = session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the command says it is not a checkbox");
    assert_eq!(outcome.data["toggled"], json!(false), "the cell is not toggled: {outcome:?}");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!("hello"), "and it keeps what it holds: {cell:?}");

    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the cell is a checkbox");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A2", "formula": "=1+1" }))
        .expect("the formula is set");
    let outcome = session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the command says the cell holds a formula");
    assert_eq!(outcome.data["toggled"], json!(false), "the cell is not toggled: {outcome:?}");
}

/// A checkbox that holds a **text** is left as it is when it is
/// toggled: the tree's own rule — only a boolean and an empty cell are
/// toggled.
#[test]
fn a_checkbox_that_holds_a_text_is_left_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "hi" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is a checkbox");
    session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the box is toggled");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!("hi"), "the text is kept: {cell:?}");
    assert_eq!(cell["text"], json!("hi"), "and it shows as it is: {cell:?}");
}

/// **Delete** on a checkbox works in **two passes**: a ticked box is
/// unchecked first, and only then is the box removed.
#[test]
fn delete_on_a_checkbox_works_in_two_passes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is a checkbox");
    session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the box is toggled");
    // The **first** delete unchecks the box.
    session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is cleared");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!(false), "the box is unchecked: {cell:?}");
    assert_eq!(cell["format"], json!("checkbox"), "and it is still a checkbox: {cell:?}");
    // The **second** delete removes the box.
    session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is cleared");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!(null), "the cell is empty: {cell:?}");
    assert_eq!(cell["format"], json!(null), "and the box is gone: {cell:?}");
}

/// **Clear Formats** removes the box and keeps the value: the tree's
/// own rule.
#[test]
fn clear_formats_removes_the_box_and_keeps_the_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.checkbox.set", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is a checkbox");
    session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the box is toggled");
    session
        .execute("grid.cell.clear.format", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the format is cleared");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], json!(true), "the value is kept: {cell:?}");
    assert_eq!(cell["format"], json!(null), "and the box is gone: {cell:?}");
}
