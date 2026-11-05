//! The **border** of a cell.

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

/// The **borders** of a cell, as the answer holds them.
fn borders_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session
        .execute("grid.cell.border.get", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the borders are read");
    outcome.data["borders"].clone()
}

/// A **border** is set on an edge of a cell.
#[test]
fn a_border_is_set_on_an_edge() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    let borders = borders_of(&mut session, sheet, "B2");
    assert_eq!(borders["top"]["style"], Json::String("thin".to_string()), "the style is there");
    assert_eq!(borders["top"]["colour"], Json::String("#000000".to_string()), "and the colour is black");
    assert!(borders.get("left").is_none(), "and the other edges have no border");
}

/// The **colour** of a border is said.
#[test]
fn the_colour_of_a_border_is_said() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "double", "colour": "#CC3344" }),
        )
        .expect("the border is set");
    let borders = borders_of(&mut session, sheet, "B2");
    assert_eq!(
        borders["top"]["colour"],
        Json::String("#cc3344".to_string()),
        "the colour is kept as it is written"
    );
}

/// A colour that is **not** a colour is refused.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin", "colour": "red" }),
        )
        .expect_err("the colour is refused");
    assert!(refused.to_string().contains("#rrggbb"), "the refusal says what a colour is");
}

/// A style that is **not** a border style is refused.
#[test]
fn a_style_that_is_not_a_border_style_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "wobbly" }),
        )
        .expect_err("the style is refused");
    assert!(refused.to_string().contains("thin"), "the refusal says what a style is");
}

/// An edge that is **not** an edge is refused.
#[test]
fn an_edge_that_is_not_an_edge_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "sideways", "style": "thin" }),
        )
        .expect_err("the edge is refused");
    assert!(refused.to_string().contains("bottom"), "the refusal says what an edge is");
}

/// A border of the style **none** takes the border away.
#[test]
fn a_border_of_the_style_none_takes_the_border_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "none" }),
        )
        .expect("the border is taken away");
    let borders = borders_of(&mut session, sheet, "B2");
    assert!(borders.get("top").is_none(), "there is no border on the top");
}

/// A border is **taken away** and the other edges stay.
#[test]
fn a_border_is_taken_away_and_the_others_stay() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for edge in ["top", "left"] {
        session
            .execute(
                "grid.cell.border.set",
                &json!({ "sheet": sheet, "cell": "B2", "edge": edge, "style": "thin" }),
            )
            .expect("the border is set");
    }
    session
        .execute("grid.cell.border.clear", &json!({ "sheet": sheet, "cell": "B2", "edge": "top" }))
        .expect("the border is taken away");
    let borders = borders_of(&mut session, sheet, "B2");
    assert!(borders.get("top").is_none(), "the top has no border");
    assert_eq!(borders["left"]["style"], Json::String("thin".to_string()), "and the left has its border");
}

/// A cell that is **covered by a merge** is refused.
#[test]
fn a_covered_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let refused = session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin" }),
        )
        .expect_err("the covered cell is refused");
    assert!(refused.to_string().contains("merge"), "the refusal says why");
}

/// The **formatting** of a protected sheet that does not allow it is
/// refused.
#[test]
fn a_protected_sheet_refuses_the_formatting() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.sheet.protect",
            &json!({ "sheet": sheet, "format": false, "sort": false, "filter": false }),
        )
        .expect("the sheet is protected");
    let refused = session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin" }),
        )
        .expect_err("the formatting is refused");
    assert!(refused.to_string().contains("formatting"), "the refusal says what is not allowed");
}

/// The border goes with the **format** of the cell.
#[test]
fn the_border_goes_with_the_format() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    session
        .execute("grid.cell.clear.format", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the format is cleared");
    let borders = borders_of(&mut session, sheet, "B2");
    assert!(borders.get("top").is_none(), "the border went with the format");
}

/// The border goes with the **whole cell** when it is cleared.
#[test]
fn the_border_goes_with_the_whole_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "B2", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the cell is cleared");
    let borders = borders_of(&mut session, sheet, "B2");
    assert!(borders.get("top").is_none(), "the border went with the cell");
}
