//! The **subtotal** of a range names the tree's eleven functions.
//!
//! A subtotal is a function number and a range, and the number says
//! what is computed.

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

/// A **sheet** with three numbers in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    for (cell, value) in [("A1", 2), ("A2", 4), ("A3", 6)] {
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
            .expect("the number is set");
    }
    sheet
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Whether a value is a **number** near the one that is expected.
fn near(value: &Json, expected: f64) -> bool {
    value.as_f64().is_some_and(|number| (number - expected).abs() < 1e-9)
}

/// The subtotal names the tree's **eleven** functions.
#[test]
fn the_subtotal_names_the_trees_eleven_functions() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert!(near(&computed(&mut session, sheet, "C1", "=SUBTOTAL(9,A1:A3)"), 12.0), "the sum is twelve");
    assert!(near(&computed(&mut session, sheet, "C2", "=SUBTOTAL(1,A1:A3)"), 4.0), "the average is four");
    assert!(
        near(&computed(&mut session, sheet, "C3", "=SUBTOTAL(6,A1:A3)"), 48.0),
        "the product is forty-eight"
    );
    assert!(near(&computed(&mut session, sheet, "C4", "=SUBTOTAL(4,A1:A3)"), 6.0), "the largest is six");
    assert!(near(&computed(&mut session, sheet, "C5", "=SUBTOTAL(5,A1:A3)"), 2.0), "the smallest is two");
    assert!(near(&computed(&mut session, sheet, "C6", "=SUBTOTAL(2,A1:A3)"), 3.0), "the count is three");
}

/// The subtotal names the **spread**: the deviation and the variance,
/// of a sample and of a whole.
#[test]
fn the_subtotal_names_the_spread() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert!(
        near(&computed(&mut session, sheet, "C1", "=SUBTOTAL(7,A1:A3)"), 2.0),
        "the sample deviation is two"
    );
    assert!(
        near(&computed(&mut session, sheet, "C2", "=SUBTOTAL(8,A1:A3)"), 8.0_f64.sqrt() / 3.0_f64.sqrt()),
        "the deviation of the whole"
    );
    assert!(
        near(&computed(&mut session, sheet, "C3", "=SUBTOTAL(10,A1:A3)"), 4.0),
        "the variance of the sample is four"
    );
    assert!(
        near(&computed(&mut session, sheet, "C4", "=SUBTOTAL(11,A1:A3)"), 8.0 / 3.0),
        "the variance of the whole"
    );
}

/// The hundred-and-more form leaves the rows that are **hidden by
/// hand** out as well.
#[test]
fn the_hundred_and_more_form_leaves_the_hidden_rows_out() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 3 })).expect("the row is hidden");
    assert!(
        near(&computed(&mut session, sheet, "C1", "=SUBTOTAL(9,A1:A3)"), 12.0),
        "the plain form keeps the hidden row"
    );
    assert!(
        near(&computed(&mut session, sheet, "C2", "=SUBTOTAL(109,A1:A3)"), 6.0),
        "and the hundred-and-more form leaves it out"
    );
}

/// A function number that is **not one of the tree's eleven** is an
/// error that says so.
#[test]
fn a_function_number_that_is_not_one_of_the_trees_is_an_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let value = computed(&mut session, sheet, "C1", "=SUBTOTAL(12,A1:A3)");
    assert!(!value.is_number(), "the number is refused: {value:?}");
}
