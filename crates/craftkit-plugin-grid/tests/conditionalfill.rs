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

/// A **colour scale** fills the cells of a range: the fill of a cell is
/// the mix of the two colours, in the proportion its number is between
/// the lowest and the highest of the range.
#[test]
fn a_colour_scale_fills_the_cells_of_a_range() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 10.0);
    session
        .execute(
            "grid.conditional.colour.scale",
            &json!({
                "sheet": sheet,
                "range": "A1:A3",
                "colours": ["#000000", "#ffffff"],
            }),
        )
        .expect("the scale is set");
    assert_eq!(
        cell(&mut session, sheet, "A1")["fill"].as_str(),
        Some("#000000"),
        "the lowest is the low colour"
    );
    assert_eq!(
        cell(&mut session, sheet, "A3")["fill"].as_str(),
        Some("#ffffff"),
        "the highest is the high colour"
    );
    assert_eq!(cell(&mut session, sheet, "A2")["fill"].as_str(), Some("#808080"), "the middle is the mix");
}

/// A range whose numbers are all the same has no **span**, and it is
/// filled with nothing.
#[test]
fn a_range_without_a_span_is_filled_with_nothing() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    a_number(&mut session, sheet, "A2", 5.0);
    session
        .execute(
            "grid.conditional.colour.scale",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "colours": ["#000000", "#ffffff"],
            }),
        )
        .expect("the scale is set");
    assert!(cell(&mut session, sheet, "A1")["fill"].is_null(), "the cell is not filled");
}

/// A scale has **two** colours, and the tree refuses any other number
/// of them.
#[test]
fn a_scale_has_two_colours() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let failure = session
        .execute(
            "grid.conditional.colour.scale",
            &json!({ "sheet": sheet, "range": "A1:A2", "colours": ["#000000"] }),
        )
        .expect_err("the scale is refused");
    let message = failure.to_string();
    assert!(message.contains("the colours of a colour scale are two"), "the refusal says why: {message}");
    let failure = session
        .execute(
            "grid.conditional.colour.scale",
            &json!({ "sheet": sheet, "range": "A1:A2", "colours": ["#000000", "purple"] }),
        )
        .expect_err("the scale is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a colour"), "the refusal says why: {message}");
}

/// A scale may have **three** colours, and the middle one stands for the
/// middle of the numbers.
#[test]
fn a_scale_may_have_three_colours() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 10.0);
    session
        .execute(
            "grid.conditional.colour.scale",
            &json!({
                "sheet": sheet,
                "range": "A1:A3",
                "colours": ["#000000", "#ff0000", "#ffffff"],
            }),
        )
        .expect("the scale is set");
    assert_eq!(
        cell(&mut session, sheet, "A1")["fill"].as_str(),
        Some("#000000"),
        "the lowest is the low colour"
    );
    assert_eq!(
        cell(&mut session, sheet, "A2")["fill"].as_str(),
        Some("#ff0000"),
        "the middle is the middle colour"
    );
    assert_eq!(
        cell(&mut session, sheet, "A3")["fill"].as_str(),
        Some("#ffffff"),
        "the highest is the high colour"
    );
}

/// A scale with **four** colours is refused.
#[test]
fn a_scale_with_four_colours_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let failure = session
        .execute(
            "grid.conditional.colour.scale",
            &json!({
                "sheet": sheet,
                "range": "A1:A2",
                "colours": ["#000000", "#111111", "#222222", "#333333"],
            }),
        )
        .expect_err("the scale is refused");
    let message = failure.to_string();
    assert!(message.contains("two or three"), "the refusal says why: {message}");
}

/// A **data bar** is drawn behind the cells of a range: the bar is as
/// long as the cell's number is between the lowest and the highest of
/// the range.
#[test]
fn a_data_bar_is_drawn_behind_the_cells_of_a_range() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 10.0);
    session
        .execute(
            "grid.conditional.data.bar",
            &json!({ "sheet": sheet, "range": "A1:A3", "colour": "#638ec6" }),
        )
        .expect("the bar is set");
    let lowest = cell(&mut session, sheet, "A1");
    assert_eq!(lowest["bar"]["part"].as_f64(), Some(0.0), "the lowest has no bar");
    assert_eq!(lowest["bar"]["colour"].as_str(), Some("#638ec6"), "and the bar has the colour");
    let middle = cell(&mut session, sheet, "A2");
    assert_eq!(middle["bar"]["part"].as_f64(), Some(0.5), "the middle is half the cell");
    let highest = cell(&mut session, sheet, "A3");
    assert_eq!(highest["bar"]["part"].as_f64(), Some(1.0), "the highest fills the cell");
}

/// A cell that does not hold a number has no bar.
#[test]
fn a_cell_that_does_not_hold_a_number_has_no_bar() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 9.0);
    session
        .execute(
            "grid.conditional.data.bar",
            &json!({ "sheet": sheet, "range": "A1:A3", "colour": "#638ec6" }),
        )
        .expect("the bar is set");
    assert!(cell(&mut session, sheet, "A3")["bar"].is_null(), "the cell has no bar");
}

/// A bar has a **colour**, and the tree refuses one that is not a
/// colour.
#[test]
fn a_bar_has_a_colour() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let failure = session
        .execute("grid.conditional.data.bar", &json!({ "sheet": sheet, "range": "A1:A2", "colour": "blue" }))
        .expect_err("the bar is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a colour"), "the refusal says why: {message}");
}

/// An **icon set** puts an icon in the cells of a range: the high one,
/// the middle one and the low one, and the percentiles of the numbers
/// of the range say which is which.
#[test]
fn an_icon_set_puts_an_icon_in_the_cells_of_a_range() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    a_number(&mut session, sheet, "A3", 9.0);
    session
        .execute("grid.conditional.icon.set", &json!({ "sheet": sheet, "range": "A1:A3" }))
        .expect("the icons are set");
    // The **percentiles**: the 67th and the 33rd of the numbers
    // 1, 5 and 9 are 6.33 and 3.67.
    assert_eq!(cell(&mut session, sheet, "A1")["icon"].as_u64(), Some(0), "the lowest has the low icon");
    assert_eq!(cell(&mut session, sheet, "A2")["icon"].as_u64(), Some(1), "the middle has the middle icon");
    assert_eq!(cell(&mut session, sheet, "A3")["icon"].as_u64(), Some(2), "the highest has the high icon");
}

/// The **percents** of an icon set are two of them, and the high one is
/// above the low one.
#[test]
fn the_percents_of_an_icon_set_are_two() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    let failure = session
        .execute(
            "grid.conditional.icon.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "high": 20.0, "low": 40.0 }),
        )
        .expect_err("the icons are refused");
    let message = failure.to_string();
    assert!(message.contains("above the low one"), "the refusal says why: {message}");
    let failure = session
        .execute("grid.conditional.icon.set", &json!({ "sheet": sheet, "range": "A1:A2", "high": 120.0 }))
        .expect_err("the icons are refused");
    let message = failure.to_string();
    assert!(message.contains("a hundred at most"), "the refusal says why: {message}");
}

/// A rule may carry the **colour** of the text: the tree's rule that
/// says "this text is red".
#[test]
fn a_rule_may_carry_the_colour_of_the_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    a_number(&mut session, sheet, "A2", 1.0);
    session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "3", "colour": "#9C0006" }),
        )
        .expect("the rule is set");
    assert_eq!(cell(&mut session, sheet, "A1")["colour"].as_str(), Some("#9c0006"), "the text is red");
    assert!(cell(&mut session, sheet, "A2")["colour"].is_null(), "and the other cell is not");
}

/// The colour of a rule is a **colour**, and `none` is not one.
#[test]
fn the_colour_of_a_rule_is_a_colour() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "3", "colour": "red" }),
        )
        .expect_err("the colour is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a colour"), "the refusal says why: {message}");
    let failure = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "3", "colour": "none" }),
        )
        .expect_err("the colour is refused");
    let message = failure.to_string();
    assert!(message.contains("`none` is not one"), "the refusal says why: {message}");
}

/// A rule that says **nothing** about the look of the cells is refused:
/// it must name the format, the fill, the colour, or any two of them.
#[test]
fn a_rule_that_says_nothing_about_the_look_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute(
            "grid.conditional.set",
            &json!({ "sheet": sheet, "range": "A1:A2", "op": ">", "value": "3" }),
        )
        .expect_err("the rule is refused");
    let message = failure.to_string();
    assert!(message.contains("name what the rule says"), "the refusal says why: {message}");
}
