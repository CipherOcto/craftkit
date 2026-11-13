//! The **aggregates that count the text**: the tree's `A` functions.
//!
//! A formula may gather a set of numbers in a way that counts the text
//! as a zero and the booleans as one or zero.

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

/// The **text of a cell is a zero** and a boolean is one or zero.
#[test]
fn the_text_of_a_cell_is_a_zero() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "A2", json!("apple"));
    value(&mut session, sheet, "A3", json!(4));
    value(&mut session, sheet, "A4", json!(true));
    formula(&mut session, sheet, "C1", "=AVERAGEA(A1:A4)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(1.75), "two, zero, four and one");
}

/// The **largest** and the **smallest**, and the smallest of a set
/// with text in it is the zero of the text.
#[test]
fn the_largest_and_the_smallest() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "A2", json!("apple"));
    value(&mut session, sheet, "A3", json!(4));
    formula(&mut session, sheet, "C1", "=MAXA(A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(4.0), "the four is the largest");
    formula(&mut session, sheet, "C2", "=MINA(A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(0.0), "and the text is the smallest");
}

/// The **variance** of the set — the one of a sample and the one of
/// the whole — and the **deviation** that goes with it.
#[test]
fn the_variance_and_the_deviation() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(0));
    value(&mut session, sheet, "A2", json!(2));
    formula(&mut session, sheet, "C1", "=VARPA(A1:A2)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(1.0), "the variance of the whole set");
    formula(&mut session, sheet, "C2", "=VARA(A1:A2)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(2.0), "and the one of the sample");
    formula(&mut session, sheet, "C3", "=STDEVPA(A1:A2)");
    assert_eq!(value_of(&mut session, sheet, "C3"), json!(1.0), "the deviation of the whole set");
    formula(&mut session, sheet, "C4", "=ROUND(STDEVA(A1:A2)^2,10)");
    assert_eq!(
        value_of(&mut session, sheet, "C4"),
        value_of(&mut session, sheet, "C2"),
        "and the deviation of the sample is the square root of its variance"
    );
}

/// A **plain argument** is the value itself: the text of one is the
/// number it writes, and a text that is not a number is an error.
#[test]
fn a_plain_argument_is_the_value_itself() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "C1", "=AVERAGEA(\"5\")");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(5.0), "the text that writes five");
    formula(&mut session, sheet, "C2", "=AVERAGEA(\"apple\")");
    let value = value_of(&mut session, sheet, "C2");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
}

/// An **empty cell** is not counted, and an **error** in the set is
/// the error.
#[test]
fn an_empty_cell_is_not_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(6));
    formula(&mut session, sheet, "C1", "=AVERAGEA(A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(6.0), "the empty cells are not counted");
    formula(&mut session, sheet, "A3", "=1/0");
    formula(&mut session, sheet, "C2", "=AVERAGEA(A1:A3)");
    let value = value_of(&mut session, sheet, "C2");
    assert_eq!(value["error"].as_str(), Some("#DIV/0!"), "it says why: {value}");
}
