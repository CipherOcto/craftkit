//! The **replacing** of text in a sheet.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// The **values** of a range, as the answer holds them.
fn values_of(session: &mut Session, sheet: u64, range: &str) -> Vec<Json> {
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": range }))
        .expect("the range is read");
    outcome.data["values"].as_array().expect("the values are a list").clone()
}

/// The text that is looked for is **replaced**.
#[test]
fn the_text_is_replaced() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one two one"));
    let outcome = session
        .execute("grid.replace", &json!({ "sheet": sheet, "what": "one", "with": "three" }))
        .expect("the text is replaced");
    assert_eq!(outcome.data["count"], json!(1), "one cell is changed");
    let values = values_of(&mut session, sheet, "A1:A1");
    assert_eq!(values[0][0], json!("three two three"), "and every occurrence is replaced");
}

/// The **case** does not matter unless the search says it does.
#[test]
fn the_case_does_not_matter_unless_the_search_says_it_does() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("One one"));
    session
        .execute("grid.replace", &json!({ "sheet": sheet, "what": "one", "with": "three" }))
        .expect("the text is replaced");
    let values = values_of(&mut session, sheet, "A1:A1");
    assert_eq!(values[0][0], json!("three three"), "the words are found whatever their case");
}

/// A cell whose text **is a number** holds a number afterwards.
#[test]
fn a_cell_that_holds_a_number_holds_a_number_afterwards() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(12));
    session
        .execute("grid.replace", &json!({ "sheet": sheet, "what": "2", "with": "3" }))
        .expect("the text is replaced");
    let values = values_of(&mut session, sheet, "A1:A1");
    assert_eq!(values[0][0], json!(13.0), "the number is still a number");
}

/// The **whole cell** may have to match.
#[test]
fn the_whole_cell_may_have_to_match() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one two"));
    value(&mut session, sheet, "A2", json!("one"));
    let outcome = session
        .execute(
            "grid.replace",
            &json!({ "sheet": sheet, "what": "one", "with": "three", "whole_cell": true }),
        )
        .expect("the text is replaced");
    assert_eq!(outcome.data["count"], json!(1), "only the cell that is one whole is changed");
    let values = values_of(&mut session, sheet, "A1:A2");
    assert_eq!(values[0][0], json!("one two"), "the other cell is left alone");
    assert_eq!(values[1][0], json!("three"), "and the cell is changed");
}

/// The **formulas** may be looked in.
#[test]
fn the_formulas_may_be_looked_in() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    formula(&mut session, sheet, "A2", "=A1+1");
    let outcome = session
        .execute("grid.replace", &json!({ "sheet": sheet, "what": "A1", "with": "A2", "formulas": true }))
        .expect("the text is replaced");
    assert_eq!(outcome.data["count"], json!(1), "the formula is changed");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A2" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=A2+1"), "and it is the formula that was written");
}

/// A formula that does not parse after the replacing is **refused**.
#[test]
fn a_formula_that_does_not_parse_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=1+1");
    let outcome = session
        .execute("grid.replace", &json!({ "sheet": sheet, "what": "1+1", "with": "1+", "formulas": true }));
    assert!(outcome.is_err(), "the formula would not be a formula");
}

/// A **wildcard** is not something that can be replaced.
#[test]
fn a_wildcard_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    let outcome = session.execute("grid.replace", &json!({ "sheet": sheet, "what": "o*", "with": "x" }));
    assert!(outcome.is_err(), "a wildcard cannot be replaced");
}
