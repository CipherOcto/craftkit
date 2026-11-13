//! The **determinant** of a square of numbers.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **determinant** of a square of numbers.
#[test]
fn the_determinant_of_a_square_of_numbers() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "B1", json!(2));
    value(&mut session, sheet, "A2", json!(3));
    value(&mut session, sheet, "B2", json!(4));
    formula(&mut session, sheet, "D1", "=MDETERM(A1:B2)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(-2.0), "four less six");
    value(&mut session, sheet, "E1", json!(2));
    value(&mut session, sheet, "F1", json!(3));
    value(&mut session, sheet, "G1", json!(1));
    value(&mut session, sheet, "E2", json!(4));
    value(&mut session, sheet, "F2", json!(1));
    value(&mut session, sheet, "G2", json!(2));
    value(&mut session, sheet, "E3", json!(1));
    value(&mut session, sheet, "F3", json!(5));
    value(&mut session, sheet, "G3", json!(3));
    formula(&mut session, sheet, "D2", "=MDETERM(E1:G3)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(-25.0), "and the three by three");
}

/// A square that is **not square**, and a square with a **text** in
/// it, are errors — and a square with no pivot has a determinant of
/// **zero**.
#[test]
fn the_squares_that_are_not_squares_are_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "B1", json!(2));
    value(&mut session, sheet, "C1", json!(3));
    formula(&mut session, sheet, "D1", "=MDETERM(A1:C1)");
    let refused = value_of(&mut session, sheet, "D1");
    assert_eq!(refused["error"], Json::String("#VALUE!".to_string()), "it is an error");
    value(&mut session, sheet, "A2", json!("apple"));
    value(&mut session, sheet, "B2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    value(&mut session, sheet, "B3", json!(4));
    formula(&mut session, sheet, "D2", "=MDETERM(A2:B3)");
    let refused = value_of(&mut session, sheet, "D2");
    assert_eq!(refused["error"], Json::String("#VALUE!".to_string()), "and it is an error");
    value(&mut session, sheet, "A4", json!(1));
    value(&mut session, sheet, "B4", json!(2));
    value(&mut session, sheet, "A5", json!(2));
    value(&mut session, sheet, "B5", json!(4));
    formula(&mut session, sheet, "D3", "=MDETERM(A4:B5)");
    assert_eq!(value_of(&mut session, sheet, "D3"), json!(0.0), "and a square with no pivot is zero");
}
