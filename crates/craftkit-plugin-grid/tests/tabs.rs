//! The **tabs** of a workbook: the sheets that are hidden, and the
//! colour of a sheet's tab.

use craftkit_sdk::serde_json::{Value, json};
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

/// A **workbook** with two **sheets** in it, and their ids.
fn two_sheets(session: &mut Session) -> (u64, u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "one" }))
        .expect("the sheet is added");
    let one = outcome.data["id"].as_u64().expect("the sheet has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "two" }))
        .expect("the sheet is added");
    let two = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, one, two)
}

/// Whether a sheet is **hidden**, as the document holds it.
fn hidden_of(session: &Session, sheet: u64) -> bool {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data.get("hidden").and_then(|hidden| hidden.as_bool()).unwrap_or(false)
}

/// The **colour** of a sheet's tab, as the document holds it.
fn colour_of(session: &Session, sheet: u64) -> Value {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data.get("tab_color").cloned().unwrap_or(Value::Null)
}

/// A sheet may be **hidden**.
#[test]
fn a_sheet_may_be_hidden() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    session.execute("grid.sheet.hide", &json!({ "sheet": one })).expect("the sheet is hidden");
    assert!(hidden_of(&session, one), "the sheet is hidden");
}

/// The last **visible** sheet may not be hidden: a workbook with no
/// sheet to look at is not a workbook.
#[test]
fn the_last_visible_sheet_may_not_be_hidden() {
    let mut session = session();
    let (_, one, two) = two_sheets(&mut session);
    session.execute("grid.sheet.hide", &json!({ "sheet": two })).expect("the sheet is hidden");
    let error = session
        .execute("grid.sheet.hide", &json!({ "sheet": one }))
        .expect_err("a workbook must keep one visible sheet");
    assert!(
        error.to_string().contains("one visible sheet"),
        "the error says a workbook must keep one visible sheet: {error}"
    );
    assert!(!hidden_of(&session, one), "and the sheet is still there");
}

/// A sheet may be **unhidden**.
#[test]
fn a_sheet_may_be_unhidden() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    session.execute("grid.sheet.hide", &json!({ "sheet": one })).expect("the sheet is hidden");
    session.execute("grid.sheet.unhide", &json!({ "sheet": one })).expect("the sheet is not hidden");
    assert!(!hidden_of(&session, one), "the sheet is not hidden");
}

/// A sheet's **tab colour** may be set.
#[test]
fn a_sheets_tab_colour_may_be_set() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    session
        .execute("grid.sheet.color", &json!({ "sheet": one, "color": "#FF0000" }))
        .expect("the tab is red");
    assert_eq!(colour_of(&session, one).as_str(), Some("#FF0000"), "the tab is red");
}

/// The colour may be taken **away**.
#[test]
fn the_colour_may_be_taken_away() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    session
        .execute("grid.sheet.color", &json!({ "sheet": one, "color": "#FF0000" }))
        .expect("the tab is red");
    session
        .execute("grid.sheet.color", &json!({ "sheet": one, "color": "none" }))
        .expect("the tab has no colour");
    assert_eq!(colour_of(&session, one).as_u64(), None, "the tab has no colour");
}

/// A colour that is **not a colour** is refused.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    let error = session
        .execute("grid.sheet.color", &json!({ "sheet": one, "color": "red" }))
        .expect_err("`red` is not a colour");
    assert!(error.to_string().contains("not a colour"), "the error says so: {error}");
    let error = session
        .execute("grid.sheet.color", &json!({ "sheet": one, "color": "#FF00" }))
        .expect_err("`#FF00` is not a colour");
    assert!(error.to_string().contains("not a colour"), "the error says so: {error}");
}

/// A sheet that is **not there** is refused.
#[test]
fn a_sheet_that_is_not_there_is_refused() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    let error = session
        .execute("grid.sheet.hide", &json!({ "sheet": one + 1000 }))
        .expect_err("there is no such sheet");
    assert!(error.to_string().contains("no sheet"), "the error says so: {error}");
}

/// The hidden sheet and its colour are **saved**.
#[test]
fn the_hidden_sheet_and_its_colour_are_saved() {
    let mut session = session();
    let (_, one, _) = two_sheets(&mut session);
    session.execute("grid.sheet.hide", &json!({ "sheet": one })).expect("the sheet is hidden");
    session
        .execute("grid.sheet.color", &json!({ "sheet": one, "color": "#00FF00" }))
        .expect("the tab is green");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(one).expect("the sheet is there");
    assert_eq!(node.data["hidden"].as_bool(), Some(true), "the sheet is still hidden");
    assert_eq!(node.data["tab_color"].as_str(), Some("#00FF00"), "and the tab is still green");
}
