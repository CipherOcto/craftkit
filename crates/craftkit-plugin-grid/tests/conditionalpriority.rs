//! The **priority** of a conditional format: the rule that is tried
//! first is the one that writes the cell.
//!
//! The tree's rules: the rules are tried in priority, and the rule
//! that names the lower number is seen first.

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
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **text** a cell is written with.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell has text").to_string()
}

/// The rule with the **lower priority** is the one that writes the
/// cell.
#[test]
fn the_rule_with_the_lower_priority_is_the_one_that_writes_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    a_number(&mut session, sheet, "B1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B3", "op": ">", "value": "1", "format": "0.00", "priority": 2 }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "format": "0.000", "priority": 1 }),
        )
        .expect("the rule is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.000", "the rule of priority one writes the cell");
    assert_eq!(
        text_of(&mut session, sheet, "B1"),
        "5.00",
        "and the rule of priority two writes the cell it covers"
    );
}

/// A rule that names **no priority** is tried after the ones that do.
#[test]
fn a_rule_that_names_no_priority_is_tried_after_the_ones_that_do() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B3", "op": ">", "value": "1", "format": "0.0" }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "format": "0.000", "priority": 1 }),
        )
        .expect("the rule is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.000", "the rule that names a priority is tried first");
}

/// Two rules that name the **same priority** keep the order the sheet
/// holds them in.
#[test]
fn two_rules_with_the_same_priority_keep_the_order_of_the_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B3", "op": ">", "value": "1", "format": "0.00", "priority": 1 }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "format": "0.000", "priority": 1 }),
        )
        .expect("the rule is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.00", "the rule the sheet holds first is tried first");
}

/// The priority of a rule **stays** when another rule is taken away.
#[test]
fn the_priority_of_a_rule_stays_when_another_rule_is_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "D1:D1", "op": ">", "value": "1", "format": "0.00", "priority": 3 }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B1", "op": ">", "value": "1", "format": "0.00" }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A1", "op": ">", "value": "1", "format": "0.000", "priority": 2 }),
        )
        .expect("the rule is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.000", "the rule that names a priority is tried first");
    session
        .execute("grid.conditional.clear", &json!({ "sheet": sheet, "range": "D1:D1" }))
        .expect("the rule is taken away");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.000", "and the priority is still there");
}

/// A priority that is **not a whole number** is refused.
#[test]
fn a_priority_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for priority in [-1.0, 1.5] {
        let outcome = session.execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "format": "0.00", "priority": priority }),
        );
        assert!(outcome.is_err(), "the priority {priority} is refused: {outcome:?}");
    }
}

/// The priority is **saved**: it is in the document after it is done.
#[test]
fn the_priority_is_saved() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "format": "0.00", "priority": 4 }),
        )
        .expect("the rule is set");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    let list = node.data["conditionals"].as_array().expect("the list is there");
    assert_eq!(list[0]["priority"].as_u64(), Some(4), "the priority is there");
}
