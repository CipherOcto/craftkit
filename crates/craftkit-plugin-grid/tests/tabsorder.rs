//! The **order** of the sheets.

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

/// A **workbook** with a sheet in it, and their ids.
fn a_sheet(session: &mut Session, name: &str) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": name }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **sheets** of a workbook, in the order they are in.
fn order_of(session: &Session, workbook: u64) -> Vec<String> {
    session
        .document()
        .node(workbook)
        .expect("the workbook is there")
        .children
        .iter()
        .filter(|child| child.kind == "grid.sheet")
        .map(|child| child.name.clone())
        .collect()
}

/// A sheet may be **moved** in the order.
#[test]
fn a_sheet_may_be_moved() {
    let mut session = session();
    let (workbook, first) = a_sheet(&mut session, "one");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "two" }))
        .expect("the sheet is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "three" }))
        .expect("the sheet is added");
    let third = outcome.data["id"].as_u64().expect("the sheet has an id");
    let outcome =
        session.execute("grid.sheet.move", &json!({ "sheet": third, "to": 1 })).expect("the sheet is moved");
    assert_eq!(outcome.data["to"].as_u64(), Some(1), "the place is said");
    assert_eq!(outcome.data["from"].as_u64(), Some(3), "and where it was");
    assert_eq!(order_of(&session, workbook), vec!["three", "one", "two"], "the order follows");
    assert!(session.document().node(first).is_some(), "and no sheet was lost");
}

/// A sheet may be moved to the **last** place.
#[test]
fn a_sheet_may_be_moved_to_the_last_place() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session, "one");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "two" }))
        .expect("the sheet is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "three" }))
        .expect("the sheet is added");
    let third = outcome.data["id"].as_u64().expect("the sheet has an id");
    session.execute("grid.sheet.move", &json!({ "sheet": third, "to": 3 })).expect("the sheet is moved");
    assert_eq!(order_of(&session, workbook), vec!["one", "two", "three"], "the order follows");
}

/// A place that is **not there** is refused.
#[test]
fn a_place_that_is_not_there_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "one");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "two" }))
        .expect("the sheet is added");
    let error = session
        .execute("grid.sheet.move", &json!({ "sheet": sheet, "to": 5 }))
        .expect_err("the move is refused");
    let message = error.to_string();
    assert!(message.contains("there is no place 5"), "it says why: {message}");
    assert_eq!(order_of(&session, workbook), vec!["one", "two"], "and nothing was moved");
    let error = session
        .execute("grid.sheet.move", &json!({ "sheet": sheet, "to": 0 }))
        .expect_err("the move is refused");
    let message = error.to_string();
    assert!(message.contains("there is no place 0"), "and the place is counted from one: {message}");
}

/// A place that is **not a whole number** is refused.
#[test]
fn a_place_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "one");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "two" }))
        .expect("the sheet is added");
    let error = session
        .execute("grid.sheet.move", &json!({ "sheet": sheet, "to": 1.5 }))
        .expect_err("the move is refused");
    let message = error.to_string();
    assert!(message.contains("whole number"), "it says why: {message}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session, "one");
    let error = session
        .execute("grid.sheet.move", &json!({ "sheet": workbook, "to": 1 }))
        .expect_err("the move is refused");
    let message = error.to_string();
    assert!(message.contains("not a sheet"), "it says why: {message}");
    let error = session
        .execute("grid.sheet.move", &json!({ "sheet": 9999, "to": 1 }))
        .expect_err("the move is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}

/// The order is **saved** with the document.
#[test]
fn the_order_is_saved() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session, "one");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "two" }))
        .expect("the sheet is added");
    let two = outcome.data["id"].as_u64().expect("the sheet has an id");
    session.execute("grid.sheet.move", &json!({ "sheet": two, "to": 1 })).expect("the sheet is moved");
    let text = session.document().to_json().to_string();
    assert!(text.contains("two"), "the sheets are in the document");
    assert_eq!(order_of(&session, workbook), vec!["two", "one"], "and the order is theirs");
}
