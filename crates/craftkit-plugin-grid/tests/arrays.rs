//! The **arrays** of the tree's formula language.
//!
//! A formula here is a value and not a table, and so every function
//! of the tree that would give back a table is refused — with a
//! message that says so.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// **Every** function of the tree's array family is refused, and it
/// says why.
#[test]
fn every_array_function_is_refused_and_says_why() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let list = craftkit_plugin_grid::arrays::ARRAY_FUNCTIONS;
    assert!(!list.is_empty(), "the list of the tree's arrays is not empty");
    for (index, name) in list.iter().enumerate() {
        let cell = format!("A{}", index + 1);
        formula(&mut session, sheet, &cell, &format!("={name}(1,2)"));
        let value = value_of(&mut session, sheet, &cell);
        assert_eq!(
            value["error"],
            Json::String("not supported yet: an array".to_string()),
            "{name} is refused and says why: {value}"
        );
    }
}
