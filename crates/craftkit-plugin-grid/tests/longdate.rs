//! The **long date**: the tree's own shape for a date that says the
//! weekday, the month by name, the day and the year at once.

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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **text** a cell is written with.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell has text").to_string()
}

/// A **date** in a cell, written with a format.
fn a_date(session: &mut Session, sheet: u64, cell: &str, date: &str, format: Option<&str>) {
    let mut params = json!({ "sheet": sheet, "cell": cell, "date": date });
    if let Some(format) = format {
        params["format"] = json!(format);
    }
    session.execute("grid.cell.date", &params).expect("the date is set");
}

/// The **long date** says the weekday, the month by name, the day and
/// the year.
#[test]
fn the_long_date_says_the_weekday_the_month_the_day_and_the_year() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_date(&mut session, sheet, "A1", "2026-10-26", Some("long date"));
    assert_eq!(text_of(&mut session, sheet, "A1"), "Monday, October 26, 2026", "the date is written whole");
}

/// The **name** says the code, and the code is kept.
#[test]
fn the_name_says_the_code() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_date(&mut session, sheet, "A1", "2026-10-26", Some("long date"));
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(
        outcome.data["format"].as_str(),
        Some("dddd, mmmm dd, yyyy"),
        "the code the name said is kept"
    );
}

/// The **code** may be given as it is.
#[test]
fn the_code_may_be_given_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_date(&mut session, sheet, "A1", "2026-10-26", Some("dddd, mmmm dd, yyyy"));
    assert_eq!(text_of(&mut session, sheet, "A1"), "Monday, October 26, 2026", "the date is written whole");
}

/// The day is written with **two** digits.
#[test]
fn the_day_is_written_with_two_digits() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_date(&mut session, sheet, "A1", "2026-01-02", Some("long date"));
    assert_eq!(text_of(&mut session, sheet, "A1"), "Friday, January 02, 2026", "the day has two digits");
}

/// The tree's own code carries a **locale tag**, and this tree has no
/// locale: the code with the tag is refused.
#[test]
fn the_code_with_the_locale_tag_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.cell.format",
            &json!({ "sheet": sheet, "cell": "A1", "format": "[$-F800]dddd, mmmm dd, yyyy" }),
        )
        .expect_err("the code is refused");
    assert!(error.to_string().contains("is not a format this tree has"), "the error says so: {error}");
}
