//! The **outline symbols** and the **formulas** of a sheet's view: the
//! tree's own flags of a sheet.

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

/// A **sheet**, and its id.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "the sheet" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **view** of a sheet, as the document holds it.
fn view_of(session: &Session, sheet: u64) -> Value {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data.get("view").cloned().unwrap_or(Value::Null)
}

/// A new sheet has the tree's **own** flags: the outline symbols are
/// drawn and the formulas are not shown.
#[test]
fn a_new_sheet_has_the_trees_own_flags() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let view = view_of(&session, sheet);
    assert_eq!(view["outline_symbols"].as_bool(), Some(true), "the outline symbols are drawn");
    assert_eq!(view["show_formulas"].as_bool(), Some(false), "the formulas are not shown");
}

/// The **outline symbols** may be taken away: the tree's own flag.
#[test]
fn the_outline_symbols_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "outline_symbols": false }))
        .expect("the view is set");
    assert_eq!(view_of(&session, sheet)["outline_symbols"].as_bool(), Some(false), "the symbols are gone");
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "outline_symbols": true }))
        .expect("the view is set");
    assert_eq!(view_of(&session, sheet)["outline_symbols"].as_bool(), Some(true), "and they are back");
}

/// The **formulas** may be shown: the tree's own flag.
#[test]
fn the_formulas_may_be_shown() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "show_formulas": true }))
        .expect("the view is set");
    assert_eq!(view_of(&session, sheet)["show_formulas"].as_bool(), Some(true), "the formulas are shown");
}

/// A **flag that is not named** keeps its value: the tree's own rule.
#[test]
fn a_flag_that_is_not_named_keeps_its_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "show_formulas": true }))
        .expect("the view is set");
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "gridlines": false }))
        .expect("the view is set");
    let view = view_of(&session, sheet);
    assert_eq!(view["show_formulas"].as_bool(), Some(true), "the formulas are still shown");
    assert_eq!(view["gridlines"].as_bool(), Some(false), "and the gridlines are still gone");
}

/// The **answer** of the command says what the view is.
#[test]
fn the_answer_says_what_the_view_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "show_formulas": true }))
        .expect("the view is set");
    assert_eq!(outcome.data["view"]["show_formulas"].as_bool(), Some(true), "the answer says so");
    assert_eq!(outcome.data["view"]["outline_symbols"].as_bool(), Some(true), "and the rest of it");
}
