//! The **time** and the **days** of a date.

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

/// A **time** is a fraction of a day.
#[test]
fn a_time_is_a_fraction_of_a_day() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=TIME(12,0,0)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(0.5), "the half of the day");
    formula(&mut session, sheet, "A2", "=TIME(6,0,0)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(0.25), "and the quarter");
}

/// The **parts** of a time are taken.
#[test]
fn the_parts_of_a_time_are_taken() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=HOUR(TIME(13,45,30))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(13.0), "the hour");
    formula(&mut session, sheet, "A2", "=MINUTE(TIME(13,45,30))");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(45.0), "the minute");
    formula(&mut session, sheet, "A3", "=SECOND(TIME(13,45,30))");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(30.0), "the second");
}

/// The **day of the week** is a number, and the number says where the
/// week begins.
#[test]
fn the_day_of_the_week_is_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // The 26th of October 2026 is a Monday.
    formula(&mut session, sheet, "A1", "=WEEKDAY(DATE(2026,10,26))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(2.0), "the Sunday is one");
    formula(&mut session, sheet, "A2", "=WEEKDAY(DATE(2026,10,26),2)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(1.0), "the Monday is one");
    formula(&mut session, sheet, "A3", "=WEEKDAY(DATE(2026,10,26),3)");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(0.0), "and the Monday is zero");
}

/// A date a number of **months** away is found, and the day of the
/// month is clamped.
#[test]
fn a_date_a_number_of_months_away_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=EDATE(DATE(2026,1,31),1)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(46081.0), "the 28th of February");
    formula(&mut session, sheet, "A2", "=EOMONTH(DATE(2026,1,15),1)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(46081.0), "the last day of February");
}

/// The number of **days** between two dates.
#[test]
fn the_number_of_days_between_two_dates() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DAYS(DATE(2026,10,26),DATE(2026,10,20))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(6.0), "six days");
}

/// A **return type** that is not one of the tree's is an error.
#[test]
fn a_return_type_that_is_not_one_of_the_trees_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=WEEKDAY(DATE(2026,10,26),5)");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#NUM!".to_string()), "it is an error");
}

/// A **time** that is not a time is an error.
#[test]
fn a_time_that_is_not_a_time_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=TIME(-1,0,0)");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#NUM!".to_string()), "it is an error");
}
