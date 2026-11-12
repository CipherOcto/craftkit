//! The **alignment** of a cell: how its text lies across it and how it
//! lies down it, and how far it is indented and whether it wraps.
//!
//! The tree's rules: naming the alignment a cell already has returns
//! it to general, an indent above zero makes the horizontal alignment
//! left, and the indent is from zero to two hundred and fifty.

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

/// The **alignment** of a cell, as the sheet keeps it.
fn align_of(session: &Session, sheet: u64, cell: &str) -> Option<craftkit_sdk::serde_json::Value> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let align = node.data["cells"][cell]["align"].clone();
    if align.is_null() { None } else { Some(align) }
}

/// The **horizontal** alignment of a cell.
fn horizontal_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let align = align_of(session, sheet, cell)?;
    align["horizontal"].as_str().map(|text| text.to_string())
}

/// The text lies **across** the cell.
#[test]
fn the_text_lies_across_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the title" }))
        .expect("the cell is set");
    let outcome = session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "center" }))
        .expect("the cell is aligned");
    assert_eq!(outcome.data["horizontal"], json!("center"), "the text is in the middle");
    assert_eq!(horizontal_of(&session, sheet, "A1"), Some("center".to_string()), "and the sheet keeps it");
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["cells"]["A1"]["value"], json!("the title"), "the cell keeps its value");
}

/// Naming the alignment a cell **already has** returns it to general.
#[test]
fn naming_the_alignment_a_cell_already_has_returns_it_to_general() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "right" }))
        .expect("the cell is aligned");
    assert_eq!(horizontal_of(&session, sheet, "A1"), Some("right".to_string()), "the text is to the right");
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "right" }))
        .expect("the cell is aligned again");
    assert_eq!(horizontal_of(&session, sheet, "A1"), Some("general".to_string()), "and back to general");
}

/// The text lies **down** the cell.
#[test]
fn the_text_lies_down_the_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "vertical": "bottom" }))
        .expect("the cell is aligned");
    assert_eq!(outcome.data["vertical"], json!("bottom"), "the text is at the bottom");
    let align = align_of(&session, sheet, "A1").expect("the cell has an alignment");
    assert_eq!(align["vertical"], json!("bottom"), "and the sheet keeps it");
    assert_eq!(align["horizontal"], json!("general"), "the other way is not touched");
}

/// An **indent** above zero makes the text start at the left.
#[test]
fn an_indent_above_zero_makes_the_text_start_at_the_left() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "center" }))
        .expect("the cell is aligned");
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "indent": 2 }))
        .expect("the cell is indented");
    let align = align_of(&session, sheet, "A1").expect("the cell has an alignment");
    assert_eq!(align["indent"], json!(2), "the text is indented twice");
    assert_eq!(align["horizontal"], json!("left"), "and it starts at the left");
}

/// An indent that is **not one of them** is refused.
#[test]
fn an_indent_that_is_not_one_of_them_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for indent in [251.0, -1.0, 0.5] {
        let outcome =
            session.execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "indent": indent }));
        assert!(outcome.is_err(), "the indent {indent} is refused: {outcome:?}");
    }
}

/// The **wrap** is a yes or a no.
#[test]
fn the_wrap_is_a_yes_or_a_no() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "wrap": true }))
        .expect("the cell is aligned");
    let align = align_of(&session, sheet, "A1").expect("the cell has an alignment");
    assert_eq!(align["wrap"], json!(true), "the text wraps");
}

/// A name that is **not an alignment** is refused.
#[test]
fn a_name_that_is_not_an_alignment_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "sideways" }));
    assert!(outcome.is_err(), "there is no alignment of that name: {outcome:?}");
    let outcome =
        session.execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "vertical": "sideways" }));
    assert!(outcome.is_err(), "there is no alignment of that name: {outcome:?}");
}

/// A command that names **nothing** is refused.
#[test]
fn a_command_that_names_nothing_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1" }));
    assert!(outcome.is_err(), "there is nothing to align: {outcome:?}");
}

/// The alignment **stays** when the cell's value changes.
#[test]
fn the_alignment_stays_when_the_cells_value_changes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "right" }))
        .expect("the cell is aligned");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the title" }))
        .expect("the cell is set");
    assert_eq!(
        horizontal_of(&session, sheet, "A1"),
        Some("right".to_string()),
        "the text is still to the right"
    );
}
