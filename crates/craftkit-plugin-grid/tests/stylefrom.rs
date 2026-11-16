//! The **style** made from the look of a cell.

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

/// The **cell** as the answer holds it.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// A style takes the **format**, the **font** and the **alignment** of
/// the cell.
#[test]
fn a_style_takes_the_format_the_font_and_the_alignment_of_the_cell() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.5 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the font is bold");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "right" }))
        .expect("the cell is aligned");
    session
        .execute(
            "grid.style.from",
            &json!({ "workbook": workbook, "name": "Bold", "sheet": sheet, "cell": "A1" }),
        )
        .expect("the style is made");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 1.5 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "B1", "style": "Bold" }))
        .expect("the cell names the style");
    let cell = cell_of(&mut session, sheet, "B1");
    assert_eq!(cell["font"]["bold"], json!(true), "the cell is bold");
    assert_eq!(cell["align"]["horizontal"], json!("right"), "and it is aligned to the right");
    assert_eq!(cell["text"], json!("1.50"), "and the format is the one of the style");
}

/// A style made from a cell that names a **style** takes that style's
/// look.
#[test]
fn a_style_made_from_a_cell_that_names_a_style_takes_that_styles_look() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the font is bold");
    session
        .execute(
            "grid.style.from",
            &json!({ "workbook": workbook, "name": "Bold", "sheet": sheet, "cell": "A1" }),
        )
        .expect("the style is made");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Bold" }))
        .expect("the cell names the style");
    session
        .execute(
            "grid.style.from",
            &json!({ "workbook": workbook, "name": "Bold too", "sheet": sheet, "cell": "A1" }),
        )
        .expect("the second style is made");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "B1", "style": "Bold too" }))
        .expect("the cell names the style");
    let cell = cell_of(&mut session, sheet, "B1");
    assert_eq!(cell["font"]["bold"], json!(true), "the second style is bold too");
}

/// A cell that is **not there** is refused.
#[test]
fn a_cell_that_is_not_there_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session.execute(
        "grid.style.from",
        &json!({ "workbook": workbook, "name": "Bold", "sheet": sheet, "cell": "Z9" }),
    );
    assert!(outcome.is_err(), "there is no cell to take a look from");
}

/// A style that is **already there** is refused.
#[test]
fn a_style_that_is_already_there_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute(
            "grid.style.from",
            &json!({ "workbook": workbook, "name": "Bold", "sheet": sheet, "cell": "A1" }),
        )
        .expect("the style is made");
    let outcome = session.execute(
        "grid.style.from",
        &json!({ "workbook": workbook, "name": "Bold", "sheet": sheet, "cell": "A1" }),
    );
    assert!(outcome.is_err(), "the style is already in the workbook");
}
