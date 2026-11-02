//! The **filling** of a range.
//!
//! The tree has four ways of filling, and they differ in what they do
//! with a lone number.

use craftkit_sdk::serde_json::{Value, json};
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

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **value** of a cell, as the payload holds it.
fn value_of(session: &Session, sheet: u64, cell: &str) -> Option<Value> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].clone())
}

/// The **value** of a cell, as text.
fn text_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    Some(value_of(session, sheet, cell)?.as_str()?.to_string())
}

/// A **copy** fill copies the value of the source down the range.
#[test]
fn a_copy_fill_copies_the_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3", "mode": "copy" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!(7.0)), "the second cell holds the value");
    assert_eq!(value_of(&session, sheet, "A3"), Some(json!(7.0)), "and so does the third");
}

/// The tree's **series** — its default — copies a lone number.
#[test]
fn the_series_copies_a_lone_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!(7.0)), "the number is copied");
    assert_eq!(value_of(&session, sheet, "A3"), Some(json!(7.0)), "and so is the next");
}

/// The tree's **extend** counts a lone number up.
#[test]
fn the_extend_counts_a_lone_number_up() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3", "mode": "extend" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!(8.0)), "the number counts up");
    assert_eq!(value_of(&session, sheet, "A3"), Some(json!(9.0)), "and so does the next");
}

/// The tree's **alternate** counts a lone number up and copies the
/// rest.
#[test]
fn the_alternate_counts_a_number_up_and_copies_a_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    session
        .execute(
            "grid.range.fill",
            &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2", "mode": "alternate" }),
        )
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!(8.0)), "the number counts up");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": "Item 9" }))
        .expect("set");
    session
        .execute(
            "grid.range.fill",
            &json!({ "sheet": sheet, "from": "B1", "to": "B1:B2", "mode": "alternate" }),
        )
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "B2").as_deref(), Some("Item 9"), "the text is copied");
}

/// Two numbers are a **trend**: the least squares line through them.
#[test]
fn two_numbers_are_a_trend() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": 3 })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1:A2", "to": "A1:A4" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A3"), Some(json!(5.0)), "the trend counts on by two");
    assert_eq!(value_of(&session, sheet, "A4"), Some(json!(7.0)), "and so does the next");
}

/// A **series** fill counts the number in a text on, and keeps the
/// width of the digits.
#[test]
fn a_series_fill_counts_the_number_in_a_text_on() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "Item 7" }))
        .expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("Item 8"), "the number counts on");
    assert_eq!(text_of(&session, sheet, "A3").as_deref(), Some("Item 9"), "and so does the next");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": "Q003x" }))
        .expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "B1", "to": "B1:B2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "B2").as_deref(), Some("Q004x"), "the width of the digits is kept");
}

/// A text with **no number** in it is copied.
#[test]
fn a_text_with_no_number_is_copied() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "on backorder" }))
        .expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("on backorder"), "the text is copied");
}

/// A source that is a **range** fills **lane by lane**: each column
/// continues down its own lane.
#[test]
fn a_range_source_fills_lane_by_lane() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 10 })).expect("set");
    session
        .execute(
            "grid.range.fill",
            &json!({ "sheet": sheet, "from": "A1:B1", "to": "A1:B3", "mode": "extend" }),
        )
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!(2.0)), "the first lane counts on");
    assert_eq!(value_of(&session, sheet, "B2"), Some(json!(11.0)), "and so does the second");
    assert_eq!(value_of(&session, sheet, "A3"), Some(json!(3.0)), "the first lane goes on");
    assert_eq!(value_of(&session, sheet, "B3"), Some(json!(12.0)), "and so does the second");
}

/// The fill goes **up** when the source is at the bottom of the range.
#[test]
fn the_fill_goes_up() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A3", "value": 1 })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A3", "to": "A1:A3", "mode": "extend" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!(0.0)), "the cell above counts down");
    assert_eq!(value_of(&session, sheet, "A1"), Some(json!(-1.0)), "and so does the next");
}

/// The fill goes **right** when the range is a row.
#[test]
fn the_fill_goes_right() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:C1", "mode": "extend" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "B1"), Some(json!(6.0)), "the cell beside counts on");
    assert_eq!(value_of(&session, sheet, "C1"), Some(json!(7.0)), "and so does the next");
}

/// A target that does **not hold** the source is refused.
#[test]
fn a_target_that_does_not_hold_the_source_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "B1:B3" }))
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("does not hold the source"), "the error says why: {error}");
}

/// A source that is **not at a corner** of the target is refused.
#[test]
fn a_source_that_is_not_at_a_corner_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 7 })).expect("set");
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "B2", "to": "A1:C3" }))
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("is not at a corner"), "the error says why: {error}");
}

/// A source that is **empty** is refused: there is nothing to fill
/// with.
#[test]
fn an_empty_source_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("nothing at the source"), "the error says why: {error}");
}

/// A mode that is **not a mode** is refused.
#[test]
fn a_mode_that_is_not_a_mode_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3", "mode": "guess" }))
        .expect_err("the fill is refused");
    assert!(
        error.to_string().contains("a fill is `copy`, `series`, `extend` or `alternate`"),
        "the error says why: {error}"
    );
}

/// A fill that is **too large** is refused.
#[test]
fn a_fill_that_is_too_large_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A20000" }))
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("the fill is too large"), "the error says why: {error}");
}

/// A sheet that is **not there** is refused.
#[test]
fn a_sheet_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": 1_000_000, "from": "A1", "to": "A1:A3" }))
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("there is no sheet"), "the error says why: {error}");
}
