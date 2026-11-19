//! A **number** may be written with a **percent sign**.

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

/// A percent is a **hundredth** of the number it follows.
#[test]
fn a_percent_is_a_hundredth_of_the_number() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "A1", "=50%");
    assert_eq!(answer, json!(0.5), "fifty percent is a half");
    let answer = computed(&mut session, sheet, "A2", "=200%");
    assert_eq!(answer, json!(2.0), "and two hundred percent is two");
    let answer = computed(&mut session, sheet, "A3", "=0%");
    assert_eq!(answer, json!(0.0), "and nothing percent is nothing");
}

/// The percent follows a **cell** and a **call** as well.
#[test]
fn the_percent_follows_a_cell_and_a_call() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B1", json!(50));
    let answer = computed(&mut session, sheet, "A1", "=B1%");
    assert_eq!(answer, json!(0.5), "the cell in a hundredth of itself");
    let answer = computed(&mut session, sheet, "A2", "=SUM(B1:B1)%");
    assert_eq!(answer, json!(0.5), "and the call as well");
}

/// The percent binds tighter than the **power**: `2^50%` is two to the
/// half.
#[test]
fn the_percent_binds_tighter_than_the_power() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "A1", "=2^50%");
    assert_eq!(answer, json!(2.0_f64.powf(0.5)), "two to the half is not half of two to the fiftieth");
}

/// A percent of a **text** is `#VALUE!`: the tree's rule of a sign is
/// the rule of the negation.
#[test]
fn a_percent_of_a_text_is_a_value_error() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B1", json!("fifty"));
    let answer = computed(&mut session, sheet, "A1", "=B1%");
    assert_eq!(answer, json!({ "error": "#VALUE!" }), "a text is not a number");
}

/// A percent is written as it was written.
#[test]
fn a_percent_is_written_as_it_was_written() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=50%" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=50%"), "the percent is as it was written");
}

/// A percent **moves** with the cell it is in: the cell it holds is
/// moved, and the percent stays.
#[test]
fn a_percent_moves_with_the_cell_it_holds() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=B1%" }))
        .expect("the formula is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the formula is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "A2" }))
        .expect("the formula is pasted");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A2" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=B2%"), "the cell moves and the percent stays");
}
