//! A function's name may carry the **prefix a file writes**.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// The prefix `_xlfn.` is how a file says *this is a function the
/// format did not have*, and the tree reads the name after it.
#[test]
fn the_prefix_of_a_function_is_read() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    value(&mut session, sheet, "B1", json!(10));
    value(&mut session, sheet, "B2", json!(20));
    value(&mut session, sheet, "B3", json!(30));
    let answer = computed(&mut session, sheet, "D1", "=_xlfn.XLOOKUP(2,A1:A3,B1:B3)");
    assert_eq!(answer, json!(20.0), "the prefixed function is the function");
    let answer = computed(&mut session, sheet, "D2", "=_xlfn.IFS(A1=1,\"one\",TRUE,\"other\")");
    assert_eq!(answer, json!("one"), "and another one as well");
}

/// The prefix `_xlws.` is read the same way.
#[test]
fn the_prefix_of_a_worksheet_function_is_read() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    let answer = computed(&mut session, sheet, "D1", "=_xlws.SUM(A1:A2)");
    assert_eq!(answer, json!(3.0), "the prefixed function is the function");
}

/// The prefix `_xludf.` marks a function of the **user's own**, and it
/// is kept: the tree does not have it.
#[test]
fn the_prefix_of_a_user_defined_function_is_kept() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "D1", "=_xludf.MYTHING(1)");
    assert_eq!(
        answer,
        json!({ "error": "there is no function `_XLUDF.MYTHING`" }),
        "the function of the user is not one of the tree's"
    );
}

/// A formula with a prefix is **kept as it was written**: the file's
/// own text is the cell's text.
#[test]
fn a_formula_with_a_prefix_is_kept_as_it_was_written() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "D1", "formula": "=_xlfn.XLOOKUP(1,A1:A3,B1:B3)" }),
        )
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "D1" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=_xlfn.XLOOKUP(1,A1:A3,B1:B3)"), "the text is as it was");
}
