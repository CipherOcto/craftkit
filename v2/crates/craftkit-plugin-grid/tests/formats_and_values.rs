//! The **formats** and **values** of a fill.
//!
//! The tree has two more ways of filling: one that copies the style
//! and not the value, and one that copies the value and not the style.

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

/// The **format** of a cell, as the payload holds it.
fn format_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"].get(cell)?.get("format")?.as_str().map(|code| code.to_string())
}

/// Put a **value** in a cell.
fn set_value(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// Put a **format** on a cell.
fn set_format(session: &mut Session, sheet: u64, cell: &str, format: &str) {
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": format }))
        .expect("the format is set");
}

/// Fill a range.
fn fill(session: &mut Session, sheet: u64, from: &str, to: &str, mode: &str) {
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": from, "to": to, "mode": mode }))
        .expect("the range is filled");
}

/// The tree's **formats** mode gives the cell the format of the source
/// and keeps its own value.
#[test]
fn the_formats_mode_gives_the_format_and_keeps_the_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_value(&mut session, sheet, "A1", 7.0);
    set_format(&mut session, sheet, "A1", "0.00");
    set_value(&mut session, sheet, "A2", 3.0);
    fill(&mut session, sheet, "A1", "A1:A2", "formats");
    assert_eq!(format_of(&session, sheet, "A2").as_deref(), Some("0.00"), "the format is copied");
    assert_eq!(text_of(&mut session, sheet, "A2"), "3.00", "and the value is the cell's own");
}

/// The tree's **values** mode gives the cell the value of the source
/// and keeps its own format.
#[test]
fn the_values_mode_gives_the_value_and_keeps_the_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_value(&mut session, sheet, "A1", 7.0);
    set_format(&mut session, sheet, "A2", "0.00");
    fill(&mut session, sheet, "A1", "A1:A2", "values");
    assert_eq!(text_of(&mut session, sheet, "A2"), "7.00", "the value is copied");
    assert_eq!(format_of(&session, sheet, "A2").as_deref(), Some("0.00"), "and the format is kept");
}

/// The tree's values mode **extends like a plain drag**: a lone
/// number is copied and not counted up.
#[test]
fn the_values_mode_extends_like_a_plain_drag() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_value(&mut session, sheet, "A1", 7.0);
    fill(&mut session, sheet, "A1", "A1:A3", "values");
    assert_eq!(text_of(&mut session, sheet, "A2"), "7", "the number is copied");
    assert_eq!(text_of(&mut session, sheet, "A3"), "7", "and so is the next");
}

/// The tree's fill **replaces** the style of the cell with the style of
/// the source, and a source with no style takes the style away.
#[test]
fn a_source_with_no_format_takes_the_format_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_value(&mut session, sheet, "A1", 7.0);
    set_format(&mut session, sheet, "A2", "0.00");
    fill(&mut session, sheet, "A1", "A1:A2", "copy");
    assert_eq!(format_of(&session, sheet, "A2"), None, "the format is gone");
    assert_eq!(text_of(&mut session, sheet, "A2"), "7", "and the value is there");
}

/// The tree's formats mode with a source that has **no** format takes
/// the format of the cell away.
#[test]
fn the_formats_mode_with_no_format_takes_the_format_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_value(&mut session, sheet, "A1", 7.0);
    set_value(&mut session, sheet, "A2", 3.0);
    set_format(&mut session, sheet, "A2", "0.00");
    fill(&mut session, sheet, "A1", "A1:A2", "formats");
    assert_eq!(format_of(&session, sheet, "A2"), None, "the format is gone");
    assert_eq!(text_of(&mut session, sheet, "A2"), "3", "and the value is the cell's own");
}
