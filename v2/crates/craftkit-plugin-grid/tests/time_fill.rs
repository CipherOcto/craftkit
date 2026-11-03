//! The **fill of a time**.
//!
//! The tree's rule: a single number under a **time** format counts an
//! hour on — as a single number under a date format counts a day on.

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

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// Put a **time** in a cell.
fn put(session: &mut Session, sheet: u64, cell: &str, time: &str) {
    session
        .execute("grid.cell.time", &json!({ "sheet": sheet, "cell": cell, "time": time }))
        .expect("the time is set");
}

/// The **text** a cell shows.
fn shown(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell shows text").to_string()
}

/// A time is filled and it counts an **hour** on.
#[test]
fn a_time_counts_an_hour_on() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "9:00");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(shown(&mut session, sheet, "A1"), "9:00", "the first cell holds the time");
    assert_eq!(shown(&mut session, sheet, "A2"), "10:00", "the second is an hour on");
    assert_eq!(shown(&mut session, sheet, "A3"), "11:00", "and the third is two");
}

/// The fill of a time carries over the **midnight**.
#[test]
fn the_fill_carries_over_the_midnight() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "23:30");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(shown(&mut session, sheet, "A2"), "0:30", "the half hour after the midnight");
}

/// A single number under a **date** format counts a **day** on.
#[test]
fn a_date_counts_a_day_on() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(shown(&mut session, sheet, "A2"), "2026-10-27", "the day is a day on");
}

/// A single number with **no format** is copied: a number is not a time
/// until a format says it is.
#[test]
fn a_number_with_no_format_is_copied() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 }))
        .expect("the cell is set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(shown(&mut session, sheet, "A2"), "5", "the number is copied");
    assert_eq!(shown(&mut session, sheet, "A3"), "5", "and copied again");
}

/// The fill of a time keeps the **format** of the time.
#[test]
fn the_fill_keeps_the_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.time",
            &json!({ "sheet": sheet, "cell": "A1", "time": "9:00", "format": "h:mm AM/PM" }),
        )
        .expect("the time is set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(shown(&mut session, sheet, "A2"), "10:00 AM", "the format is kept");
}
