//! The **concatenation** of two texts: the `&` of a formula.
//!
//! The tree's `&` puts one text after the other, and it is how a
//! **criterion** is built out of a cell: `">"&A1` is the criterion
//! "greater than the number in `A1`".

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

/// A **workbook** with a **sheet** in it, and their ids.
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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The value of a formula, as it is computed.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    formula(session, sheet, cell, text);
    value_of(session, sheet, cell)
}

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// Two texts are put one after the other.
#[test]
fn two_texts_are_put_one_after_the_other() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=\"a\"&\"b\""), json!("ab"));
    assert_eq!(computed(&mut session, sheet, "A2", "=\"the \"&\"book\""), json!("the book"));
}

/// A number is written as a formula writes a number.
#[test]
fn a_number_is_written_as_it_is_written() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=1&2"), json!("12"));
    assert_eq!(computed(&mut session, sheet, "A2", "=\"n=\"&2.5"), json!("n=2.5"));
    assert_eq!(computed(&mut session, sheet, "A3", "=\"n=\"&-3"), json!("n=-3"));
}

/// A boolean is `TRUE` or `FALSE`, and a cell that holds nothing is
/// nothing at all.
#[test]
fn a_boolean_and_an_empty_cell_are_texts() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=TRUE&\"\""), json!("TRUE"));
    assert_eq!(computed(&mut session, sheet, "A2", "=\"x\"&A9"), json!("x"));
}

/// The concatenation binds **tighter** than a comparison.
#[test]
fn the_comparison_binds_looser_than_the_concatenation() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=\"a\"&\"b\"=\"ab\""), json!(true));
    assert_eq!(computed(&mut session, sheet, "A2", "=\"a\"&\"b\"=\"ba\""), json!(false));
    // And looser than nothing: a sum is a sum before it is joined.
    assert_eq!(computed(&mut session, sheet, "A3", "=1+2&\"x\""), json!("3x"));
}

/// An **error** in either side is the error.
#[test]
fn an_error_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=1/0&\"x\""), json!({ "error": "#DIV/0!" }));
}

/// The criterion of a count may be built out of a cell.
#[test]
fn a_criterion_may_be_built_out_of_a_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for row in 1..=5 {
        value(&mut session, sheet, &format!("A{row}"), json!(row));
    }
    value(&mut session, sheet, "C1", json!(3.0));
    assert_eq!(
        computed(&mut session, sheet, "C2", "=COUNTIFS(A1:A5,\">\"&C1)"),
        json!(2.0),
        "the two numbers above three"
    );
}

/// The **printer**: a copied formula is written out again, and the
/// `&` is in it.
#[test]
fn a_copied_formula_keeps_the_concatenation() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=\"n=\"&A1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "B1", "to": "B1:B3", "mode": "copy" }))
        .expect("the fill is made");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "B3" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=\"n=\" & A3"));
}
