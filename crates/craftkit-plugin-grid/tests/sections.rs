//! The **sections** of a format.
//!
//! A format may have a different code for the positive numbers, the
//! negative ones and the zero, and the codes are separated by a
//! semicolon. The tree's rule about the negative section is the one
//! that matters: it shows the absolute value, and the sign is not
//! written.

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

/// A cell with a **value** and a **format**.
fn a_cell(session: &mut Session, sheet: u64, cell: &str, value: f64, format: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": format }))
        .expect("the format is set");
}

/// A format with **one** section writes the sign.
#[test]
fn one_section_writes_the_sign() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -1.5, "0.00");
    assert_eq!(text_of(&mut session, sheet, "A1"), "-1.50", "the sign is written");
}

/// The **negative** section shows the absolute value: the sign is not
/// written, because the section says how a negative number looks.
#[test]
fn the_negative_section_shows_the_absolute_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -1.5, "0.00;0.00");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "and there is no sign");
}

/// The **first** section writes the positive numbers.
#[test]
fn the_first_section_writes_the_positive_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1.5, "0.00;0.000");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "the first section writes it");
}

/// With **two** sections the first one writes the zero as well.
#[test]
fn two_sections_write_the_zero_with_the_first() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.0, "0.00;0.000");
    assert_eq!(text_of(&mut session, sheet, "A1"), "0.00", "the first section writes the zero");
}

/// With **three** sections the third one writes the zero.
#[test]
fn three_sections_write_the_zero_with_the_third() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 0.0, "0.00;0.000;0.0000");
    assert_eq!(text_of(&mut session, sheet, "A1"), "0.0000", "the third section writes the zero");
}

/// A **currency** may be a section too.
#[test]
fn a_currency_may_be_a_section() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -1.5, "\"$\"#,##0.00;\"$\"#,##0.00");
    assert_eq!(text_of(&mut session, sheet, "A1"), "$1.50", "the symbol is there and the sign is not");
}

/// A **fraction** may be a section too.
#[test]
fn a_fraction_may_be_a_section() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", -0.5, "# ?/?;# ?/?");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1/2", "the fraction is there and the sign is not");
}

/// The **sections** may be of different shapes.
#[test]
fn the_sections_may_be_of_different_shapes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1234.5, "0.00;0;0.0000");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1234.50", "the first section writes it");
}

/// A semicolon inside a **literal** is not a separator.
#[test]
fn a_semicolon_in_a_literal_is_not_a_separator() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", 1.5, "\"a;b\"#,##0.00");
    assert_eq!(text_of(&mut session, sheet, "A1"), "a;b1.50", "the symbol has a semicolon in it");
}

/// A format with a **fourth** section is refused when that section is
/// not a text section: the text section is here, and a number's shape
/// is not one.
#[test]
fn a_fourth_section_that_is_not_a_text_section_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect("the value is set");
    let refused = session.execute(
        "grid.cell.format",
        &json!({ "sheet": sheet, "cell": "A1", "format": "0.00;0.00;0.00;0.00" }),
    );
    assert!(refused.is_err(), "a number's shape is not a text section");
}

/// A format with an **empty** section is refused.
#[test]
fn an_empty_section_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect("the value is set");
    let refused =
        session.execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00;;0.00" }));
    assert!(refused.is_err(), "an empty section is not a format this tree has");
}

/// A section that is **not a format** is refused.
#[test]
fn a_section_that_is_not_a_format_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect("the value is set");
    let refused = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00;nonsense" }));
    assert!(refused.is_err(), "the second section is not a format");
}

/// Sections in a **style** are sections of every cell that names the
/// style.
#[test]
fn sections_in_a_style_are_sections() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": 1, "name": "plain", "format": "0.00;0.00" }))
        .expect("the style is added");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": -1.5 }))
        .expect("the value is set");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "plain" }))
        .expect("the style is applied");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1.50", "and the sign is not written");
}
