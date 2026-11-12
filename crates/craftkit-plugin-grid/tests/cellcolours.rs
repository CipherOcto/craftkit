//! The **colours** of a cell: the colour of its text and the colour
//! behind it.
//!
//! The tree's rules: a colour is `#rrggbb`, and **none** takes the
//! colour away.

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

/// The **fill** of a cell, as the sheet keeps it.
fn fill_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell]["fill"].as_str().map(|fill| fill.to_string())
}

/// The **colour** of the text of a cell, as the sheet keeps it.
fn colour_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell]["font"]["colour"].as_str().map(|colour| colour.to_string())
}

/// The text of the cell takes a **colour**.
#[test]
fn the_text_of_the_cell_takes_a_colour() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.font.colour", &json!({ "sheet": sheet, "cell": "A1", "colour": "#FFFF00" }))
        .expect("the text is coloured");
    assert_eq!(outcome.data["colour"], json!("#ffff00"), "the colour is written in lower case");
    assert_eq!(colour_of(&session, sheet, "A1"), Some("#ffff00".to_string()), "and the sheet keeps it");
}

/// **None** takes the colour away.
#[test]
fn none_takes_the_colour_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font.colour", &json!({ "sheet": sheet, "cell": "A1", "colour": "#ffff00" }))
        .expect("the text is coloured");
    session
        .execute("grid.cell.font.colour", &json!({ "sheet": sheet, "cell": "A1", "colour": "none" }))
        .expect("the colour is taken away");
    assert_eq!(colour_of(&session, sheet, "A1"), None, "the text has no colour of its own");
}

/// The cell is **filled** with a colour.
#[test]
fn the_cell_is_filled_with_a_colour() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.fill", &json!({ "sheet": sheet, "cell": "A1", "colour": "#336699" }))
        .expect("the cell is filled");
    assert_eq!(outcome.data["colour"], json!("#336699"), "the colour is written");
    assert_eq!(fill_of(&session, sheet, "A1"), Some("#336699".to_string()), "and the sheet keeps it");
}

/// A colour that is **not one of them** is refused.
#[test]
fn a_colour_that_is_not_one_of_them_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for colour in ["yellow", "#fff", "#gggggg", "123456"] {
        let outcome =
            session.execute("grid.cell.fill", &json!({ "sheet": sheet, "cell": "A1", "colour": colour }));
        assert!(outcome.is_err(), "the colour {colour} is refused: {outcome:?}");
    }
}

/// The colours **stay** when the cell's value changes.
#[test]
fn the_colours_stay_when_the_cells_value_changes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.fill", &json!({ "sheet": sheet, "cell": "A1", "colour": "#336699" }))
        .expect("the cell is filled");
    session
        .execute("grid.cell.font.colour", &json!({ "sheet": sheet, "cell": "A1", "colour": "#ffff00" }))
        .expect("the text is coloured");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the title" }))
        .expect("the cell is set");
    assert_eq!(fill_of(&session, sheet, "A1"), Some("#336699".to_string()), "the fill is still there");
    assert_eq!(
        colour_of(&session, sheet, "A1"),
        Some("#ffff00".to_string()),
        "and so is the colour of the text"
    );
}
