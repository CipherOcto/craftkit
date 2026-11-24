//! The **evaluators** of the grid battery: what the whole document means.
//!
//! An evaluator is a gesture about the **whole document**: it may look at
//! every node and say something about all of them. It is a rule the kit
//! publishes as much as a command or a kind is, and an evaluator that is
//! only proved in another battery's tests is a rule that does not travel
//! with the thing it is about.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// The **calc** evaluator is there, and it is the grid layer's.
#[test]
fn the_calc_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("grid.calc").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Calc", "the evaluator is called Calc");
    assert_eq!(evaluator.layer, "grid", "and the grid layer owns it");
}

/// The evaluator gives the **values** of the cells that have a formula in
/// them, computed from the book.
#[test]
fn the_calc_evaluator_gives_the_values_of_the_formulas() {
    let mut session = session();
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=1+1" }))
        .expect("the formula is set");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "grid.calc").expect("the plan has the values");
    assert_eq!(item.node, sheet, "the item is about the sheet");
    assert_eq!(item.value["cells"]["A1"], json!(2.0), "one and one is two: {item:?}");
}
