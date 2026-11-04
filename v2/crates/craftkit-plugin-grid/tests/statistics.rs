//! The **statistics** of a formula.
//!
//! A formula may ask about the shape of a set of numbers.

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

/// A sheet of **four numbers**.
fn numbers(session: &mut Session, sheet: u64) {
    value(session, sheet, "A1", json!(1));
    value(session, sheet, "A2", json!(2));
    value(session, sheet, "A3", json!(3));
    value(session, sheet, "A4", json!(4));
}

/// The **middle** of the numbers, and the mean of the two middle ones
/// when there are an even number of them.
#[test]
fn the_middle_of_the_numbers_is_the_median() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=MEDIAN(A1:A4)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.5), "the middle of four numbers");
    formula(&mut session, sheet, "C2", "=MEDIAN(A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(2.0), "the middle of three numbers");
}

/// The **most frequent** number is the mode, and a set with no number
/// twice has no mode.
#[test]
fn the_most_frequent_number_is_the_mode() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(2));
    value(&mut session, sheet, "A4", json!(3));
    formula(&mut session, sheet, "C1", "=MODE(A1:A4)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.0), "the two is the mode");
    value(&mut session, sheet, "A5", json!(1));
    value(&mut session, sheet, "A6", json!(2));
    value(&mut session, sheet, "A7", json!(3));
    formula(&mut session, sheet, "C2", "=MODE(A5:A7)");
    assert_eq!(
        value_of(&mut session, sheet, "C2"),
        json!({ "error": "#N/A" }),
        "and a set with no number twice has no mode"
    );
}

/// The **spread** of the numbers: the sample one and the population
/// one.
#[test]
fn the_spread_of_the_numbers_is_the_deviation() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    formula(&mut session, sheet, "C1", "=VAR(A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(1.0), "the sample variance");
    formula(&mut session, sheet, "C2", "=STDEV(A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(1.0), "and the sample deviation");
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "B2", json!(3));
    formula(&mut session, sheet, "C3", "=VARP(B1:B2)");
    assert_eq!(value_of(&mut session, sheet, "C3"), json!(1.0), "the population variance");
    formula(&mut session, sheet, "C4", "=STDEVP(B1:B2)");
    assert_eq!(value_of(&mut session, sheet, "C4"), json!(1.0), "and the population deviation");
}

/// A spread of **fewer than two** numbers is the tree's error.
#[test]
fn a_spread_of_fewer_than_two_numbers_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    formula(&mut session, sheet, "C1", "=VAR(A1:A1)");
    assert_eq!(
        value_of(&mut session, sheet, "C1"),
        json!({ "error": "#DIV/0!" }),
        "the sample variance of one number is an error"
    );
}

/// The k-th **largest** and **smallest**, and a k that is not in the
/// numbers is the tree's error.
#[test]
fn the_kth_largest_and_smallest_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=LARGE(A1:A4,2)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(3.0), "the second largest");
    formula(&mut session, sheet, "C2", "=SMALL(A1:A4,2)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(2.0), "and the second smallest");
    formula(&mut session, sheet, "C3", "=LARGE(A1:A4,9)");
    assert_eq!(
        value_of(&mut session, sheet, "C3"),
        json!({ "error": "#NUM!" }),
        "and a k that is not in the numbers is an error"
    );
}

/// The **place** of a number in a list, counting from the largest
/// unless the order says otherwise.
#[test]
fn the_place_of_a_number_in_a_list_is_its_rank() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=RANK(3,A1:A4)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.0), "the third of four is the second");
    formula(&mut session, sheet, "C2", "=RANK(3,A1:A4,1)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(3.0), "and the third when the order is the other");
    formula(&mut session, sheet, "C3", "=RANK(9,A1:A4)");
    assert_eq!(
        value_of(&mut session, sheet, "C3"),
        json!({ "error": "#N/A" }),
        "and a number that is not in the list has no place"
    );
}

/// The value at a **fraction** of the way through the numbers.
#[test]
fn the_value_at_a_fraction_of_the_numbers_is_the_percentile() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    numbers(&mut session, sheet);
    formula(&mut session, sheet, "C1", "=PERCENTILE(A1:A4,0.5)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.5), "the middle of the numbers");
    formula(&mut session, sheet, "C2", "=QUARTILE(A1:A4,1)");
    assert_eq!(value_of(&mut session, sheet, "C2"), json!(1.75), "and the first quarter");
    formula(&mut session, sheet, "C3", "=PERCENTILE(A1:A4,2)");
    assert_eq!(
        value_of(&mut session, sheet, "C3"),
        json!({ "error": "#NUM!" }),
        "and a fraction that is not a fraction is an error"
    );
}

/// The cells of a range that hold **nothing** are counted.
#[test]
fn the_cells_that_hold_nothing_are_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!("text"));
    formula(&mut session, sheet, "C1", "=COUNTBLANK(A1:A4)");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(2.0), "two cells hold nothing");
}

/// A middle of **no numbers** is the tree's error.
#[test]
fn a_middle_of_no_numbers_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "C1", "=MEDIAN(A1:A3)");
    assert_eq!(
        value_of(&mut session, sheet, "C1"),
        json!({ "error": "#NUM!" }),
        "the middle of no numbers is an error"
    );
}
