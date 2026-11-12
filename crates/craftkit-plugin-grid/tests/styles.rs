//! The **styles** of a workbook.
//!
//! A style is a named format of the workbook, and it is shared: a
//! hundred cells that name one style carry one format, and changing the
//! style changes every cell that names it.

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
        .expect("the text is read");
    outcome.data["text"].as_str().expect("the text is a string").to_string()
}

/// A cell with a **value**.
fn a_value(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// A **style** in the workbook.
fn a_style(session: &mut Session, name: &str, format: &str) {
    session
        .execute("grid.style.add", &json!({ "workbook": 1, "name": name, "format": format }))
        .expect("the style is added");
}

/// A style may be **added** to a workbook.
#[test]
fn a_style_may_be_added_to_a_workbook() {
    let mut session = session();
    a_sheet(&mut session);
    let outcome = session
        .execute("grid.style.add", &json!({ "workbook": 1, "name": "money", "format": "0.00" }))
        .expect("the style is added");
    assert_eq!(outcome.data["name"].as_str(), Some("money"), "the style has a name");
    assert_eq!(outcome.data["format"].as_str(), Some("0.00"), "and a format");
}

/// A cell that names a style is **written** with the style's format.
#[test]
fn a_cell_that_names_a_style_is_written_with_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_value(&mut session, sheet, "A1", 1.5);
    a_style(&mut session, "money", "0.00");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "money" }))
        .expect("the cell names the style");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "the cell is written with the style's format");
}

/// A style is **shared**: changing it changes every cell that names
/// it, and the cell's own format is not touched.
#[test]
fn changing_a_style_changes_every_cell_that_names_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_value(&mut session, sheet, "A1", 1.5);
    a_value(&mut session, sheet, "B1", 2.5);
    a_style(&mut session, "money", "0.00");
    for cell in ["A1", "B1"] {
        session
            .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": cell, "style": "money" }))
            .expect("the cell names the style");
    }
    session
        .execute("grid.style.set", &json!({ "workbook": 1, "name": "money", "format": "0.000" }))
        .expect("the style is changed");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.500", "the first cell has the new format");
    assert_eq!(text_of(&mut session, sheet, "B1"), "2.500", "and so does the second");
}

/// A style whose format is **general** does not change the cell's
/// format — the tree's rule.
#[test]
fn a_style_with_a_general_format_does_not_change_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_value(&mut session, sheet, "A1", 1.5);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    a_style(&mut session, "plain", "general");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "plain" }))
        .expect("the cell names the style");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "the cell keeps its own format");
}

/// A style that is **taken away** leaves the cell with its own format.
#[test]
fn a_style_that_is_taken_away_leaves_the_cell_with_its_own_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_value(&mut session, sheet, "A1", 1.5);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.0" }))
        .expect("the format is set");
    a_style(&mut session, "money", "0.00");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "money" }))
        .expect("the cell names the style");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "the cell is written with the style");
    session
        .execute("grid.style.remove", &json!({ "workbook": 1, "name": "money" }))
        .expect("the style is taken away");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.5", "and the cell keeps its own format");
}

/// An **empty** name means the cell has no style.
#[test]
fn an_empty_name_means_the_cell_has_no_style() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_value(&mut session, sheet, "A1", 1.5);
    a_style(&mut session, "money", "0.00");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "money" }))
        .expect("the cell names the style");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "" }))
        .expect("the cell has no style");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.5", "the cell is written as it is");
}

/// A style that is **not there** is refused: a name that is not there
/// is never quietly ignored.
#[test]
fn a_style_that_is_not_there_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "money" }));
    assert!(outcome.is_err(), "the style is refused");
}

/// A style whose name is **taken** is refused: the tree finds a style
/// by its name and without regard to case.
#[test]
fn a_style_whose_name_is_taken_is_refused() {
    let mut session = session();
    a_sheet(&mut session);
    a_style(&mut session, "money", "0.00");
    let outcome =
        session.execute("grid.style.add", &json!({ "workbook": 1, "name": "Money", "format": "0" }));
    assert!(outcome.is_err(), "the style is refused");
}

/// A style with a format this tree does not have is **refused**.
#[test]
fn a_style_with_a_format_this_tree_does_not_have_is_refused() {
    let mut session = session();
    a_sheet(&mut session);
    let outcome =
        session.execute("grid.style.add", &json!({ "workbook": 1, "name": "money", "format": "abc" }));
    assert!(outcome.is_err(), "the format is refused");
}

/// A style that is **not there** cannot be changed or taken away.
#[test]
fn a_style_that_is_not_there_cannot_be_changed_or_taken_away() {
    let mut session = session();
    a_sheet(&mut session);
    let outcome =
        session.execute("grid.style.set", &json!({ "workbook": 1, "name": "money", "format": "0" }));
    assert!(outcome.is_err(), "the style is refused");
    let outcome = session.execute("grid.style.remove", &json!({ "workbook": 1, "name": "money" }));
    assert!(outcome.is_err(), "the style is refused");
}

/// A cell keeps its **value** when it is styled.
#[test]
fn a_cell_keeps_its_value_when_it_is_styled() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_value(&mut session, sheet, "A1", 1.5);
    a_style(&mut session, "money", "0.00");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "money" }))
        .expect("the cell names the style");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "the value is still there");
}
