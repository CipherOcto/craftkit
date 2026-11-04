//! The **sums of squares** of two sets of numbers.

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

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put **numbers** in a row of cells.
fn row(session: &mut Session, sheet: u64, letter: &str, numbers: &[f64]) {
    for (index, number) in numbers.iter().enumerate() {
        let place = format!("{letter}{}", index + 1);
        value(session, sheet, &place, json!(number));
    }
}

/// The **sums** of the squares of two sets.
#[test]
fn the_sums_of_the_squares_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    row(&mut session, sheet, "A", &[2.0, 3.0, 9.0, 1.0, 8.0, 7.0, 5.0]);
    row(&mut session, sheet, "B", &[6.0, 5.0, 11.0, 7.0, 5.0, 4.0, 4.0]);
    assert_eq!(
        computed(&mut session, sheet, "N1", "=SUMX2MY2(A1:A7,B1:B7)"),
        json!(-55.0),
        "the difference of the squares"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=SUMX2PY2(A1:A7,B1:B7)"),
        json!(521.0),
        "the sum of the squares"
    );
    assert_eq!(
        computed(&mut session, sheet, "N3", "=SUMXMY2(A1:A7,B1:B7)"),
        json!(79.0),
        "the sum of the squares of the differences"
    );
    assert_eq!(
        computed(&mut session, sheet, "N4", "=SUMXMY2(A1:A7,B1:B1)"),
        json!({ "error": "#N/A" }),
        "and two sets of different sizes have no pairs"
    );
}

/// A pair with a **text** in it is not a pair.
#[test]
fn a_pair_of_two_numbers_is_a_pair() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    row(&mut session, sheet, "A", &[2.0, 3.0, 9.0]);
    row(&mut session, sheet, "B", &[6.0, 5.0, 11.0]);
    value(&mut session, sheet, "B1", json!("text"));
    assert_eq!(
        computed(&mut session, sheet, "N1", "=SUMX2MY2(A1:A3,B1:B3)"),
        json!(-56.0),
        "the pair of numbers is the pair"
    );
}
