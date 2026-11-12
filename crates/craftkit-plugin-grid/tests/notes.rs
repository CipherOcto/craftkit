//! The **note** on a cell.

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

/// A **workbook** with a sheet in it, and their ids.
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

/// The **note** on a cell, as the answer holds it.
fn note_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the note is read");
    outcome.data["note"].clone()
}

/// A note is **written** on a cell.
#[test]
fn a_note_is_written_on_a_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.note.set",
            &json!({ "sheet": sheet, "cell": "A1", "text": "check this", "author": "ada" }),
        )
        .expect("the note is written");
    let note = note_of(&mut session, sheet, "A1");
    assert_eq!(note["text"], Json::String("check this".to_string()), "the text is there");
    assert_eq!(note["author"], Json::String("ada".to_string()), "and the author");
}

/// A note that is already there is **edited**, and the author is kept
/// unless a new one is given.
#[test]
fn a_note_that_is_already_there_is_edited() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.note.set",
            &json!({ "sheet": sheet, "cell": "A1", "text": "first", "author": "ada" }),
        )
        .expect("the note is written");
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "second" }))
        .expect("the note is edited");
    let note = note_of(&mut session, sheet, "A1");
    assert_eq!(note["text"], Json::String("second".to_string()), "the text is the new text");
    assert_eq!(note["author"], Json::String("ada".to_string()), "and the author is kept");
}

/// A note **survives** the cell being written to.
#[test]
fn a_note_survives_the_cell_being_written_to() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    let note = note_of(&mut session, sheet, "A1");
    assert_eq!(note["text"], Json::String("check this".to_string()), "the note is still there");
}

/// The note goes with the **whole cell** being cleared.
#[test]
fn the_note_goes_with_the_whole_cell_being_cleared() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is cleared");
    assert_eq!(note_of(&mut session, sheet, "A1"), Json::Null, "and the note is gone");
}

/// The note stays when the **contents** are cleared.
#[test]
fn the_note_stays_when_the_contents_are_cleared() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "check this" }))
        .expect("the note is written");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the contents are cleared");
    let note = note_of(&mut session, sheet, "A1");
    assert_eq!(note["text"], Json::String("check this".to_string()), "and the note is still there");
}

/// A note is **taken away**.
#[test]
fn a_note_is_taken_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "check this" }))
        .expect("the note is written");
    let outcome = session
        .execute("grid.cell.note.delete", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the note is taken away");
    assert_eq!(outcome.data["was_there"], Json::Bool(true), "there was one");
    assert_eq!(note_of(&mut session, sheet, "A1"), Json::Null, "and there is none");
}

/// A cell that holds **no note** is not refused.
#[test]
fn a_cell_that_holds_no_note_is_not_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.note.delete", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("it is not refused");
    assert_eq!(outcome.data["was_there"], Json::Bool(false), "there was none");
}

/// A note with **no text** is refused.
#[test]
fn a_note_with_no_text_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("a note holds text"), "it says why: {error}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.note.set", &json!({ "sheet": workbook, "cell": "A1", "text": "hello" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("not a sheet"), "it says why: {error}");
    let error = session
        .execute("grid.cell.note.get", &json!({ "sheet": 9999, "cell": "A1" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("there is no sheet"), "and there is none: {error}");
}
