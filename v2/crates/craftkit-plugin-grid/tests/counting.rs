//! The **counting** of a condition.

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

/// A sheet of **three numbers** and two texts.
fn numbers(session: &mut Session, sheet: u64) {
    value(session, sheet, "A1", json!(1));
    value(session, sheet, "A2", json!(2));
    value(session, sheet, "A3", json!(3));
}

/// The cells that meet a **number** condition are counted.
#[test]
fn the_cells_that_meet_a_number_condition_are_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=COUNTIF(A1:A3,\">2\")");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(1.0), "one cell is greater than two");
}

/// The cells that meet a **text** condition are counted, and the text
/// is compared without case.
#[test]
fn the_cells_that_meet_a_text_condition_are_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("apple"));
    value(&mut session, sheet, "A2", json!("banana"));
    formula(&mut session, sheet, "C1", "=COUNTIF(A1:A2,\"apple\")");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(1.0), "one cell is apple");
    formula(&mut session, sheet, "C2", "=COUNTIF(A1:A2,\"APPLE\")");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(1.0), "and the case is not the case");
}

/// The **wildcards** are the tree's: a run, one character, and the
/// escape.
#[test]
fn the_wildcards_are_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("apple"));
    value(&mut session, sheet, "A2", json!("apricot"));
    value(&mut session, sheet, "A3", json!("pear"));
    formula(&mut session, sheet, "C1", "=COUNTIF(A1:A3,\"ap*\")");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.0), "two begin with ap");
    formula(&mut session, sheet, "C2", "=COUNTIF(A1:A3,\"a???e\")");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(1.0), "one is a four-letter a-word");
}

/// A number written as **text** matches only for equality.
#[test]
fn a_number_written_as_text_matches_only_for_equality() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("1"));
    formula(&mut session, sheet, "C1", "=COUNTIF(A1:A1,1)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(1.0), "the text one is one");
    formula(&mut session, sheet, "C2", "=COUNTIF(A1:A1,\">0\")");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(0.0), "and it is not greater than zero");
}

/// An **empty cell** is not counted by a number condition, and the
/// **bare empty** condition counts it.
#[test]
fn an_empty_cell_is_not_counted_by_a_number_condition() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=COUNTIF(A1:A10,\">0\")");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(3.0), "the empty cells are not counted");
    formula(&mut session, sheet, "C2", "=COUNTIF(A1:A10,\"\")");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(7.0), "and the bare empty condition counts them");
}

/// The cells that meet the condition are **added**.
#[test]
fn the_cells_that_meet_the_condition_are_added() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=SUMIF(A1:A3,\">1\")");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(5.0), "two and three are added");
}

/// The sum may read **another range**: the cells that stand opposite.
#[test]
fn the_sum_may_read_another_range() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("x"));
    value(&mut session, sheet, "A2", json!("y"));
    value(&mut session, sheet, "B1", json!(10));
    value(&mut session, sheet, "B2", json!(20));
    formula(&mut session, sheet, "C1", "=SUMIF(A1:A2,\"x\",B1:B2)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(10.0), "the ten is added");
}

/// The sum range is **sized like** the range, and is anchored at its
/// own top left cell.
#[test]
fn the_sum_range_is_sized_like_the_range() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    value(&mut session, sheet, "B1", json!(100));
    value(&mut session, sheet, "B2", json!(200));
    value(&mut session, sheet, "B3", json!(300));
    formula(&mut session, sheet, "C1", "=SUMIF(A1:A3,\">1\",B1:B1)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(500.0), "the range is sized like the first");
}

/// A criterion over the **bound** is an error.
#[test]
fn a_criterion_over_the_bound_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    let long = "x".repeat(256);
    formula(&mut session, sheet, "C1", &format!("=COUNTIF(A1:A3,\"{long}\")"));
    let value = value_of(&mut session, sheet, "C1");
    assert!(value["error"].as_str().is_some_and(|message| message.contains("255")), "it says why: {value}");
}

/// A counting function over a range that is **not there** is an
/// error.
#[test]
fn a_counting_function_over_a_range_that_is_not_there_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "C1", "=COUNTIF(Other!A1:A3,\">1\")");
    let value = value_of(&mut session, sheet, "C1");
    assert_eq!(value["error"].as_str(), Some("there is no sheet `Other`"), "it says why: {value}");
}
