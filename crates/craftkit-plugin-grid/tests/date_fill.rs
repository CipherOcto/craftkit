//! The **filling of dates**.
//!
//! The tree has four ways of filling a range of dates, and they step
//! through the calendar: days, weekdays, months and years.

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

/// Put a **date** in a cell.
fn set_date(session: &mut Session, sheet: u64, cell: &str, date: &str) {
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": cell, "date": date }))
        .expect("the date is set");
}

/// Fill a range.
fn fill(session: &mut Session, sheet: u64, from: &str, to: &str, mode: &str) {
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": from, "to": to, "mode": mode }))
        .expect("the range is filled");
}

/// A fill of **days** steps by days.
#[test]
fn a_fill_of_days_steps_by_days() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-26");
    fill(&mut session, sheet, "A1", "A1:A3", "days");
    assert_eq!(text_of(&mut session, sheet, "A2"), "2026-10-27", "the day after");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2026-10-28", "and the day after that");
}

/// Two dates a week apart are a fill of **every seventh day**.
#[test]
fn two_dates_a_week_apart_step_by_a_week() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-26");
    set_date(&mut session, sheet, "A2", "2026-11-02");
    fill(&mut session, sheet, "A1:A2", "A1:A4", "days");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2026-11-09", "the week after");
    assert_eq!(text_of(&mut session, sheet, "A4"), "2026-11-16", "and the week after that");
}

/// A fill of **weekdays** skips the Saturday and the Sunday.
#[test]
fn a_fill_of_weekdays_skips_the_weekend() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-30");
    fill(&mut session, sheet, "A1", "A1:A3", "weekdays");
    assert_eq!(text_of(&mut session, sheet, "A2"), "2026-11-02", "the Monday after the Friday");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2026-11-03", "and the Tuesday after that");
}

/// A fill of **months** keeps the day of the month and clamps it to
/// the month it lands in — and it does not drift.
#[test]
fn a_fill_of_months_clamps_the_day_and_does_not_drift() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-01-31");
    fill(&mut session, sheet, "A1", "A1:A4", "months");
    assert_eq!(text_of(&mut session, sheet, "A2"), "2026-02-28", "the February is short");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2026-03-31", "and the March is long again");
    assert_eq!(text_of(&mut session, sheet, "A4"), "2026-04-30", "and the April is shorter");
}

/// A fill of **years** steps by years.
#[test]
fn a_fill_of_years_steps_by_years() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-26");
    fill(&mut session, sheet, "A1", "A1:A3", "years");
    assert_eq!(text_of(&mut session, sheet, "A2"), "2027-10-26", "the year after");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2028-10-26", "and the year after that");
}

/// Two dates **years** apart are a fill of every year, and the day of
/// the month is kept.
#[test]
fn two_dates_years_apart_step_by_years() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-26");
    set_date(&mut session, sheet, "A2", "2028-10-26");
    fill(&mut session, sheet, "A1:A2", "A1:A4", "years");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2030-10-26", "two years on");
    assert_eq!(text_of(&mut session, sheet, "A4"), "2032-10-26", "and two years again");
}

/// A source that is **not dates** extends like a plain drag: a lone
/// number is copied.
#[test]
fn a_source_that_is_not_dates_extends_like_a_plain_drag() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 7 })).expect("set");
    fill(&mut session, sheet, "A1", "A1:A3", "days");
    assert_eq!(text_of(&mut session, sheet, "A2"), "7", "the number is copied");
    assert_eq!(text_of(&mut session, sheet, "A3"), "7", "and so is the next");
}

/// A **single date** in a plain series counts a day on: the tree's
/// rule for a single date, and not the tree's rule for a single
/// number.
#[test]
fn a_single_date_in_a_plain_series_counts_a_day_on() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-26");
    fill(&mut session, sheet, "A1", "A1:A2", "series");
    assert_eq!(text_of(&mut session, sheet, "A2"), "2026-10-27", "the day after");
}

/// Two dates of the **same day of the month** go on by months, and not
/// by a line through them.
#[test]
fn two_dates_of_the_same_day_go_on_by_months() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-01-31");
    set_date(&mut session, sheet, "A2", "2026-03-31");
    fill(&mut session, sheet, "A1:A2", "A1:A4", "series");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2026-05-31", "the months go on");
    assert_eq!(text_of(&mut session, sheet, "A4"), "2026-07-31", "and they go on by two");
}

/// Two dates of **different days of the month** go on by a line
/// through them: the tree's rule is the same day of the month or
/// nothing.
#[test]
fn two_dates_of_different_days_go_on_by_a_line() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-01-31");
    set_date(&mut session, sheet, "A2", "2026-02-28");
    fill(&mut session, sheet, "A1:A2", "A1:A3", "series");
    assert_eq!(text_of(&mut session, sheet, "A3"), "2026-03-28", "the line through the two");
}

/// A mode that is **not a mode** is refused, and the refusal names
/// the eight that are.
#[test]
fn a_mode_that_is_not_a_mode_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_date(&mut session, sheet, "A1", "2026-10-26");
    let error = session
        .execute(
            "grid.range.fill",
            &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3", "mode": "quarters" }),
        )
        .expect_err("the mode is refused");
    let message = error.to_string();
    assert!(message.contains("is not a mode"), "the refusal says why: {message}");
    assert!(message.contains("weekdays"), "and it names the date modes: {message}");
}
