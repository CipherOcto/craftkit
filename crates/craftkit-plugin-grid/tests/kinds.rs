//! The **kinds** of the grid battery: what a scenario means.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

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

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// A **scenario** is a kind, and it means the cells it changes and the
/// values they hold.
#[test]
fn a_scenario_means_the_cells_it_changes_and_the_values_they_hold() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "the price rises",
                "changing": ["A1"],
                "values": [20.0],
                "comment": "what if it rises",
            }),
        )
        .expect("the scenario is added");
    let scenario = outcome.data["id"].as_u64().expect("the scenario has an id");
    let node = session.document().node(scenario).expect("the scenario is there");
    assert_eq!(node.kind, "grid.scenario", "the node is a scenario");
    let said = said_of(&session, scenario);
    assert_eq!(said["name"], json!("the price rises"), "the scenario says its name");
    assert_eq!(said["sheet"], json!(sheet), "and the sheet it is on");
    assert_eq!(said["changing"], json!(["A1"]), "and the cells it changes");
    assert_eq!(said["values"], json!([20.0]), "and the values they hold");
}
