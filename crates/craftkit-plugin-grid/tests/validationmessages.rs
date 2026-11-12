//! The **messages** of a validation rule.
//!
//! The style of the failure, the title of the error, the input
//! message and the dropdown of a list.

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
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("grid.workbook.add", &json!({ "name": "the workbook" }))
        .expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "the sheet" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **check** of a value against the rules of a cell.
fn check(session: &mut Session, sheet: u64, cell: &str, input: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.validation.check", &json!({ "sheet": sheet, "cell": cell, "input": input }))
        .expect("the value is checked");
    outcome.data.clone()
}

/// A rule with the tree's **messages** carries them, and the check
/// says which style a failure has.
#[test]
fn the_messages_of_a_rule_are_kept() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "whole",
                "operator": "greater",
                "formula1": "10",
                "input_title": "the count",
                "input_message": "a whole number over ten",
                "error_title": "too small",
                "error_message": "the count is over ten",
                "error_style": "warning",
            }),
        )
        .expect("the rule is set");
    let rule = outcome.data["rule"].clone();
    assert_eq!(rule["input_title"], json!("the count"), "the input title is kept");
    assert_eq!(rule["input_message"], json!("a whole number over ten"), "and the message");
    assert_eq!(rule["error_title"], json!("too small"), "the error title is kept");
    assert_eq!(rule["error_message"], json!("the count is over ten"), "and the message");
    assert_eq!(rule["error_style"], json!("warning"), "and the style");
    // The **check** says the style of the failure, and the message of
    // the rule.
    let answer = check(&mut session, sheet, "A1", "3");
    assert_eq!(answer["ok"], json!(false), "the value does not pass: {answer:?}");
    assert_eq!(answer["style"], json!("warning"), "and the style is the rule's");
    assert_eq!(answer["refused"], json!(false), "a warning does not refuse the value");
    assert_eq!(answer["title"], json!("too small"), "the title is the rule's");
    assert_eq!(answer["message"], json!("the count is over ten"), "and the message");
}

/// The **default** style is `stop`, and a `stop` **refuses** the
/// value.
#[test]
fn the_default_style_is_stop() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.validation.set",
            &json!({ "sheet": sheet, "range": "A1:A9", "kind": "whole", "formula1": "10" }),
        )
        .expect("the rule is set");
    let answer = check(&mut session, sheet, "A1", "3");
    assert_eq!(answer["style"], json!("stop"), "the style is the default");
    assert_eq!(answer["refused"], json!(true), "and a stop refuses the value");
    assert_eq!(answer["title"], json!(""), "there is no title");
}

/// An **unknown style** is refused, and so is a message that is too
/// long.
#[test]
fn an_unknown_style_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.validation.set",
            &json!({ "sheet": sheet, "range": "A1:A9", "kind": "whole", "formula1": "10", "error_style": "loud" }),
        )
        .expect_err("the style is refused");
    assert!(error.to_string().contains("unknown error style"), "the refusal says why: {error}");
    let long = "x".repeat(256);
    let error = session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "A1:A9",
                "kind": "whole",
                "formula1": "10",
                "error_message": long,
            }),
        )
        .expect_err("the message is refused");
    assert!(error.to_string().contains("more than 255 characters"), "the refusal says why: {error}");
}

/// The **dropdown** of a list rule is the tree's own: it is on unless
/// it is turned off.
#[test]
fn the_dropdown_of_a_list_rule_is_on() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute(
            "grid.validation.set",
            &json!({ "sheet": sheet, "range": "B1:B9", "kind": "list", "formula1": "\"the beach, the pier\"" }),
        )
        .expect("the rule is set");
    assert_eq!(outcome.data["rule"]["dropdown"], json!(true), "the dropdown is on");
    let outcome = session
        .execute(
            "grid.validation.set",
            &json!({
                "sheet": sheet,
                "range": "C1:C9",
                "kind": "list",
                "formula1": "\"the beach, the pier\"",
                "dropdown": false,
            }),
        )
        .expect("the rule is set");
    assert_eq!(outcome.data["rule"]["dropdown"], json!(false), "and it can be turned off");
}
