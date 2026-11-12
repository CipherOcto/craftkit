//! The **confidence** of a mean.

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

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **confidence** of a mean.
#[test]
fn the_confidence_of_a_mean_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(CONFIDENCE(0.05,2.5,50),9)"),
        json!(0.692951912),
        "the normal confidence"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(CONFIDENCE.NORM(0.05,2.5,50),9)"),
        json!(0.692951912),
        "and by its other name"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(CONFIDENCE.T(0.05,1,50),9)"),
        json!(0.284196855),
        "and the confidence of Student's t"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=CONFIDENCE.T(0.05,1,1)"),
        json!({ "error": "#DIV/0!" }),
        "and one number has no spread to speak of"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=CONFIDENCE(0.05,2.5,0)"),
        json!({ "error": "#NUM!" }),
        "and a size of nothing is an error"
    );
}
