//! The **week** of a day in its year.

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
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **week** of a day, with the kinds the tree has.
#[test]
fn the_week_of_a_day_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=WEEKNUM(DATE(2024,1,1))"),
        json!(1.0),
        "the first day of the year"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=WEEKNUM(DATE(2024,1,7))"),
        json!(2.0),
        "and the Sunday that begins the second week"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=WEEKNUM(DATE(2024,1,7),2)"),
        json!(1.0),
        "and the same day in a week that begins on the Monday"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=WEEKNUM(DATE(2026,10,7))"),
        json!(41.0),
        "and a day of October"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=WEEKNUM(DATE(2021,1,1),21)"),
        json!(53.0),
        "and the ISO week of the first day of a year"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=WEEKNUM(DATE(2024,1,1),3)"),
        json!({ "error": "#NUM!" }),
        "and a kind that is no kind is an error"
    );
}

/// The **ISO week** of a day.
#[test]
fn the_iso_week_of_a_day_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ISOWEEKNUM(DATE(2021,1,1))"),
        json!(53.0),
        "the first day of a year"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ISOWEEKNUM(DATE(2026,10,7))"),
        json!(41.0),
        "and a day of October"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ISOWEEKNUM(DATE(2024,12,30))"),
        json!(1.0),
        "and the last days of a year that belong to the next one"
    );
}
