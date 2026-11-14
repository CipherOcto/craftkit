//! The **validation** of a range of cells.
//!
//! A validation rule says what may be typed into a range, and a rule
//! is written for the first cell of its range.

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

/// A **sheet**, and its id.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "the sheet" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// Whether a value **passes** the rules of a cell.
fn passes(session: &mut Session, sheet: u64, cell: &str, input: &str) -> bool {
    let outcome = session
        .execute("grid.validation.check", &json!({ "sheet": sheet, "cell": cell, "input": input }))
        .expect("the value is checked");
    outcome.data["ok"].as_bool().unwrap_or(false)
}

/// A rule of **whole numbers** between one and ten.
fn whole_rule(session: &mut Session, sheet: u64, range: &str) {
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": range,
                "kind": "whole",
                "operator": "between",
                "formula1": "1",
                "formula2": "10",
            }),
        )
        .expect("the rule is set");
}

/// A whole number between one and ten **passes**.
#[test]
fn a_whole_number_between_one_and_ten_passes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    assert!(passes(&mut session, sheet, "A1", "5"), "five passes");
    assert!(passes(&mut session, sheet, "A1", "1"), "one passes");
    assert!(passes(&mut session, sheet, "A1", "10"), "ten passes");
}

/// A whole number **outside** the range fails.
#[test]
fn a_whole_number_outside_the_range_fails() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    assert!(!passes(&mut session, sheet, "A1", "11"), "eleven fails");
    assert!(!passes(&mut session, sheet, "A1", "0"), "zero fails");
}

/// A number with a **fraction** is not a whole number.
#[test]
fn a_number_with_a_fraction_is_not_a_whole_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    assert!(!passes(&mut session, sheet, "A1", "5.5"), "five and a half fails");
}

/// A value that is **not a number** fails a rule of whole numbers.
#[test]
fn a_value_that_is_not_a_number_fails() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    assert!(!passes(&mut session, sheet, "A1", "five"), "a word fails");
}

/// A **blank** passes by default: the tree's default is that a blank
/// is allowed.
#[test]
fn a_blank_passes_by_default() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    assert!(passes(&mut session, sheet, "A1", ""), "a blank passes");
}

/// A blank may be **refused**.
#[test]
fn a_blank_may_be_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "whole",
                "operator": "between",
                "formula1": "1",
                "formula2": "10",
                "ignore_blank": false,
            }),
        )
        .expect("the rule is set");
    assert!(!passes(&mut session, sheet, "A1", ""), "a blank fails");
}

/// A rule of a **list** takes a word from the list.
#[test]
fn a_rule_of_a_list_takes_a_word_from_the_list() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "list",
                "formula1": "\"Open,In progress,Closed\"",
            }),
        )
        .expect("the rule is set");
    assert!(passes(&mut session, sheet, "A1", "Open"), "a word of the list passes");
    assert!(passes(&mut session, sheet, "A1", "Closed"), "and so does another");
    assert!(!passes(&mut session, sheet, "A1", "Unknown"), "and a word that is not of the list fails");
}

/// A list rule may name **another sheet's** cells: the words are read
/// from the sheet the rule names.
#[test]
fn a_list_rule_may_name_another_sheets_cells() {
    let mut session = session();
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "One" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Other" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    session
        .execute("grid.cell.set", &json!({ "sheet": other, "cell": "A1", "value": "ann" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": other, "cell": "A2", "value": "bob" }))
        .expect("the cell is set");
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "list",
                "formula1": "Other!A1:A2",
            }),
        )
        .expect("the rule is set");
    assert!(passes(&mut session, sheet, "A1", "ann"), "a word of the other sheet passes");
    assert!(passes(&mut session, sheet, "A1", "bob"), "and so does another");
    assert!(!passes(&mut session, sheet, "A1", "cat"), "and a word that is not of it fails");
}

/// A list rule with **no list** is refused: a rule that cannot be
/// checked is not a rule.
#[test]
fn a_list_rule_with_no_list_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.validation.set", &json!({ "sheet": sheet, "range": "A1:A9", "kind": "list" }))
        .expect_err("a list rule needs a list");
    assert!(error.to_string().contains("formula1"), "the error says so: {error}");
}

/// A kind that is **not a kind** is refused: the tree's command used
/// to give an "any value" rule for a kind it did not know.
#[test]
fn a_kind_that_is_not_a_kind_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.validation.set", &json!({ "sheet": sheet, "range": "A1", "kind": "wobble" }))
        .expect_err("there is no such kind");
    assert!(error.to_string().contains("unknown type"), "the error says so: {error}");
}

/// An operator that is **not an operator** is refused.
#[test]
fn an_operator_that_is_not_an_operator_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.validation.set",
            &json!({ "sheet": sheet, "range": "A1", "kind": "whole", "operator": "wobble" }),
        )
        .expect_err("there is no such operator");
    assert!(error.to_string().contains("unknown operator"), "the error says so: {error}");
}

/// A rule is **written for the first cell** of its range, and the
/// relative references in it move with the cell: a rule on `A1:A9`
/// whose formula is `A1>10` is checked at `A5` as `A5>10`.
#[test]
fn a_rule_moves_with_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 20 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A5", "value": 100 }))
        .expect("the cell is set");
    session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "whole",
                "operator": "greater",
                "formula1": "A1",
            }),
        )
        .expect("the rule is set");
    // At `A1` the rule reads `A1>10`: the cell holds 20, and 30 > 20
    // is true, so the value passes.
    assert!(passes(&mut session, sheet, "A1", "30"), "at A1 the rule reads A1>10");
    // At `A5` the rule reads `A5>10`: the cell holds 100, and 50 is
    // not more than 100, so the value fails — and it would have
    // passed if the rule still read `A1`.
    assert!(!passes(&mut session, sheet, "A5", "50"), "at A5 the rule reads A5>10");
}

/// The rules of a range may be **cleared**.
#[test]
fn the_rules_of_a_range_may_be_cleared() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    assert!(!passes(&mut session, sheet, "A1", "50"), "fifty fails the rule");
    session
        .execute("grid.validation.clear", &json!({ "sheet": sheet, "range": "A1:A9" }))
        .expect("the rules are cleared");
    assert!(passes(&mut session, sheet, "A1", "50"), "and fifty passes now");
}

/// A range that is **not a range** is refused.
#[test]
fn a_range_that_is_not_a_range_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.validation.set",
            &json!({ "sheet": sheet, "range": "A1:", "kind": "whole", "formula1": "1" }),
        )
        .expect_err("that is not a range");
    assert!(error.to_string().contains("not a range"), "the error says so: {error}");
}

/// The rules are **saved** with the sheet.
#[test]
fn the_rules_are_saved_with_the_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    whole_rule(&mut session, sheet, "A1:A9");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(sheet).expect("the sheet is there");
    let rules = node.data["validations"].as_array().expect("the rules are there");
    assert_eq!(rules.len(), 1, "the rule is saved");
    assert_eq!(rules[0]["kind"].as_str(), Some("whole"), "and it is the rule that was set");
}
