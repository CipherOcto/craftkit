//! The **lists** of a fill.
//!
//! The tree's fill steps through a list: the days of the week, the
//! months and the quarters.

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

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell has a text");
    outcome.data["text"].as_str().unwrap_or_default().to_string()
}

/// Put a **text** in a cell.
fn set_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the text is set");
}

/// Fill a range.
fn fill(session: &mut Session, sheet: u64, from: &str, to: &str) {
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": from, "to": to }))
        .expect("the range is filled");
}

/// A fill of a **day of the week** steps through the week.
#[test]
fn a_fill_of_a_day_steps_through_the_week() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "Mon");
    fill(&mut session, sheet, "A1", "A1:A4");
    assert_eq!(text_of(&mut session, sheet, "A2"), "Tue", "the next day");
    assert_eq!(text_of(&mut session, sheet, "A3"), "Wed", "and the next");
    assert_eq!(text_of(&mut session, sheet, "A4"), "Thu", "and the next");
}

/// A fill of a **day of the week** in full steps through the week.
#[test]
fn a_fill_of_a_day_in_full_steps_through_the_week() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "Monday");
    fill(&mut session, sheet, "A1", "A1:A3");
    assert_eq!(text_of(&mut session, sheet, "A2"), "Tuesday", "the next day");
    assert_eq!(text_of(&mut session, sheet, "A3"), "Wednesday", "and the next");
}

/// A fill of a **month** steps through the months.
#[test]
fn a_fill_of_a_month_steps_through_the_months() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "Jan");
    fill(&mut session, sheet, "A1", "A1:A3");
    assert_eq!(text_of(&mut session, sheet, "A2"), "Feb", "the next month");
    assert_eq!(text_of(&mut session, sheet, "A3"), "Mar", "and the next");
}

/// A fill of a **quarter** steps through the quarters.
#[test]
fn a_fill_of_a_quarter_steps_through_the_quarters() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "Q1");
    fill(&mut session, sheet, "A1", "A1:A4");
    assert_eq!(text_of(&mut session, sheet, "A2"), "Q2", "the next quarter");
    assert_eq!(text_of(&mut session, sheet, "A4"), "Q4", "and the last");
}

/// The list **wraps around**: a fill that runs past the end of the
/// week starts again at the beginning of it.
#[test]
fn the_list_wraps_around() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "Fri");
    fill(&mut session, sheet, "A1", "A1:A5");
    assert_eq!(text_of(&mut session, sheet, "A2"), "Sat", "the next day");
    assert_eq!(text_of(&mut session, sheet, "A3"), "Sun", "and the next");
    assert_eq!(text_of(&mut session, sheet, "A4"), "Mon", "and the week begins again");
    assert_eq!(text_of(&mut session, sheet, "A5"), "Tue", "and goes on");
}

/// Two days **apart** are a fill of every second day.
#[test]
fn two_days_apart_are_a_fill_of_every_second_day() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "Mon");
    set_text(&mut session, sheet, "A2", "Wed");
    fill(&mut session, sheet, "A1:A2", "A1:A4");
    assert_eq!(text_of(&mut session, sheet, "A3"), "Fri", "every second day");
    assert_eq!(text_of(&mut session, sheet, "A4"), "Sun", "and every second day");
}

/// The **case** of the source is the case of the list: a source in
/// capitals fills with capitals.
#[test]
fn the_case_of_the_source_is_the_case_of_the_list() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "MON");
    fill(&mut session, sheet, "A1", "A1:A2");
    assert_eq!(text_of(&mut session, sheet, "A2"), "TUE", "the list is in capitals");
}

/// A text that is **not in a list** is copied.
#[test]
fn a_text_that_is_not_in_a_list_is_copied() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_text(&mut session, sheet, "A1", "hello");
    fill(&mut session, sheet, "A1", "A1:A2");
    assert_eq!(text_of(&mut session, sheet, "A2"), "hello", "the text is copied");
}
