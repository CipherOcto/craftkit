//! The **combinations** of things.
//!
//! A formula may ask how many ways there are to choose.

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
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell and read what it computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    value_of(session, sheet, cell)
}

/// The **factorial** of a number.
#[test]
fn the_factorial_of_a_number_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=FACT(5)"), json!(120.0), "the factorial");
    assert_eq!(computed(&mut session, sheet, "A2", "=FACT(1.9)"), json!(1.0), "and of a fraction");
    assert_eq!(computed(&mut session, sheet, "A3", "=FACT(0)"), json!(1.0), "and of nothing");
}

/// The **double factorial**.
#[test]
fn the_double_factorial_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=FACTDOUBLE(6)"), json!(48.0), "of an even number");
    assert_eq!(computed(&mut session, sheet, "A2", "=FACTDOUBLE(7)"), json!(105.0), "and of an odd one");
    assert_eq!(computed(&mut session, sheet, "A3", "=FACTDOUBLE(-1)"), json!(1.0), "and of minus one");
}

/// The **combinations**, with and without repetition.
#[test]
fn the_combinations_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=COMBIN(8,2)"), json!(28.0), "without repetition");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=COMBIN(8.9,2.9)"),
        json!(28.0),
        "and of numbers that are not whole"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=COMBIN(60,30)"),
        json!(118264581564861424.0),
        "and of a great many"
    );
    assert_eq!(computed(&mut session, sheet, "A4", "=COMBINA(4,3)"), json!(20.0), "with repetition");
    assert_eq!(computed(&mut session, sheet, "A5", "=COMBINA(10,3)"), json!(220.0), "and more of it");
}

/// The **permutations**, with and without repetition.
#[test]
fn the_permutations_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=PERMUT(100,3)"), json!(970200.0), "without repetition");
    assert_eq!(computed(&mut session, sheet, "A2", "=PERMUT(3,2)"), json!(6.0), "and of a few");
    assert_eq!(computed(&mut session, sheet, "A3", "=PERMUTATIONA(3,2)"), json!(9.0), "with repetition");
}

/// The **multinomial**: the ways of dividing things between people.
#[test]
fn the_multinomial_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=MULTINOMIAL(2,3,4)"),
        json!(1260.0),
        "the ways of dividing them"
    );
}

/// The tree's **errors**: a negative number, a factorial beyond a
/// double and a choice of more than there is.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=FACT(-1)"),
        json!({ "error": "#NUM!" }),
        "a negative number is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=FACT(171)"),
        json!({ "error": "#NUM!" }),
        "and a factorial beyond a double is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=COMBIN(2,3)"),
        json!({ "error": "#NUM!" }),
        "and a choice of more than there is is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=PERMUT(2,3)"),
        json!({ "error": "#NUM!" }),
        "and so is an order of more than there is"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=FACTDOUBLE(-2)"),
        json!({ "error": "#NUM!" }),
        "and a double factorial below minus one is an error"
    );
}
