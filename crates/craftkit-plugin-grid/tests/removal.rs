//! The **taking away** of a sheet.

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

/// The **value** of a cell, as the evaluator computes it.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Option<String> {
    let outcome = session.execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell })).ok()?;
    outcome.data["text"].as_str().map(|text| text.to_string())
}

/// A sheet is **taken away**.
#[test]
fn a_sheet_is_taken_away() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "Ledger");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Two" }))
        .expect("the sheet is added");
    let outcome =
        session.execute("grid.sheet.remove", &json!({ "sheet": sheet })).expect("the sheet is taken away");
    assert_eq!(outcome.data["name"].as_str(), Some("Ledger"), "the name is said");
    assert!(session.document().node(sheet).is_none(), "and the sheet is not there");
}

/// The **last visible sheet** of a workbook may not be taken away.
#[test]
fn the_last_visible_sheet_may_not_be_taken_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session, "Ledger");
    let error = session
        .execute("grid.sheet.remove", &json!({ "sheet": sheet }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("must keep one visible sheet"), "it says why: {message}");
    assert!(session.document().node(sheet).is_some(), "and the sheet is still there");
}

/// A **hidden** sheet may be taken away when another sheet can still be
/// looked at. The tree counts the sheets that are visible and refuses
/// the taking away when there is one of them — so a hidden sheet may
/// not be taken away while it is the last one that can be looked at.
#[test]
fn a_hidden_sheet_may_be_taken_away() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "Ledger");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Two" }))
        .expect("the sheet is added");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Three" }))
        .expect("the sheet is added");
    session.execute("grid.sheet.hide", &json!({ "sheet": sheet })).expect("the sheet is hidden");
    session.execute("grid.sheet.remove", &json!({ "sheet": sheet })).expect("the sheet is taken away");
    assert!(session.document().node(sheet).is_none(), "and the sheet is not there");
}

/// A **hidden** sheet may not be taken away when it is the last sheet
/// that can be looked at: the tree counts the sheets that are visible
/// and refuses the taking away when there is one of them.
#[test]
fn a_hidden_sheet_may_not_be_taken_away_when_it_is_the_last_visible_one() {
    let mut session = session();
    let (workbook, _ledger) = a_sheet(&mut session, "Ledger");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Two" }))
        .expect("the sheet is added");
    let two = session.document().walk().find(|node| node.name == "Two").expect("the sheet is there").id;
    session.execute("grid.sheet.hide", &json!({ "sheet": two })).expect("the sheet is hidden");
    let error = session
        .execute("grid.sheet.remove", &json!({ "sheet": two }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("must keep one visible sheet"), "it says why: {message}");
}

/// A formula that **named** the sheet reads an error that says which
/// sheet is missing.
#[test]
fn a_formula_that_named_the_sheet_reads_an_error() {
    let mut session = session();
    let (workbook, ledger) = a_sheet(&mut session, "Ledger");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Two" }))
        .expect("the sheet is added");
    let two = outcome.data["id"].as_u64().expect("the sheet has an id");
    session
        .execute("grid.formula.set", &json!({ "sheet": two, "cell": "A1", "formula": "=Ledger!B2" }))
        .expect("the formula is set");
    session.execute("grid.sheet.remove", &json!({ "sheet": ledger })).expect("the sheet is taken away");
    let text = value_of(&mut session, two, "A1").expect("the cell has a text");
    assert!(text.contains("there is no sheet `Ledger`"), "it says which sheet is missing: {text}");
}

/// A node that is **not a sheet** may not be taken away.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session, "Ledger");
    let error = session
        .execute("grid.sheet.remove", &json!({ "sheet": workbook }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("not a sheet"), "it says why: {message}");
    let error = session
        .execute("grid.sheet.remove", &json!({ "sheet": 9999 }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}
