//! A formula the tree cannot read says **what it is**.

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

/// The **refusal** of a formula, and what it says.
fn refused(session: &mut Session, sheet: u64, formula: &str) -> String {
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": formula }))
        .expect_err("the formula is refused");
    error.to_string()
}

/// A **spill** is refused, and the refusal says that it is a spill.
#[test]
fn a_spill_is_refused_and_says_that_it_is_a_spill() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let refusal = refused(&mut session, sheet, "=A1#");
    assert!(refusal.contains("spill"), "the refusal says what it is: {refusal}");
    let refusal = refused(&mut session, sheet, "=SUM(A1:A3#)");
    assert!(refusal.contains("spill"), "the refusal says what it is: {refusal}");
}

/// The file's way of writing a spill is refused the same way.
#[test]
fn the_file_s_way_of_writing_a_spill_is_refused() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let refusal = refused(&mut session, sheet, "=_xlfn.ANCHORARRAY(A1)");
    assert!(refusal.contains("spill"), "the refusal says what it is: {refusal}");
}
