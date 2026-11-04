//! The **whole numbers** of a number: the next even, the next odd and
//! the root of pi.

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

/// The **next even** whole number.
#[test]
fn the_next_even_whole_number_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=EVEN(1.5)"), json!(2.0), "a half");
    assert_eq!(computed(&mut session, sheet, "A2", "=EVEN(3)"), json!(4.0), "an odd whole");
    assert_eq!(computed(&mut session, sheet, "A3", "=EVEN(2)"), json!(2.0), "an even whole");
    assert_eq!(computed(&mut session, sheet, "A4", "=EVEN(-1)"), json!(-2.0), "and away from nothing");
    assert_eq!(computed(&mut session, sheet, "A5", "=EVEN(0)"), json!(0.0), "and nothing is nothing");
}

/// The **next odd** whole number.
#[test]
fn the_next_odd_whole_number_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ODD(1.5)"), json!(3.0), "a half");
    assert_eq!(computed(&mut session, sheet, "A2", "=ODD(3)"), json!(3.0), "an odd whole");
    assert_eq!(computed(&mut session, sheet, "A3", "=ODD(2)"), json!(3.0), "an even whole");
    assert_eq!(computed(&mut session, sheet, "A4", "=ODD(-1)"), json!(-1.0), "and away from nothing");
    assert_eq!(computed(&mut session, sheet, "A5", "=ODD(-2)"), json!(-3.0), "and the other way");
    assert_eq!(computed(&mut session, sheet, "A6", "=ODD(0)"), json!(1.0), "and nothing is one");
}

/// The **root of pi**.
#[test]
fn the_root_of_pi_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(SQRTPI(1),12)"),
        json!(1.772453850906),
        "the root of pi"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=SQRTPI(-1)"),
        json!({ "error": "#NUM!" }),
        "and a negative number has no root"
    );
}
