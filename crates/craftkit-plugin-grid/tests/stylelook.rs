//! The **look** of a style: the alignment and the font it carries.
//!
//! The tree's style carries a font, a fill, an alignment and a
//! protection, and the cells that name it are written with them. Here
//! a style carries the alignment and the font, and a cell that says
//! nothing of its own takes them.

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

/// A **style** in the workbook.
fn a_style(session: &mut Session, workbook: u64, name: &str) {
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": name, "format": "general" }))
        .expect("the style is added");
}

/// The **cell** as it is read.
fn cell(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// The style's **alignment** is the alignment of the cells that name
/// it and say nothing of their own.
#[test]
fn the_alignment_of_a_style_is_the_alignment_of_its_cells() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "head");
    session
        .execute(
            "grid.style.align",
            &json!({ "workbook": workbook, "name": "head", "horizontal": "center", "wrap": true }),
        )
        .expect("the style is aligned");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "head" }))
        .expect("the cell names the style");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["align"]["horizontal"].as_str(), Some("center"), "the alignment is the style's");
    assert_eq!(read["align"]["wrap"].as_bool(), Some(true), "and the wrap");
}

/// A cell that says of itself how it is aligned says **more** than the
/// style it names.
#[test]
fn the_cell_says_more_than_the_style() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "head");
    session
        .execute("grid.style.align", &json!({ "workbook": workbook, "name": "head", "horizontal": "center" }))
        .expect("the style is aligned");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "head" }))
        .expect("the cell names the style");
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "right" }))
        .expect("the cell is aligned");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["align"]["horizontal"].as_str(), Some("right"), "the cell says so");
}

/// The style's **font** is the font of the cells that name it and say
/// nothing of their own.
#[test]
fn the_font_of_a_style_is_the_font_of_its_cells() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "head");
    session
        .execute(
            "grid.style.font",
            &json!({ "workbook": workbook, "name": "head", "face": "Iowan", "size": 14.0 }),
        )
        .expect("the style has a font");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "head" }))
        .expect("the cell names the style");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["font"]["name"].as_str(), Some("Iowan"), "the face is the style's");
    assert_eq!(read["font"]["size"].as_f64(), Some(14.0), "and the size");
}

/// The look of a style is **derived**: changing the style changes the
/// cells that name it.
#[test]
fn the_look_of_a_style_is_derived() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "head");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "head" }))
        .expect("the cell names the style");
    session
        .execute("grid.style.align", &json!({ "workbook": workbook, "name": "head", "horizontal": "center" }))
        .expect("the style is aligned");
    assert_eq!(
        cell(&mut session, sheet, "A1")["align"]["horizontal"].as_str(),
        Some("center"),
        "it is aligned"
    );
    session
        .execute("grid.style.align", &json!({ "workbook": workbook, "name": "head", "horizontal": "right" }))
        .expect("the style is aligned");
    assert_eq!(cell(&mut session, sheet, "A1")["align"]["horizontal"].as_str(), Some("right"), "and again");
}

/// The **format** of a style does not take its **look** away.
#[test]
fn the_format_of_a_style_does_not_take_its_look_away() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "head");
    session
        .execute("grid.style.align", &json!({ "workbook": workbook, "name": "head", "horizontal": "center" }))
        .expect("the style is aligned");
    session
        .execute("grid.style.font", &json!({ "workbook": workbook, "name": "head", "face": "Iowan" }))
        .expect("the style has a font");
    session
        .execute("grid.style.set", &json!({ "workbook": workbook, "name": "head", "format": "0.00" }))
        .expect("the format is set");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "head" }))
        .expect("the cell names the style");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["align"]["horizontal"].as_str(), Some("center"), "the alignment is there");
    assert_eq!(read["font"]["name"].as_str(), Some("Iowan"), "and the font");
}

/// A style that is **not there** may not be aligned.
#[test]
fn a_style_that_is_not_there_may_not_be_aligned() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.style.align",
            &json!({ "workbook": workbook, "name": "nothing", "horizontal": "center" }),
        )
        .expect_err("the style is refused");
    assert!(error.to_string().contains("there is no style `nothing`"), "the refusal says so: {error}");
}
