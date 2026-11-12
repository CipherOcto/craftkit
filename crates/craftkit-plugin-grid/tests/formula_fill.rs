//! The **filling of a formula**.
//!
//! The tree copies a formula and **shifts** the references of it, and
//! the `$` of an address is what says whether a reference moves.

use craftkit_sdk::serde_json::{Value, json};
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

/// The **formula** of a cell, as the payload holds it.
fn formula_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"].get(cell)?.get("formula")?.as_str().map(|text| text.to_string())
}

/// The **value** of a cell, as the payload holds it.
fn value_of(session: &Session, sheet: u64, cell: &str) -> Option<Value> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].clone())
}

/// Put a formula in a cell.
fn set_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// A fill of a formula **shifts** the references of it.
#[test]
fn a_fill_of_a_formula_shifts_its_references() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A2").as_deref(), Some("=B2"), "the reference moves down");
    assert_eq!(formula_of(&session, sheet, "A3").as_deref(), Some("=B3"), "and down again");
}

/// A copy of a formula holds a **formula**, and not a value.
#[test]
fn a_copy_of_a_formula_holds_a_formula() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A2").as_deref(), Some("=B2"), "the formula is there");
    let node = session.document().node(sheet).expect("the sheet is there");
    assert!(node.data["cells"]["A2"].get("value").is_none(), "and no value is written beside it");
}

/// An **anchored** reference does not move.
#[test]
fn an_anchored_reference_does_not_move() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=$B$1");
    set_formula(&mut session, sheet, "C1", "=B$1");
    set_formula(&mut session, sheet, "E1", "=$B1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "C1", "to": "C1:C3" }))
        .expect("the range is filled");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "E1", "to": "E1:E3" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A2").as_deref(), Some("=$B$1"), "the whole address holds");
    assert_eq!(formula_of(&session, sheet, "A3").as_deref(), Some("=$B$1"), "and so does the next");
    assert_eq!(formula_of(&session, sheet, "C2").as_deref(), Some("=B$1"), "the row holds");
    assert_eq!(formula_of(&session, sheet, "E2").as_deref(), Some("=$B2"), "the column holds");
}

/// A **range** in a formula shifts as a whole.
#[test]
fn a_range_in_a_formula_shifts() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=SUM(B1:C1)");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A2").as_deref(), Some("=SUM(B2:C2)"), "the range moves");
}

/// A reference to **another sheet** keeps its sheet.
#[test]
fn a_reference_to_another_sheet_keeps_its_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet2" }))
        .expect("the second sheet is added");
    set_formula(&mut session, sheet, "A1", "=Sheet2!A1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A2").as_deref(), Some("=Sheet2!A2"), "the sheet holds");
}

/// The **mode** does not matter for a formula: the tree copies the
/// formula and shifts it, whatever the fill is doing with numbers.
#[test]
fn the_mode_does_not_matter_for_a_formula() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2", "mode": "copy" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A2").as_deref(), Some("=B2"), "the reference moves");
}

/// A formula that would **read itself** is refused, and the whole fill
/// with it.
#[test]
fn a_fill_that_makes_a_cell_read_itself_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    set_formula(&mut session, sheet, "B2", "=A2");
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect_err("the fill is refused");
    let message = error.to_string();
    assert!(message.contains("read itself"), "the refusal says why: {message}");
    assert_eq!(formula_of(&session, sheet, "A2"), None, "and nothing was written");
}

/// A reference that would leave the sheet names the error: the tree
/// writes `#REF!` there.
#[test]
fn a_reference_that_would_leave_the_sheet_names_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A3", "=A1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A3", "to": "A1:A3" }))
        .expect("the fill is done");
    assert_eq!(formula_of(&session, sheet, "A2"), Some("=#REF!".to_string()), "the copy names the error");
}

/// The value of a copied formula is **derived**: it is computed from
/// the cells it reads.
#[test]
fn the_value_of_a_copied_formula_is_derived() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 10 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 20 })).expect("set");
    set_formula(&mut session, sheet, "A1", "=B1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the cell has a text");
    assert_eq!(outcome.data["text"].as_str(), Some("20"), "the copy reads the cell beside it");
}

/// A fill of a **range** of formulas shifts every one of them.
#[test]
fn a_fill_of_a_range_of_formulas_shifts_every_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    set_formula(&mut session, sheet, "A2", "=B2");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1:A2", "to": "A1:A4" }))
        .expect("the range is filled");
    assert_eq!(formula_of(&session, sheet, "A3").as_deref(), Some("=B3"), "the first formula moves");
    assert_eq!(formula_of(&session, sheet, "A4").as_deref(), Some("=B4"), "and the second");
}

/// A fill of a formula copies the **format** of the source.
#[test]
fn a_fill_of_a_formula_copies_the_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    let node = session.document().node(sheet).expect("the sheet is there");
    let format = node.data["cells"]["A2"]["format"].as_str();
    assert_eq!(format, Some("0.00"), "the format is copied");
}

/// A **text** that looks like a formula is a text: it is copied, and
/// it is never read as a formula.
#[test]
fn a_text_is_not_a_formula() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "hello" }))
        .expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C1", "value": "=B1" })).expect("set");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "C1", "to": "C1:C2" }))
        .expect("the range is filled");
    assert_eq!(value_of(&session, sheet, "A2"), Some(json!("hello")), "the text is copied");
    assert_eq!(formula_of(&session, sheet, "C2"), None, "and the text is not a formula");
}
