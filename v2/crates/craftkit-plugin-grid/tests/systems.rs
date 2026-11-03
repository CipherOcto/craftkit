//! The **date system** of a workbook.
//!
//! The tree's workbook says which calendar its serials are counted in:
//! the 1900 system, with the 29th of February of 1900 in it that never
//! happened, or the 1904 system, without one.

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

/// A **workbook** with one sheet, and the two ids.
fn a_book(session: &mut Session) -> (u64, u64) {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    (1, outcome.data["id"].as_u64().expect("the sheet has an id"))
}

/// The **value** of a cell, as the payload holds it.
fn value_of(session: &Session, sheet: u64, cell: &str) -> Option<f64> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell]["value"].as_f64()
}

/// The **text** a cell shows.
fn shown(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell shows text").to_string()
}

/// A workbook counts its days in the **1900** system unless it says
/// otherwise.
#[test]
fn a_workbook_is_in_the_1900_system() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    let node = session.document().node(book).expect("the workbook is there");
    assert_eq!(node.data["date_system"].as_str(), Some("1900"), "the workbook says its system");
    let _ = sheet;
}

/// The system of a workbook may be **changed**.
#[test]
fn the_system_may_be_changed() {
    let mut session = session();
    let (book, _) = a_book(&mut session);
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    let node = session.document().node(book).expect("the workbook is there");
    assert_eq!(node.data["date_system"].as_str(), Some("1904"), "the workbook says its system");
}

/// A system that is **not** a system is refused.
#[test]
fn a_system_that_is_not_a_system_is_refused() {
    let mut session = session();
    let (book, _) = a_book(&mut session);
    let outcome =
        session.execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1969" }));
    assert!(outcome.is_err(), "the system that is not one is refused");
}

/// The two systems are **1462 days** apart: the same date is the serial
/// 1462 higher in the 1900 system.
#[test]
fn the_two_systems_are_1462_days_apart() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    let in_1900 = value_of(&session, sheet, "A1").expect("the cell holds a number");
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    let in_1904 = value_of(&session, sheet, "A1").expect("the cell holds a number");
    assert_eq!(in_1900 - in_1904, 1462.0, "the serials are 1462 days apart");
}

/// The **day of the week** of a date is the tree's fictitious one in
/// the 1900 system.
///
/// The 1st of January 1900 was really a Monday, and the tree's 1900
/// calendar has the day that never happened in it, so the tree's
/// calendar says the Sunday — and every date after the 1st of March
/// of 1900 is the day it really is.
#[test]
fn the_weekday_is_the_one_of_the_system() {
    let mut session = session();
    let (_, sheet) = a_book(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "1900-01-01" }))
        .expect("the date is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "ddd" }))
        .expect("the cell is formatted");
    assert_eq!(shown(&mut session, sheet, "A1"), "Sun", "the 1900 calendar says the Sunday");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A2", "date": "2026-10-26" }))
        .expect("the date is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A2", "format": "ddd" }))
        .expect("the cell is formatted");
    assert_eq!(shown(&mut session, sheet, "A2"), "Mon", "and after March the day is the real one");
}

/// The **same serial** is a different date in the two systems.
#[test]
fn the_same_serial_is_a_different_date() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1000 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "yyyy-mm-dd" }))
        .expect("the cell is formatted");
    let in_1900 = shown(&mut session, sheet, "A1");
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    let in_1904 = shown(&mut session, sheet, "A1");
    assert_ne!(in_1900, in_1904, "the same number is two different dates");
}

/// A date that is **before** the calendar of the system is refused.
#[test]
fn a_date_before_the_calendar_is_refused() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    let outcome =
        session.execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "1903-12-31" }));
    assert!(outcome.is_err(), "the date before the 1904 calendar is refused");
}

/// The **1900** system has a day in it that never happened, and the
/// 1904 system has not.
#[test]
fn the_1904_system_has_no_day_that_never_happened() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "1904-01-01" }))
        .expect("the date is set");
    assert_eq!(value_of(&session, sheet, "A1"), Some(0.0), "the first of January 1904 is the serial 0");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A2", "date": "1904-03-01" }))
        .expect("the date is set");
    assert_eq!(value_of(&session, sheet, "A2"), Some(60.0), "and the first of March is the serial 60");
}

/// The system belongs to the **workbook**: another workbook keeps its
/// own.
#[test]
fn the_system_belongs_to_the_workbook() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    session
        .execute("grid.workbook.add", &json!({ "name": "the other book" }))
        .expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 2, "name": "Sheet1" }))
        .expect("the sheet is added");
    let other_sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    session
        .execute("grid.cell.date", &json!({ "sheet": other_sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    let in_1904 = value_of(&session, sheet, "A1").expect("the cell holds a number");
    let in_1900 = value_of(&session, other_sheet, "A1").expect("the cell holds a number");
    assert_eq!(in_1900 - in_1904, 1462.0, "the two books count their days differently");
}

/// A **fill** of dates is counted in the system's calendar.
#[test]
fn a_fill_of_dates_is_in_the_system() {
    let mut session = session();
    let (book, sheet) = a_book(&mut session);
    session
        .execute("grid.workbook.date.system", &json!({ "workbook": book, "system": "1904" }))
        .expect("the system is set");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3", "mode": "days" }))
        .expect("the range is filled");
    assert_eq!(shown(&mut session, sheet, "A2"), "2026-10-27", "the day is a day on");
    assert_eq!(shown(&mut session, sheet, "A3"), "2026-10-28", "and the day after that");
}
