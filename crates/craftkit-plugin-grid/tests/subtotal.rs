//! The **subtotal** of a range.
//!
//! `SUBTOTAL` is the tree's own function: it sums a range the way a
//! total row does, and it leaves out what a total row leaves out.

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

/// A **workbook** with a **sheet** in it.
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **formula** in a cell.
fn a_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** of a cell, worked out.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// A subtotal sums a range.
#[test]
fn a_subtotal_sums_a_range() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    a_number(&mut session, sheet, "A3", 3.0);
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(9,A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(6.0), "the subtotal is the sum");
}

/// The **hundred-and-more** form leaves the hidden rows out, and the
/// plain form does not: the tree's own rule.
#[test]
fn the_hundred_form_leaves_the_hidden_rows_out() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 2.0);
    a_number(&mut session, sheet, "A3", 3.0);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 2 })).expect("the row is hidden");
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(109,A1:A3)");
    a_formula(&mut session, sheet, "B2", "=SUBTOTAL(9,A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(4.0), "the hidden row is left out");
    assert_eq!(value_of(&mut session, sheet, "B2"), json!(6.0), "and the plain form keeps it");
}

/// A **nested** subtotal is left out, always — in the hundred-and-more
/// form too.
#[test]
fn a_nested_subtotal_is_left_out() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    a_formula(&mut session, sheet, "A2", "=SUBTOTAL(9,A1)");
    a_formula(&mut session, sheet, "C1", "=SUBTOTAL(109,A1:A2)");
    a_formula(&mut session, sheet, "C2", "=SUBTOTAL(9,A1:A2)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.0), "the nested subtotal is left out");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(2.0), "in both forms");
}

/// The other **functions** the tree names work as well.
#[test]
fn the_other_functions_work() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "A2", 20.0);
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(1,A1:A2)");
    a_formula(&mut session, sheet, "B2", "=SUBTOTAL(2,A1:A2)");
    a_formula(&mut session, sheet, "B3", "=SUBTOTAL(4,A1:A2)");
    a_formula(&mut session, sheet, "B4", "=SUBTOTAL(5,A1:A2)");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(15.0), "the average");
    assert_eq!(value_of(&mut session, sheet, "B2"), json!(2.0), "the count");
    assert_eq!(value_of(&mut session, sheet, "B3"), json!(20.0), "the largest");
    assert_eq!(value_of(&mut session, sheet, "B4"), json!(10.0), "and the smallest");
}

/// The **sample** deviation needs two numbers, and one is `#DIV/0!`.
#[test]
fn the_sample_deviation_of_one_number_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(7,A1:A1)");
    let value = value_of(&mut session, sheet, "B1");
    let text = value.to_string();
    assert!(text.contains("#DIV/0!"), "the error says so: {text}");
}

/// A function number that is **not a number at all** is refused.
#[test]
fn a_function_number_out_of_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(42,A1:A1)");
    let value = value_of(&mut session, sheet, "B1");
    let text = value.to_string();
    assert!(text.contains("1 to 11 or 101 to 111"), "the error says so: {text}");
}

/// A subtotal with **no range** is refused.
#[test]
fn a_subtotal_with_no_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(9)");
    let value = value_of(&mut session, sheet, "B1");
    let text = value.to_string();
    assert!(text.contains("a subtotal is a function number and a range"), "the error says so: {text}");
}

/// A subtotal may read **two** ranges.
#[test]
fn a_subtotal_may_read_two_ranges() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_formula(&mut session, sheet, "C1", "=SUBTOTAL(9,A1:A1,B1:B1)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(3.0), "the subtotal is the sum of both");
}

/// A subtotal may read a **name**.
#[test]
fn a_subtotal_may_read_a_name() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 4.0);
    a_number(&mut session, sheet, "A2", 6.0);
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1:A2" }),
        )
        .expect("the name is written");
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(9,Sales)");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(10.0), "the subtotal is the sum");
}

/// The subtotal is **saved**: it is in the document after it is done.
#[test]
fn the_subtotal_is_saved() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "B1", "=SUBTOTAL(9,A1:A1)");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    assert_eq!(
        node.data["cells"]["B1"]["formula"].as_str(),
        Some("=SUBTOTAL(9,A1:A1)"),
        "the formula is there"
    );
}
