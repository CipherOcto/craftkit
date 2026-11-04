//! The **parts** of a date.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// A date is **made** of a year, a month and a day.
#[test]
fn a_date_is_made_of_a_year_a_month_and_a_day() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATE(2026,10,26)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(46321.0), "the serial of that day");
    formula(&mut session, sheet, "A2", "=DATE(2026,1,1)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(46023.0), "and of the first of January");
}

/// The month and the day **roll over**.
#[test]
fn the_month_and_the_day_roll_over() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATE(2026,13,1)");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        json!(46388.0),
        "the thirteenth month is the next January"
    );
    formula(&mut session, sheet, "A2", "=DATE(2026,1,32)");
    assert_eq!(
        value_of(&mut session, sheet, "A2"),
        json!(46054.0),
        "and the thirty-second day is the first of February"
    );
}

/// A year **below 1900** is a year after 1900.
#[test]
fn a_year_below_1900_is_a_year_after_1900() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATE(24,1,1)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(8767.0), "the year 24 is the year 1924");
}

/// A date is **taken apart**.
#[test]
fn a_date_is_taken_apart() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=YEAR(46321)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(2026.0), "the year");
    formula(&mut session, sheet, "A2", "=MONTH(46321)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(10.0), "the month");
    formula(&mut session, sheet, "A3", "=DAY(46321)");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(26.0), "the day");
}

/// The parts of a date may be put **together again**.
#[test]
fn the_parts_of_a_date_may_be_put_together_again() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATE(YEAR(46321),MONTH(46321),DAY(46321))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(46321.0), "the same day");
}

/// A date outside the calendar is an **error**.
#[test]
fn a_date_outside_the_calendar_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATE(10000,1,1)");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#NUM!".to_string()), "it is an error");
    formula(&mut session, sheet, "A2", "=YEAR(9999999)");
    let value = value_of(&mut session, sheet, "A2");
    assert_eq!(value["error"], Json::String("#NUM!".to_string()), "and a serial that is not one");
}

/// The **1904 system** counts from another day.
#[test]
fn the_1904_system_counts_from_another_day() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": workbook, "system": "1904" }))
        .expect("the system is set");
    formula(&mut session, sheet, "A1", "=DATE(1904,1,1)");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        json!(0.0),
        "the first of January 1904 is the zeroth day"
    );
}
