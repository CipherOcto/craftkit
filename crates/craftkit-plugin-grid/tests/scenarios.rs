//! The **scenarios** of a workbook: the tree's own what-if scenarios.

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
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** of a cell.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// A **scenario** is added to the workbook, and it is **listed**: the
/// tree's own rules.
#[test]
fn a_scenario_is_added_and_listed() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
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
    assert_eq!(outcome.data["name"], json!("the price rises"), "the scenario is named");
    assert_eq!(outcome.data["changing"], json!(1), "and it changes one cell");
    let listed = session
        .execute("grid.scenario.list", &json!({ "workbook": workbook }))
        .expect("the scenarios are listed");
    let scenarios = listed.data["scenarios"].as_array().expect("the scenarios are there");
    assert_eq!(scenarios.len(), 1, "there is one scenario");
    assert_eq!(scenarios[0]["name"], json!("the price rises"), "and it is the one");
    assert_eq!(scenarios[0]["changing"], json!(["A1"]), "the changing cells are there");
    assert_eq!(scenarios[0]["values"], json!([20.0]), "and the values");
    assert_eq!(scenarios[0]["comment"], json!("what if it rises"), "and the comment");
}

/// The **values** of a scenario are the values the cells hold now when
/// the caller says none: the tree's own rule.
#[test]
fn the_values_are_the_values_the_cells_hold_now() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "B1", 30.0);
    session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "as it is",
                "changing": ["A1", "B1"],
            }),
        )
        .expect("the scenario is added");
    let listed = session
        .execute("grid.scenario.list", &json!({ "workbook": workbook }))
        .expect("the scenarios are listed");
    let scenarios = listed.data["scenarios"].as_array().expect("the scenarios are there");
    assert_eq!(scenarios[0]["values"], json!([10.0, 30.0]), "the values are the ones there are");
}

/// **Showing** a scenario puts the values of it in the changing
/// cells, and a **number** stays a number: the tree's own rule.
#[test]
fn showing_a_scenario_sets_the_cells() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "the price rises",
                "changing": ["A1"],
                "values": [20.0],
            }),
        )
        .expect("the scenario is added");
    session
        .execute("grid.scenario.show", &json!({ "workbook": workbook, "name": "the price rises" }))
        .expect("the scenario is shown");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(20.0), "the cell holds the value of it");
}

/// A scenario of the **same name on the same sheet** is refused, and it
/// is **replaced** when the caller says so: the tree's own rule.
#[test]
fn a_scenario_of_the_same_name_is_refused_and_replaced() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    let scenario = json!({
        "workbook": workbook,
        "sheet": sheet,
        "name": "the price rises",
        "changing": ["A1"],
        "values": [20.0],
    });
    session.execute("grid.scenario.add", &scenario).expect("the scenario is added");
    let error = session.execute("grid.scenario.add", &scenario).expect_err("the scenario is refused");
    assert!(error.to_string().contains("already on this sheet"), "the error says why: {error}");
    let mut replaced = scenario.clone();
    replaced["replace"] = json!(true);
    replaced["values"] = json!([30.0]);
    session.execute("grid.scenario.add", &replaced).expect("the scenario is replaced");
    let listed = session
        .execute("grid.scenario.list", &json!({ "workbook": workbook }))
        .expect("the scenarios are listed");
    let scenarios = listed.data["scenarios"].as_array().expect("the scenarios are there");
    assert_eq!(scenarios.len(), 1, "there is one scenario");
    assert_eq!(scenarios[0]["values"], json!([30.0]), "and it is the new one");
}

/// A scenario with **no changing cell** is refused, and so is one with
/// **too many**: the tree's own rule and its own bound.
#[test]
fn a_scenario_with_the_wrong_number_of_cells_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "nothing changes",
                "changing": [],
            }),
        )
        .expect_err("the scenario is refused");
    assert!(error.to_string().contains("no changing cells"), "the error says why: {error}");
    let many: Vec<String> = (0..33).map(|index| format!("A{}", index + 1)).collect();
    let error = session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "too many",
                "changing": many,
            }),
        )
        .expect_err("the scenario is refused");
    assert!(error.to_string().contains("at most 32"), "the error says why: {error}");
}

/// A scenario with **no name** is refused: the tree's own rule.
#[test]
fn a_scenario_with_no_name_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "  ",
                "changing": ["A1"],
            }),
        )
        .expect_err("the scenario is refused");
    assert!(error.to_string().contains("has a name"), "the error says why: {error}");
}

/// A scenario is **taken away**: the tree's own rule.
#[test]
fn a_scenario_is_taken_away() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": "the price rises",
                "changing": ["A1"],
                "values": [20.0],
            }),
        )
        .expect("the scenario is added");
    session
        .execute("grid.scenario.delete", &json!({ "workbook": workbook, "name": "the price rises" }))
        .expect("the scenario is taken away");
    let listed = session
        .execute("grid.scenario.list", &json!({ "workbook": workbook }))
        .expect("the scenarios are listed");
    let scenarios = listed.data["scenarios"].as_array().expect("the scenarios are there");
    assert!(scenarios.is_empty(), "there is no scenario: {scenarios:?}");
}
