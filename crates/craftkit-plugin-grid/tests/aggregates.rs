//! The **aggregates of a condition**.

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

/// A sheet of **three rows**: a name, a number and a second number.
fn rows(session: &mut Session, sheet: u64) {
    value(session, sheet, "A1", json!("apple"));
    value(session, sheet, "A2", json!("banana"));
    value(session, sheet, "A3", json!("apple"));
    value(session, sheet, "B1", json!(10));
    value(session, sheet, "B2", json!(20));
    value(session, sheet, "B3", json!(30));
    value(session, sheet, "C1", json!(1));
    value(session, sheet, "C2", json!(2));
    value(session, sheet, "C3", json!(1));
}

/// The cells that meet **every** condition are counted.
#[test]
fn the_cells_that_meet_every_condition_are_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=COUNTIFS(A1:A3,\"apple\",C1:C3,1)");
    assert_eq!(value_of(&mut session, sheet, "E1"), json!(2.0), "the two apples of the first size");
    formula(&mut session, sheet, "E2", "=COUNTIFS(A1:A3,\"apple\",C1:C3,2)");
    assert_eq!(value_of(&mut session, sheet, "E2"), json!(0.0), "and no apple of the second size");
}

/// The cells that meet every condition are **added**, and they are
/// read from another range.
#[test]
fn the_cells_that_meet_every_condition_are_added() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=SUMIFS(B1:B3,A1:A3,\"apple\",C1:C3,1)");
    assert_eq!(value_of(&mut session, sheet, "E1"), json!(40.0), "the ten and the thirty are added");
}

/// The **average** of the cells that meet every condition.
#[test]
fn the_average_of_the_cells_that_meet_every_condition() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=AVERAGEIFS(B1:B3,A1:A3,\"apple\")");
    assert_eq!(value_of(&mut session, sheet, "E1"), json!(20.0), "the ten and the thirty average twenty");
}

/// The **average** of nothing is a division by zero.
#[test]
fn the_average_of_nothing_is_a_division_by_zero() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=AVERAGEIFS(B1:B3,A1:A3,\"pear\")");
    let value = value_of(&mut session, sheet, "E1");
    assert_eq!(value["error"].as_str(), Some("#DIV/0!"), "it says why: {value}");
}

/// The **largest** and the **smallest** of the cells that meet every
/// condition — and of nothing, they are zero.
#[test]
fn the_largest_and_the_smallest_of_the_cells_that_meet_every_condition() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=MAXIFS(B1:B3,A1:A3,\"apple\")");
    assert_eq!(value_of(&mut session, sheet, "E1"), json!(30.0), "the thirty is the largest");
    formula(&mut session, sheet, "E2", "=MINIFS(B1:B3,A1:A3,\"apple\")");
    assert_eq!(value_of(&mut session, sheet, "E2"), json!(10.0), "the ten is the smallest");
    formula(&mut session, sheet, "E3", "=MAXIFS(B1:B3,A1:A3,\"pear\")");
    assert_eq!(value_of(&mut session, sheet, "E3"), json!(0.0), "nothing is zero");
    formula(&mut session, sheet, "E4", "=MINIFS(B1:B3,A1:A3,\"pear\")");
    assert_eq!(value_of(&mut session, sheet, "E4"), json!(0.0), "and nothing is zero");
}

/// The **average** of the cells that meet **one** condition.
#[test]
fn the_average_of_the_cells_that_meet_one_condition() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=AVERAGEIF(A1:A3,\"apple\",B1:B3)");
    assert_eq!(value_of(&mut session, sheet, "E1"), json!(20.0), "the ten and the thirty average twenty");
}

/// The average range of `AVERAGEIF` is **sized like** the range it is
/// averaged over, and is anchored at its own top left cell — so a
/// range that is smaller is read past its own end.
#[test]
fn the_average_range_is_sized_like_the_range() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=AVERAGEIF(A1:A3,\"apple\",B1:B1)");
    assert_eq!(value_of(&mut session, sheet, "E1"), json!(20.0), "the ten and the thirty average twenty");
}

/// A range of conditions that is **not the shape** of the first is an
/// error.
#[test]
fn a_range_of_conditions_that_is_not_the_shape_of_the_first_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=COUNTIFS(A1:A3,\"apple\",C1:C2,1)");
    let value = value_of(&mut session, sheet, "E1");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
    formula(&mut session, sheet, "E2", "=SUMIFS(B1:B3,A1:A2,\"apple\",C1:C3,1)");
    let value = value_of(&mut session, sheet, "E2");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
}

/// A list of conditions that is **not made of pairs** is an error.
#[test]
fn a_list_of_conditions_that_is_not_made_of_pairs_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    formula(&mut session, sheet, "E1", "=COUNTIFS(A1:A3,\"apple\",C1:C3)");
    let value = value_of(&mut session, sheet, "E1");
    assert_eq!(value["error"].as_str(), Some("#VALUE!"), "it says why: {value}");
}

/// A criterion over the **bound** is refused when there is one
/// condition, and **cut** to its first two hundred and fifty-five
/// characters when there are two.
#[test]
fn a_criterion_over_the_bound_is_refused_and_cut() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    rows(&mut session, sheet);
    let long = "x".repeat(256);
    formula(&mut session, sheet, "E1", &format!("=COUNTIFS(A1:A3,\"{long}\")"));
    let refused = value_of(&mut session, sheet, "E1");
    assert!(
        refused["error"].as_str().is_some_and(|message| message.contains("255")),
        "it says why: {refused}"
    );
    // Two conditions: the criterion is cut, and a cell that holds the
    // cut text meets it.
    value(&mut session, sheet, "D1", json!("x".repeat(255)));
    formula(&mut session, sheet, "E2", &format!("=COUNTIFS(A1:A3,\"{long}\",C1:C3,1)"));
    assert_eq!(value_of(&mut session, sheet, "E2"), json!(0.0), "the cut text is not in the range");
    formula(&mut session, sheet, "E3", &format!("=COUNTIFS(D1:D1,\"{long}\",C1:C1,1)"));
    assert_eq!(value_of(&mut session, sheet, "E3"), json!(1.0), "the cut criterion meets the cell");
}
