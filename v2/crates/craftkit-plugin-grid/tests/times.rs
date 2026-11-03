//! The **times** of a cell.
//!
//! A time in a spreadsheet is a **number** and a **format**: the number
//! is the fraction of the day, and the format says that the number is a
//! time and how it is written. The tree's clock shapes and its elapsed
//! shapes are both here.

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

/// Put a **number** in a cell.
fn number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// Put a **format** on a cell.
fn format(session: &mut Session, sheet: u64, cell: &str, code: &str) {
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": code }))
        .expect("the cell is formatted");
}

/// The **text** a cell shows.
fn shown(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell shows text").to_string()
}

/// The **value** of a cell, as the payload holds it.
fn value_of(session: &Session, sheet: u64, cell: &str) -> Option<Value> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].clone())
}

/// A time is put in a cell and it shows the time.
#[test]
fn a_time_is_put_in_a_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "13:45" }))
        .expect("the time is set");
    assert_eq!(shown(&mut session, sheet, "A1"), "13:45", "the cell shows the time");
}

/// A time is a **number**: the fraction of the day.
#[test]
fn a_time_is_a_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "12:00" }))
        .expect("the time is set");
    let value = value_of(&session, sheet, "A1").expect("the cell holds a value");
    assert_eq!(value, json!(0.5), "the noon is half a day");
}

/// The clock is a clock of **twenty-four** hours.
#[test]
fn the_clock_is_of_twenty_four_hours() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 18.0 / 24.0);
    format(&mut session, sheet, "A1", "h:mm");
    assert_eq!(shown(&mut session, sheet, "A1"), "18:00", "the evening is eighteen");
}

/// The clock of **twelve** hours says which half of the day it is.
#[test]
fn the_clock_of_twelve_hours_says_the_half() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 18.0 / 24.0);
    format(&mut session, sheet, "A1", "h:mm AM/PM");
    assert_eq!(shown(&mut session, sheet, "A1"), "6:00 PM", "the evening is six in the afternoon");
}

/// The **midnight** is the twelve of the morning.
#[test]
fn the_midnight_is_twelve_in_the_morning() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 0.0);
    format(&mut session, sheet, "A1", "h:mm AM/PM");
    assert_eq!(shown(&mut session, sheet, "A1"), "12:00 AM", "the midnight is twelve");
}

/// The **noon** is the twelve of the afternoon.
#[test]
fn the_noon_is_twelve_in_the_afternoon() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 0.5);
    format(&mut session, sheet, "A1", "h:mm AM/PM");
    assert_eq!(shown(&mut session, sheet, "A1"), "12:00 PM", "the noon is twelve");
}

/// The **seconds** are written.
#[test]
fn the_seconds_are_written() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", (13.0 * 3600.0 + 45.0 * 60.0 + 30.0) / 86_400.0);
    format(&mut session, sheet, "A1", "h:mm:ss");
    assert_eq!(shown(&mut session, sheet, "A1"), "13:45:30", "the second is written");
}

/// The time is rounded to the nearest **second**, and a second that
/// rounds to the next one carries into the minute.
#[test]
fn the_time_is_rounded_to_the_second() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 0.5 + 59.6 / 86_400.0);
    format(&mut session, sheet, "A1", "h:mm:ss");
    assert_eq!(shown(&mut session, sheet, "A1"), "12:01:00", "the second carries into the minute");
}

/// The **elapsed** hours are the hours of the whole number, and they do
/// not stop at twenty-four.
#[test]
fn the_elapsed_hours_do_not_stop_at_twenty_four() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 1.5);
    format(&mut session, sheet, "A1", "[h]:mm");
    assert_eq!(shown(&mut session, sheet, "A1"), "36:00", "the day and a half is thirty-six hours");
}

/// The **elapsed** minutes are the minutes of the whole number.
#[test]
fn the_elapsed_minutes_are_the_minutes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    number(&mut session, sheet, "A1", 1.0 / 24.0);
    format(&mut session, sheet, "A1", "[mm]:ss");
    assert_eq!(shown(&mut session, sheet, "A1"), "60:00", "the hour is sixty minutes");
}

/// A time is written as it was written: `1:45 pm` is on a clock of
/// twelve.
#[test]
fn a_time_on_a_clock_of_twelve_is_written_as_it_was() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "1:45 pm" }))
        .expect("the time is set");
    assert_eq!(shown(&mut session, sheet, "A1"), "1:45 PM", "the afternoon is written as it was");
}

/// The noon is the **twelve** of the afternoon when it is written with
/// a half of the day.
#[test]
fn the_noon_written_with_a_half_of_the_day() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "12:00 pm" }))
        .expect("the time is set");
    assert_eq!(shown(&mut session, sheet, "A1"), "12:00 PM", "the noon is the twelve of the afternoon");
}

/// The **midnight** written with a half of the day is the zero of the
/// clock of twenty-four.
#[test]
fn the_midnight_written_with_a_half_of_the_day() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "12:00 am" }))
        .expect("the time is set");
    let value = value_of(&session, sheet, "A1").expect("the cell holds a value");
    assert_eq!(value, json!(0.0), "the midnight is the zero of the day");
}

/// A time that **did not happen** is refused: the sixtieth minute is
/// not a minute.
#[test]
fn a_time_that_did_not_happen_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "13:60" }));
    assert!(outcome.is_err(), "the sixtieth minute is refused");
}

/// A number on its own is not a **time**.
#[test]
fn a_number_on_its_own_is_not_a_time() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.cell.time", &json!({ "sheet": sheet, "cell": "A1", "time": "13" }));
    assert!(outcome.is_err(), "the lone number is refused");
}

/// A format that is not a **time format** is refused.
#[test]
fn a_format_that_is_not_a_time_format_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute(
        "grid.cell.time",
        &json!({ "sheet": sheet, "cell": "A1", "time": "13:45", "format": "0.00" }),
    );
    assert!(outcome.is_err(), "the number format is refused");
}

/// A time may be written with the format it is **not** written in.
#[test]
fn a_time_may_be_written_with_another_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.time",
            &json!({ "sheet": sheet, "cell": "A1", "time": "13:45", "format": "h:mm:ss" }),
        )
        .expect("the time is set");
    assert_eq!(shown(&mut session, sheet, "A1"), "13:45:00", "the second is written as a zero");
}
