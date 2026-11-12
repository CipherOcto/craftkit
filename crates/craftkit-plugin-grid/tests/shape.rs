//! The **shape** of a range: how many rows it has, and how many
//! columns.

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

/// The rows and the columns of a **range**.
#[test]
fn the_shape_of_a_range_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "D1", "=ROWS(A1:B4)"), json!(4.0), "the rows");
    assert_eq!(computed(&mut session, sheet, "D2", "=COLUMNS(A1:B4)"), json!(2.0), "the columns");
    assert_eq!(computed(&mut session, sheet, "D3", "=ROWS(B2:B2)"), json!(1.0), "and a range of one row");
    assert_eq!(computed(&mut session, sheet, "D4", "=COLUMNS(A1:A9)"), json!(1.0), "and of one column");
}

/// An argument that is not a range is **one cell**.
#[test]
fn an_argument_that_is_not_a_range_is_one_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "D1", "=ROWS(5)"), json!(1.0), "the rows of a number");
    assert_eq!(
        computed(&mut session, sheet, "D2", "=COLUMNS(\"text\")"),
        json!(1.0),
        "and the columns of a text"
    );
}
