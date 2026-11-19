//! A **number** may be written with an **exponent**.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> craftkit_sdk::serde_json::Value {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// A number with an **exponent** is a number.
#[test]
fn a_number_with_an_exponent_is_a_number() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "A1", "=1E3");
    assert_eq!(answer, json!(1000.0), "the exponent is a thousand");
    let answer = computed(&mut session, sheet, "A2", "=1.5e-3");
    assert_eq!(answer, json!(0.0015), "and a small number is a small number");
    let answer = computed(&mut session, sheet, "A3", "=2E+2");
    assert_eq!(answer, json!(200.0), "and the sign of the exponent is read");
}

/// A letter **after** a number is not an exponent, and the formula
/// that holds it is refused.
#[test]
fn a_letter_after_a_number_is_not_an_exponent() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let refused = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=1E" }))
        .expect_err("the formula is refused");
    assert!(refused.to_string().contains("after the formula"), "the refusal says why: {refused}");
}
