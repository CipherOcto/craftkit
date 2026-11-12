//! **Finding** a cell.

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

/// The **hits** of a search.
fn find(session: &mut Session, params: Json) -> Json {
    let outcome = session.execute("grid.find", &params).expect("the search is made");
    outcome.data.clone()
}

/// A text is **found** in a sheet.
#[test]
fn a_text_is_found_in_a_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("red"));
    value(&mut session, sheet, "A2", json!("green"));
    let data = find(&mut session, json!({ "sheet": sheet, "what": "red" }));
    assert_eq!(data["count"], json!(1), "one cell holds it");
    assert_eq!(data["hits"][0]["cell"], Json::String("A1".to_string()), "and it is the first");
}

/// The **case** does not matter, unless it is asked for.
#[test]
fn the_case_does_not_matter_unless_it_is_asked_for() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("red"));
    let data = find(&mut session, json!({ "sheet": sheet, "what": "RED" }));
    assert_eq!(data["count"], json!(1), "the case does not matter");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "RED", "match_case": true }));
    assert_eq!(data["count"], json!(0), "and it does when it is asked for");
}

/// The **wildcards** are the tree's.
#[test]
fn the_wildcards_are_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("apple"));
    value(&mut session, sheet, "A2", json!("apricot"));
    value(&mut session, sheet, "A3", json!("pear"));
    let data = find(&mut session, json!({ "sheet": sheet, "what": "ap*" }));
    assert_eq!(data["count"], json!(2), "two begin with ap");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "a???e" }));
    assert_eq!(data["count"], json!(1), "one is a five-letter a-word");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "a*e" }));
    assert_eq!(data["count"], json!(1), "and one begins with a and ends with e");
}

/// The **whole cell** may be asked for.
#[test]
fn the_whole_cell_may_be_asked_for() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("red"));
    value(&mut session, sheet, "A2", json!("reddish"));
    let data = find(&mut session, json!({ "sheet": sheet, "what": "red" }));
    assert_eq!(data["count"], json!(2), "both hold the text");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "red", "whole_cell": true }));
    assert_eq!(data["count"], json!(1), "and one holds nothing else");
    assert_eq!(data["hits"][0]["cell"], Json::String("A1".to_string()), "and it is the first");
}

/// The **formulas** may be searched instead of the values.
#[test]
fn the_formulas_may_be_searched() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    formula(&mut session, sheet, "B1", "=A1*3");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "A1", "formulas": true }));
    assert_eq!(data["count"], json!(1), "the formula holds it");
    assert_eq!(data["hits"][0]["cell"], Json::String("B1".to_string()), "and it is the formula's cell");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "A1" }));
    assert_eq!(data["count"], json!(0), "and the value does not");
}

/// The whole **workbook** may be searched.
#[test]
fn the_whole_workbook_may_be_searched() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    value(&mut session, sheet, "A1", json!("red"));
    value(&mut session, other, "A1", json!("red"));
    let data = find(&mut session, json!({ "what": "red" }));
    assert_eq!(data["count"], json!(2), "both sheets hold it");
    let data = find(&mut session, json!({ "sheet": sheet, "what": "red" }));
    assert_eq!(data["count"], json!(1), "and one sheet holds it");
}

/// What is **not found** is an empty answer, and not an error.
#[test]
fn what_is_not_found_is_an_empty_answer() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("red"));
    let data = find(&mut session, json!({ "sheet": sheet, "what": "purple" }));
    assert_eq!(data["count"], json!(0), "nothing holds it");
    assert_eq!(data["hits"].as_array().map(|hits| hits.len()), Some(0), "and there are no hits");
}

/// Nothing to look for is **refused**.
#[test]
fn nothing_to_look_for_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error =
        session.execute("grid.find", &json!({ "sheet": sheet, "what": "" })).expect_err("it is refused");
    assert!(error.to_string().contains("nothing to look for"), "it says why: {error}");
}
