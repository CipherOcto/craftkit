//! The **fill** of a conditional format, and the tree's **stop if
//! true**.
//!
//! The tree's rules: every rule that matches is tried, and where two
//! of them set the same part of a cell the one seen first wins; and a
//! rule that says stop ends the walk when it matched.

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

/// The **cell** as it is read.
fn cell(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// A rule may carry a **fill**: the tree's rule that says "this cell
/// is red".
#[test]
fn a_rule_may_carry_a_fill() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "fill": "#ff0000" }),
        )
        .expect("the rule is set");
    assert_eq!(cell(&mut session, sheet, "A1")["fill"].as_str(), Some("#ff0000"), "the cell is red");
}

/// A rule that carries a fill and **no** format leaves the cell's own
/// format alone.
#[test]
fn a_rule_with_a_fill_and_no_format_leaves_the_format_alone() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "fill": "#ff0000" }),
        )
        .expect("the rule is set");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["text"].as_str(), Some("5"), "the cell keeps its own format");
    assert_eq!(read["fill"].as_str(), Some("#ff0000"), "and it takes the fill");
}

/// The rules **layer**: two rules that match give the cell the format
/// of one and the fill of the other.
#[test]
fn the_rules_layer() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B3", "op": ">", "value": "1", "format": "0.00" }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "fill": "#ff0000" }),
        )
        .expect("the rule is set");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["text"].as_str(), Some("5.00"), "the format is the first rule's");
    assert_eq!(read["fill"].as_str(), Some("#ff0000"), "and the fill is the second rule's");
}

/// The tree's **stop if true**: a rule that matched and says stop ends
/// the walk, and the rules after it are not tried.
#[test]
fn a_rule_that_says_stop_ends_the_walk() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "fill": "#ff0000", "stop": true }),
        )
        .expect("the rule is set");
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:B3", "op": ">", "value": "1", "format": "0.00" }),
        )
        .expect("the rule is set");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["fill"].as_str(), Some("#ff0000"), "the fill is the first rule's");
    assert_eq!(
        read["text"].as_str(),
        Some("5"),
        "and the walk stopped, so the format is not the second rule's"
    );
}

/// A fill that is **not a colour** is refused.
#[test]
fn a_fill_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1", "fill": "red" }),
        )
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("is not a colour"), "the refusal says so: {error}");
}

/// A rule that says **neither** the format nor the fill is refused.
#[test]
fn a_rule_that_says_neither_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A3", "op": ">", "value": "1" }),
        )
        .expect_err("the rule is refused");
    assert!(error.to_string().contains("name what the rule says"), "the refusal says so: {error}");
}
