//! The **odds and ends**: the sum of the products, the power series,
//! the probability of a range and the fraction of a total.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The sum of the products of the sets is taken one place at a time.
#[test]
fn the_sum_of_the_products_is_taken_one_place_at_a_time() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    value(&mut session, sheet, "B1", json!(10));
    value(&mut session, sheet, "B2", json!(20));
    value(&mut session, sheet, "B3", json!(30));
    formula(&mut session, sheet, "D1", "=SUMPRODUCT(A1:A3,B1:B3)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(140.0), "1·10 + 2·20 + 3·30");
}

/// Sets that are **not of the same size** are a value error.
#[test]
fn sets_that_are_not_of_the_same_size_are_a_value_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "B1", json!(10));
    formula(&mut session, sheet, "D1", "=SUMPRODUCT(A1:A2,B1:B1)");
    assert_eq!(
        value_of(&mut session, sheet, "D1")["error"],
        Json::String("#VALUE!".to_string()),
        "not the same size"
    );
}

/// A **text** in a set of cells counts as nothing, and a text that is
/// an argument on its own is an error.
#[test]
fn a_text_counts_as_nothing_in_a_set_and_is_an_error_alone() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!("two"));
    value(&mut session, sheet, "B1", json!(10));
    value(&mut session, sheet, "B2", json!(20));
    formula(&mut session, sheet, "D1", "=SUMPRODUCT(A1:A2,B1:B2)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(10.0), "the text counts as nothing");
    formula(&mut session, sheet, "D2", "=SUMPRODUCT(\"x\",B1:B2)");
    assert_eq!(
        value_of(&mut session, sheet, "D2")["error"],
        Json::String("#VALUE!".to_string()),
        "a text alone"
    );
}

/// The sum of a **power series** is the tree's.
#[test]
fn the_sum_of_a_power_series_is_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1.0));
    value(&mut session, sheet, "A2", json!(2.0));
    value(&mut session, sheet, "A3", json!(3.0));
    // 1·x⁰ + 2·x¹ + 3·x² with x = 2: 1 + 4 + 12.
    formula(&mut session, sheet, "D1", "=SERIESSUM(2,0,1,A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(17.0), "the series");
}

/// The probability of a **range** of values.
#[test]
fn the_probability_of_a_range_of_values() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    value(&mut session, sheet, "B1", json!(0.2));
    value(&mut session, sheet, "B2", json!(0.3));
    value(&mut session, sheet, "B3", json!(0.5));
    formula(&mut session, sheet, "D1", "=PROB(A1:A3,B1:B3,2,3)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(0.8), "from two to three");
    formula(&mut session, sheet, "D2", "=PROB(A1:A3,B1:B3,2)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(0.3), "the second alone");
}

/// Probabilities that do not add up to **one** are a number error.
#[test]
fn probabilities_that_do_not_add_up_to_one_are_a_number_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "B1", json!(0.2));
    value(&mut session, sheet, "B2", json!(0.3));
    formula(&mut session, sheet, "D1", "=PROB(A1:A2,B1:B2)");
    assert_eq!(value_of(&mut session, sheet, "D1")["error"], Json::String("#NUM!".to_string()), "not one");
}

/// The sum of a subset as a fraction of the sum of all the values.
#[test]
fn the_sum_of_a_subset_as_a_fraction_of_the_total() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(10));
    value(&mut session, sheet, "A2", json!(30));
    formula(&mut session, sheet, "D1", "=PERCENTOF(A1,A1:A2)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(0.25), "a quarter");
    formula(&mut session, sheet, "D2", "=PERCENTOF(A1,A1)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(1.0), "all of it");
}

/// A total of **nothing** is a division by nothing.
#[test]
fn a_total_of_nothing_is_a_division_by_nothing() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(0));
    formula(&mut session, sheet, "D1", "=PERCENTOF(A1,A1)");
    assert_eq!(value_of(&mut session, sheet, "D1")["error"], Json::String("#DIV/0!".to_string()), "no total");
}
