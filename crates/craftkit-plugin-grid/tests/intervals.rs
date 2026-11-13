//! The **days between two dates**: `DAYS360` and `DATEDIF`.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **days** between two dates under the year of three hundred and
/// sixty days.
#[test]
fn the_days_under_the_year_of_three_hundred_and_sixty_days() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DAYS360(DATE(2020,1,31),DATE(2020,3,31))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(60.0), "two months of thirty days");
    formula(&mut session, sheet, "A2", "=DAYS360(DATE(2020,1,1),DATE(2021,1,1))");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(360.0), "a year is three hundred and sixty days");
}

/// The **European method** is the tree's, and it differs at the end
/// of February.
#[test]
fn the_european_method_differs_at_the_end_of_february() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DAYS360(DATE(2021,2,28),DATE(2021,3,30))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(30.0), "the last day of February is the thirtieth");
    formula(&mut session, sheet, "A2", "=DAYS360(DATE(2021,2,28),DATE(2021,3,30),TRUE)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(32.0), "and the European one is not");
}

/// The **days** between two dates, and the **whole months**, the
/// **whole years**, the months that are left of the year, the days
/// that are left of the month, and the days that are left of the year.
#[test]
fn the_time_between_two_dates_in_the_unit_that_is_asked_for() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"D\")");
    formula(&mut session, sheet, "A2", "=DAYS(DATE(2022,3,20),DATE(2020,1,15))");
    let days = value_of(&mut session, sheet, "A1");
    assert_eq!(days, value_of(&mut session, sheet, "A2"), "the days are the days: {days}");
    formula(&mut session, sheet, "A3", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"M\")");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(26.0), "twenty-six whole months");
    formula(&mut session, sheet, "A4", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"Y\")");
    assert_eq!(value_of(&mut session, sheet, "A4"), json!(2.0), "two whole years");
    formula(&mut session, sheet, "A5", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"YM\")");
    assert_eq!(value_of(&mut session, sheet, "A5"), json!(2.0), "two months of the year that is left");
    formula(&mut session, sheet, "A6", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"MD\")");
    assert_eq!(value_of(&mut session, sheet, "A6"), json!(5.0), "five days of the month");
    formula(&mut session, sheet, "A7", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"YD\")");
    assert_eq!(value_of(&mut session, sheet, "A7"), json!(64.0), "and sixty-four days of the year");
}

/// A start date **after** the end date is an error, and so is a unit
/// that is not one of the six.
#[test]
fn a_start_date_after_the_end_date_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=DATEDIF(DATE(2022,3,20),DATE(2020,1,15),\"D\")");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"].as_str(), Some("#NUM!"), "it says why: {value}");
    formula(&mut session, sheet, "A2", "=DATEDIF(DATE(2020,1,15),DATE(2022,3,20),\"W\")");
    let value = value_of(&mut session, sheet, "A2");
    assert_eq!(value["error"].as_str(), Some("#NUM!"), "it says why: {value}");
}
