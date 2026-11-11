//! The **decimals** of a format: one more of them, or one less.
//!
//! The tree's rules: a cell whose format is the General one has the
//! decimals of the number it shows, the last digit placeholder of a
//! section is the one that counts, and a section with no digit
//! placeholder is left as it is.

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

/// The **format** of a cell, as the sheet keeps it.
fn format_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell]["format"].as_str().map(|format| format.to_string())
}

/// The **General** format takes the decimals of the number the cell
/// shows.
#[test]
fn the_general_format_takes_the_decimals_of_the_number_the_cell_shows() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.25 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": 1 }))
        .expect("the decimals are moved");
    assert_eq!(
        format_of(&session, sheet, "A1"),
        Some("0.000".to_string()),
        "the cell shows two decimals and one more"
    );
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": -1 }))
        .expect("the decimals are moved back");
    assert_eq!(format_of(&session, sheet, "A1"), Some("0.00".to_string()), "and one less");
}

/// A format with decimals **gains** one.
#[test]
fn a_format_with_decimals_gains_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": 1 }))
        .expect("the decimals are moved");
    assert_eq!(format_of(&session, sheet, "A1"), Some("0.000".to_string()), "the format has three decimals");
}

/// A format with decimals **loses** one.
#[test]
fn a_format_with_decimals_loses_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": -1 }))
        .expect("the decimals are moved");
    assert_eq!(format_of(&session, sheet, "A1"), Some("0.0".to_string()), "the format has one decimal");
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": -1 }))
        .expect("the decimals are moved again");
    assert_eq!(format_of(&session, sheet, "A1"), Some("0".to_string()), "and none at all");
}

/// A format with **no decimals** is refused when one is taken away.
#[test]
fn a_format_with_no_decimals_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0" }))
        .expect("the cell is formatted");
    let outcome =
        session.execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": -1 }));
    assert!(outcome.is_err(), "there are no decimals to take away: {outcome:?}");
    assert_eq!(format_of(&session, sheet, "A1"), Some("0".to_string()), "the cell is left as it was");
}

/// The **thousands** of a format are kept.
#[test]
fn the_thousands_of_a_format_are_kept() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "#,##0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": 1 }))
        .expect("the decimals are moved");
    assert_eq!(format_of(&session, sheet, "A1"), Some("#,##0.000".to_string()), "the thousands are kept");
}

/// A format with **sections** has its sections moved one by one.
#[test]
fn a_format_with_sections_has_its_sections_moved_one_by_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00;0.000" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": 1 }))
        .expect("the decimals are moved");
    assert_eq!(
        format_of(&session, sheet, "A1"),
        Some("0.000;0.0000".to_string()),
        "both sections have one more"
    );
}

/// A **date** has no decimals to move, and it is left as it is.
#[test]
fn a_date_has_no_decimals_to_move() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "yyyy-mm-dd" }))
        .expect("the cell is formatted");
    let outcome = session
        .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": 1 }))
        .expect("the decimals are moved");
    assert!(!outcome.changed, "nothing changed: {outcome:?}");
    assert_eq!(
        format_of(&session, sheet, "A1"),
        Some("yyyy-mm-dd".to_string()),
        "the format is left as it was"
    );
}

/// A step that is **not a whole number** is refused, and so is a step
/// of nothing.
#[test]
fn a_step_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    for delta in [0.0, 0.5, -0.5] {
        let outcome = session
            .execute("grid.cell.format.decimals", &json!({ "sheet": sheet, "cell": "A1", "delta": delta }));
        assert!(outcome.is_err(), "the step {delta} is refused: {outcome:?}");
    }
}
