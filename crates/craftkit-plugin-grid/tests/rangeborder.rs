//! The **border** of a range of cells.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// The **borders** of a cell, as the answer holds them.
fn borders_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session
        .execute("grid.cell.border.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the borders are read");
    outcome.data["borders"].clone()
}

/// Every cell of a range is given **every edge** it has.
#[test]
fn every_cell_of_a_range_is_given_every_edge() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1:B2", "preset": "all" }))
        .expect("the border is written");
    for cell in ["A1", "B1", "A2", "B2"] {
        let borders = borders_of(&mut session, sheet, cell);
        for edge in ["top", "right", "bottom", "left"] {
            assert_eq!(borders[edge]["style"], json!("thin"), "the {cell} has a {edge} border");
        }
    }
}

/// The cells on the **rim** of the range get the edges that face out.
#[test]
fn the_cells_on_the_rim_get_the_edges_that_face_out() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1:B2", "preset": "outside" }))
        .expect("the border is written");
    let corner = borders_of(&mut session, sheet, "A1");
    assert_eq!(corner["top"]["style"], json!("thin"), "the corner has a top border");
    assert_eq!(corner["left"]["style"], json!("thin"), "and a left one");
    let other = borders_of(&mut session, sheet, "B2");
    assert_eq!(other["bottom"]["style"], json!("thin"), "the other corner has a bottom border");
    assert_eq!(other["right"]["style"], json!("thin"), "and a right one");
}

/// The cells that are **not on the rim** get the edges that face in.
#[test]
fn the_cells_that_are_not_on_the_rim_get_the_edges_that_face_in() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1:C3", "preset": "inside" }))
        .expect("the border is written");
    let middle = borders_of(&mut session, sheet, "B2");
    assert_eq!(middle["top"]["style"], json!("thin"), "the middle has a top border");
    assert_eq!(middle["bottom"]["style"], json!("thin"), "and a bottom one");
    let corner = borders_of(&mut session, sheet, "A1");
    assert_eq!(corner["top"], Json::Null, "the corner has no top border");
    assert_eq!(corner["left"], Json::Null, "and no left one");
}

/// One edge may be given to the cells that are **on it**.
#[test]
fn one_edge_may_be_given_to_the_cells_that_are_on_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1:B2", "preset": "top" }))
        .expect("the border is written");
    let top = borders_of(&mut session, sheet, "A1");
    assert_eq!(top["top"]["style"], json!("thin"), "the cell on the top has a top border");
    let below = borders_of(&mut session, sheet, "A2");
    assert_eq!(below["top"], Json::Null, "and the cell below it has not");
}

/// A border of the style **`none`** takes the border away.
#[test]
fn a_border_of_the_style_none_takes_the_border_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1:B2", "preset": "all" }))
        .expect("the border is written");
    session
        .execute(
            "grid.range.border",
            &json!({ "sheet": sheet, "range": "A1:B2", "preset": "all", "style": "none" }),
        )
        .expect("the border is taken away");
    let borders = borders_of(&mut session, sheet, "A1");
    assert_eq!(borders["top"], Json::Null, "the cell has no top border");
    assert_eq!(borders["left"], Json::Null, "and no left one");
}

/// The **colour** of the line is the colour that is given.
#[test]
fn the_colour_of_the_line_is_the_colour_that_is_given() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.range.border",
            &json!({ "sheet": sheet, "range": "A1", "preset": "all", "colour": "#ff0000" }),
        )
        .expect("the border is written");
    let borders = borders_of(&mut session, sheet, "A1");
    assert_eq!(borders["top"]["colour"], json!("#ff0000"), "the line is red");
}

/// A **preset** that is not one of the tree's is refused.
#[test]
fn a_preset_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1", "preset": "middle" }));
    assert!(outcome.is_err(), "a preset is one of the tree's");
}

/// A cell that is **covered by a merge** is refused.
#[test]
fn a_cell_that_is_covered_by_a_merge_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the cells are merged");
    let outcome =
        session.execute("grid.range.border", &json!({ "sheet": sheet, "range": "A1:B1", "preset": "all" }));
    assert!(outcome.is_err(), "the border would be drawn under another cell");
}
