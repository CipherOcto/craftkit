//! The **link** on a cell.

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

/// The **link** on a cell, as the answer holds it.
fn link_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session
        .execute("grid.cell.link.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the link is read");
    outcome.data["link"].clone()
}

/// A link is **written** on a cell.
#[test]
fn a_link_is_written_on_a_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.link.set",
            &json!({ "sheet": sheet, "cell": "A1", "target": "https://example.org", "tooltip": "the site" }),
        )
        .expect("the link is written");
    let link = link_of(&mut session, sheet, "A1");
    assert_eq!(link["target"], Json::String("https://example.org".to_string()), "the target is there");
    assert_eq!(link["tooltip"], Json::String("the site".to_string()), "and the tooltip");
    assert_eq!(link["inside"], Json::Bool(false), "and it is outside");
}

/// A link that is already there is **edited**, and the tooltip is kept
/// unless a new one is given.
#[test]
fn a_link_that_is_already_there_is_edited() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.link.set",
            &json!({ "sheet": sheet, "cell": "A1", "target": "one", "tooltip": "the first" }),
        )
        .expect("the link is written");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "two" }))
        .expect("the link is edited");
    let link = link_of(&mut session, sheet, "A1");
    assert_eq!(link["target"], Json::String("two".to_string()), "the target is the new target");
    assert_eq!(link["tooltip"], Json::String("the first".to_string()), "and the tooltip is kept");
}

/// A link that goes to a place **in this workbook** is inside, and
/// that is derived.
#[test]
fn a_link_that_goes_to_a_place_in_this_workbook_is_inside() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "Sheet1!B2" }))
        .expect("the link is written");
    let link = link_of(&mut session, sheet, "A1");
    assert_eq!(link["inside"], Json::Bool(true), "it is inside");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A2", "target": "Other!B2" }))
        .expect("the link is written");
    let link = link_of(&mut session, sheet, "A2");
    assert_eq!(link["inside"], Json::Bool(false), "and a sheet that is not there is outside");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A3", "target": "Sheet1!nope" }))
        .expect("the link is written");
    let link = link_of(&mut session, sheet, "A3");
    assert_eq!(link["inside"], Json::Bool(false), "and a place that is not a cell is outside");
}

/// A link **survives** the cell being written to, and goes with the
/// whole cell being cleared.
#[test]
fn a_link_survives_the_cell_being_written_to() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "one" }))
        .expect("the link is written");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    let link = link_of(&mut session, sheet, "A1");
    assert_eq!(link["target"], Json::String("one".to_string()), "the link is still there");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is cleared");
    assert_eq!(link_of(&mut session, sheet, "A1"), Json::Null, "and it goes with the cell");
}

/// A link is **taken away**, and a cell that holds no link is not
/// refused.
#[test]
fn a_link_is_taken_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "one" }))
        .expect("the link is written");
    let outcome = session
        .execute("grid.cell.link.delete", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the link is taken away");
    assert_eq!(outcome.data["was_there"], Json::Bool(true), "there was one");
    assert_eq!(link_of(&mut session, sheet, "A1"), Json::Null, "and there is none");
    let outcome = session
        .execute("grid.cell.link.delete", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("it is not refused");
    assert_eq!(outcome.data["was_there"], Json::Bool(false), "and there was none");
}

/// A link that goes **nowhere** is refused.
#[test]
fn a_link_that_goes_nowhere_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "  " }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("a link goes somewhere"), "it says why: {error}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.link.set", &json!({ "sheet": workbook, "cell": "A1", "target": "one" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("not a sheet"), "it says why: {error}");
    let error = session
        .execute("grid.cell.link.get", &json!({ "sheet": 9999, "cell": "A1" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("there is no sheet"), "and there is none: {error}");
}
