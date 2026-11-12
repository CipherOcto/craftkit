//! The **comment** on a cell: a thread of replies.

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

/// The **comment** on a cell, as the answer holds it.
fn comment_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the comment is read");
    outcome.data["comment"].clone()
}

/// A comment is **written** on a cell.
#[test]
fn a_comment_is_written_on_a_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.comment.set",
            &json!({ "sheet": sheet, "cell": "A1", "text": "check this", "author": "ada" }),
        )
        .expect("the comment is written");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment["text"], Json::String("check this".to_string()), "the text is there");
    assert_eq!(comment["author"], Json::String("ada".to_string()), "and the author");
    assert_eq!(comment["resolved"], Json::Bool(false), "and the thread is open");
    assert_eq!(comment["replies"].as_array().map(|replies| replies.len()), Some(0), "and no replies");
}

/// A comment that is already there is **edited**: the text is the
/// new text, the author is kept unless a new one is given, and the
/// replies stay. The tree's own rule.
#[test]
fn a_comment_that_is_already_there_is_edited() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.comment.set",
            &json!({ "sheet": sheet, "cell": "A1", "text": "first", "author": "ada" }),
        )
        .expect("the comment is written");
    session
        .execute("grid.cell.comment.reply", &json!({ "sheet": sheet, "cell": "A1", "text": "a reply" }))
        .expect("the reply is written");
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "edited" }))
        .expect("the comment is edited");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment["text"], Json::String("edited".to_string()), "the text is the new text");
    assert_eq!(comment["author"], Json::String("ada".to_string()), "the author is kept");
    assert_eq!(comment["replies"].as_array().map(|replies| replies.len()), Some(1), "the reply stays");
}

/// A **reply** joins the thread of the cell.
#[test]
fn a_reply_joins_the_thread_of_the_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "first" }))
        .expect("the comment is written");
    session
        .execute(
            "grid.cell.comment.reply",
            &json!({ "sheet": sheet, "cell": "A1", "text": "a reply", "author": "bob" }),
        )
        .expect("the reply is written");
    let comment = comment_of(&mut session, sheet, "A1");
    let replies = comment["replies"].as_array().expect("the replies are a list");
    assert_eq!(replies.len(), 1, "there is one reply: {comment:?}");
    assert_eq!(replies[0]["text"], Json::String("a reply".to_string()), "and its text");
    assert_eq!(replies[0]["author"], Json::String("bob".to_string()), "and its author");
}

/// A cell that has **no comment** has no thread to join, and that is
/// refused by name.
#[test]
fn a_reply_to_a_cell_without_a_comment_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.comment.reply", &json!({ "sheet": sheet, "cell": "A1", "text": "a reply" }));
    assert!(outcome.is_err(), "the reply is refused: {outcome:?}");
}

/// A thread may be **resolved**, and unresolved again.
#[test]
fn a_thread_is_resolved_and_unresolved_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "first" }))
        .expect("the comment is written");
    session
        .execute("grid.cell.comment.resolve", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the thread is resolved");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment["resolved"], Json::Bool(true), "the thread is resolved: {comment:?}");
    session
        .execute("grid.cell.comment.resolve", &json!({ "sheet": sheet, "cell": "A1", "resolved": false }))
        .expect("the thread is unresolved");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment["resolved"], Json::Bool(false), "and unresolved again: {comment:?}");
}

/// A **note replaces a comment** and a comment replaces a note: the
/// tree's own rule.
#[test]
fn a_note_replaces_a_comment_and_a_comment_replaces_a_note() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "first" }))
        .expect("the comment is written");
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "a note" }))
        .expect("the note is written");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment, Json::Null, "the comment is gone: {comment:?}");
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "again" }))
        .expect("the comment is written");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"], Json::Null, "and the note is gone: {outcome:?}");
}

/// The **whole cell cleared** takes the comment with it: the tree's
/// *clear all* takes the comment of the cell with it.
#[test]
fn clearing_the_cell_takes_the_comment_with_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "first" }))
        .expect("the comment is written");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is emptied");
    let comment = comment_of(&mut session, sheet, "A1");
    assert_eq!(comment, Json::Null, "the comment is gone: {comment:?}");
}
