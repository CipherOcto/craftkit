//! The **custom lists** of a workbook.
//!
//! The tree's own lists are the days of the week, the months and the
//! quarters. A workbook may carry lists of your own, and a fill steps
//! through one when the text beside it is in it.

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

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> (u64, u64) {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    (1, outcome.data["id"].as_u64().expect("the sheet has an id"))
}

/// Put a **text** in a cell.
fn put(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// The **text** of a cell, as the payload holds it.
fn text_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].as_str()?.to_string())
}

/// A **custom list** may be added to a workbook.
#[test]
fn a_custom_list_may_be_added() {
    let mut session = session();
    let (book, _) = a_sheet(&mut session);
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red\ngreen\nblue" }))
        .expect("the list is added");
    let node = session.document().node(book).expect("the workbook is there");
    let lists = node.data["custom_lists"].as_array().expect("the lists are there");
    assert_eq!(lists.len(), 1, "the workbook has a list");
    assert_eq!(lists[0].as_array().expect("the list is a list").len(), 3, "of three words");
}

/// A fill steps through a **custom list**.
#[test]
fn a_fill_steps_through_a_custom_list() {
    let mut session = session();
    let (book, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red\ngreen\nblue" }))
        .expect("the list is added");
    put(&mut session, sheet, "A1", "green");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A1").as_deref(), Some("green"), "the source is there");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("blue"), "the next word of the list");
    assert_eq!(text_of(&session, sheet, "A3").as_deref(), Some("red"), "and the one after that");
}

/// A custom list **wraps** after its last word.
#[test]
fn a_custom_list_wraps() {
    let mut session = session();
    let (book, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red\ngreen" }))
        .expect("the list is added");
    put(&mut session, sheet, "A1", "green");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("red"), "the list wraps");
    assert_eq!(text_of(&session, sheet, "A3").as_deref(), Some("green"), "and wraps again");
}

/// A custom list is found by **membership**, and the case of the
/// source is the case of the list.
#[test]
fn a_custom_list_is_found_by_membership() {
    let mut session = session();
    let (book, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "Red\nGreen\nBlue" }))
        .expect("the list is added");
    put(&mut session, sheet, "A1", "green");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("Blue"), "the list keeps its own case");
}

/// A custom list **shadows** one of the tree's own: the tree tries
/// the lists of the workbook first.
#[test]
fn a_custom_list_shadows_a_tree_list() {
    let mut session = session();
    let (book, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "Mon\nTue\nWed" }))
        .expect("the list is added");
    put(&mut session, sheet, "A1", "Tue");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("Wed"), "the own list is used");
}

/// A list of **one** word is not a list.
#[test]
fn a_list_of_one_word_is_refused() {
    let mut session = session();
    let (book, _) = a_sheet(&mut session);
    let outcome = session.execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red" }));
    assert!(outcome.is_err(), "the list of one word is refused");
}

/// A word that is in the list **twice** is refused: the fill would
/// find the first of them and never the second.
#[test]
fn a_word_twice_is_refused() {
    let mut session = session();
    let (book, _) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red\ngreen\nRED" }));
    assert!(outcome.is_err(), "the word twice is refused");
}

/// A custom list may be **taken away**.
#[test]
fn a_custom_list_may_be_taken_away() {
    let mut session = session();
    let (book, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red\ngreen" }))
        .expect("the list is added");
    session
        .execute("grid.workbook.list.remove", &json!({ "workbook": book, "list": 1 }))
        .expect("the list is taken away");
    let node = session.document().node(book).expect("the workbook is there");
    let lists = node.data["custom_lists"].as_array().expect("the lists are there");
    assert!(lists.is_empty(), "the workbook has no lists");
    put(&mut session, sheet, "A1", "green");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("green"), "and the fill copies now");
}

/// A list that is **not there** may not be taken away.
#[test]
fn a_list_that_is_not_there_is_refused() {
    let mut session = session();
    let (book, _) = a_sheet(&mut session);
    let outcome = session.execute("grid.workbook.list.remove", &json!({ "workbook": book, "list": 3 }));
    assert!(outcome.is_err(), "the list that is not there is refused");
}

/// The lists belong to the **workbook**: another book keeps its own.
#[test]
fn the_lists_belong_to_the_workbook() {
    let mut session = session();
    let (book, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.add", &json!({ "name": "the other book" }))
        .expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 2, "name": "Sheet1" }))
        .expect("the sheet is added");
    let other_sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    session
        .execute("grid.workbook.list.add", &json!({ "workbook": book, "words": "red\ngreen" }))
        .expect("the list is added");
    put(&mut session, sheet, "A1", "green");
    put(&mut session, other_sheet, "A1", "green");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    session
        .execute("grid.range.fill", &json!({ "sheet": other_sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("red"), "the book steps through its list");
    assert_eq!(text_of(&session, other_sheet, "A2").as_deref(), Some("green"), "and the other book copies");
}
