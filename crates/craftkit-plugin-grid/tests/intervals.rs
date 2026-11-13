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

/// The **fraction of a year** between two dates, under a basis.
#[test]
fn the_fraction_of_a_year_under_a_basis() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=YEARFRAC(DATE(2021,1,1),DATE(2022,1,1),3)");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        json!(1.0),
        "three hundred and sixty-five over three hundred and sixty-five"
    );
    formula(&mut session, sheet, "A2", "=YEARFRAC(DATE(2021,1,1),DATE(2022,1,1),1)");
    assert_eq!(
        value_of(&mut session, sheet, "A2"),
        json!(1.0),
        "and the actual days over the actual days of the year"
    );
    formula(&mut session, sheet, "A3", "=YEARFRAC(DATE(2021,1,1),DATE(2021,7,1),0)");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(0.5), "six months of thirty days are half a year");
    formula(&mut session, sheet, "A4", "=YEARFRAC(DATE(2021,1,1),DATE(2021,7,1),4)");
    assert_eq!(value_of(&mut session, sheet, "A4"), json!(0.5), "and the European one is the same here");
    formula(&mut session, sheet, "A5", "=YEARFRAC(DATE(2021,1,1),DATE(2023,1,1),1)");
    assert_eq!(value_of(&mut session, sheet, "A5"), json!(2.0), "two years over two years");
}

/// The two dates are put **in order**, and a basis that is not one of
/// the five is an error.
#[test]
fn the_two_dates_are_put_in_order() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=YEARFRAC(DATE(2022,1,1),DATE(2021,1,1),3)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(1.0), "the order does not matter");
    formula(&mut session, sheet, "A2", "=YEARFRAC(DATE(2021,1,1),DATE(2022,1,1),5)");
    let value = value_of(&mut session, sheet, "A2");
    assert_eq!(value["error"].as_str(), Some("#NUM!"), "it says why: {value}");
}

/// The **working days** between two dates.
#[test]
fn the_working_days_between_two_dates() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=NETWORKDAYS(DATE(2021,1,1),DATE(2021,1,31))");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        json!(21.0),
        "January of twenty twenty-one has twenty-one working days"
    );
    formula(&mut session, sheet, "A2", "=NETWORKDAYS(DATE(2021,1,31),DATE(2021,1,1))");
    assert_eq!(
        value_of(&mut session, sheet, "A2"),
        json!(-21.0),
        "and the other way round is the other sign"
    );
}

/// The date that is **n working days** after a date.
#[test]
fn the_date_that_is_n_working_days_after_a_date() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=WORKDAY(DATE(2021,1,1),5)");
    formula(&mut session, sheet, "A2", "=DATE(2021,1,8)");
    let workday = value_of(&mut session, sheet, "A1");
    assert_eq!(
        workday,
        value_of(&mut session, sheet, "A2"),
        "the fifth working day is the eighth: {workday}"
    );
    formula(&mut session, sheet, "A3", "=WORKDAY(DATE(2021,1,1),0)");
    formula(&mut session, sheet, "A4", "=DATE(2021,1,1)");
    assert_eq!(
        value_of(&mut session, sheet, "A3"),
        value_of(&mut session, sheet, "A4"),
        "and zero is the start"
    );
}

/// The tree's **holidays** are a list, and a list is an array.
#[test]
fn the_holidays_are_not_here() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=NETWORKDAYS(DATE(2021,1,1),DATE(2021,1,31),A1:A2)");
    let value = value_of(&mut session, sheet, "B1");
    assert_eq!(value["error"].as_str(), Some("not supported yet: an array"), "it says why: {value}");
    formula(&mut session, sheet, "B2", "=WORKDAY(DATE(2021,1,1),5,A1:A2)");
    let value = value_of(&mut session, sheet, "B2");
    assert_eq!(value["error"].as_str(), Some("not supported yet: an array"), "it says why: {value}");
}

/// The **weekend** of the tree's own is the third argument of the
/// international working days.
#[test]
fn the_weekend_of_the_trees_own_is_the_third_argument() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=NETWORKDAYS.INTL(DATE(2006,1,1),DATE(2006,1,31))");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(22.0), "the ordinary weekend");
    formula(&mut session, sheet, "A2", "=NETWORKDAYS.INTL(DATE(2006,1,1),DATE(2006,2,1),7)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(24.0), "a weekend of two days");
    formula(&mut session, sheet, "A3", "=NETWORKDAYS.INTL(DATE(2006,1,1),DATE(2006,2,1),\"0000110\")");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(24.0), "and the same weekend written as text");
    formula(&mut session, sheet, "A4", "=NETWORKDAYS.INTL(DATE(2006,1,1),DATE(2006,1,31),11)");
    assert_eq!(value_of(&mut session, sheet, "A4"), json!(26.0), "a weekend of one day");
}

/// A weekend of **every** day leaves no working day at all, and a
/// weekend that is not a weekend is a number error.
#[test]
fn a_weekend_of_every_day_leaves_no_working_day() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=NETWORKDAYS.INTL(DATE(2006,1,1),DATE(2006,1,31),\"1111111\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(0.0), "no working day");
    formula(&mut session, sheet, "A2", "=WORKDAY.INTL(DATE(2012,1,1),10,\"1111111\")");
    assert_eq!(
        value_of(&mut session, sheet, "A2")["error"],
        Json::String("#VALUE!".to_string()),
        "no day to move to"
    );
    formula(&mut session, sheet, "A3", "=NETWORKDAYS.INTL(DATE(2006,1,1),DATE(2006,1,31),8)");
    assert_eq!(
        value_of(&mut session, sheet, "A3")["error"],
        Json::String("#NUM!".to_string()),
        "no such weekend"
    );
}

/// The **working day** away with a weekend of the tree's own.
#[test]
fn the_working_day_away_with_a_weekend_of_the_trees_own() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=WORKDAY.INTL(DATE(2012,1,1),90,11)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(41013.0), "the tree's own day");
    formula(&mut session, sheet, "A2", "=WORKDAY.INTL(DATE(2012,1,1),30,17)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(40944.0), "and the other one");
}
