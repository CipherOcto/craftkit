//! The **font** of a named style.

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

/// A style may be **bold**.
#[test]
fn a_style_may_be_bold() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Loud", "format": "general" }))
        .expect("the style is made");
    session
        .execute("grid.style.font", &json!({ "workbook": workbook, "name": "Loud", "bold": true }))
        .expect("the style is bold");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Loud" }))
        .expect("the cell names the style");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["font"]["bold"], json!(true), "the cell is bold");
}

/// A style may be **italic** and **struck through**.
#[test]
fn a_style_may_be_italic_and_struck_through() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Odd", "format": "general" }))
        .expect("the style is made");
    session
        .execute(
            "grid.style.font",
            &json!({ "workbook": workbook, "name": "Odd", "italic": true, "strike": true }),
        )
        .expect("the style is odd");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Odd" }))
        .expect("the cell names the style");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["font"]["italic"], json!(true), "the cell is italic");
    assert_eq!(cell["font"]["strike"], json!(true), "and struck through");
}

/// The **bold** may be taken away by name.
#[test]
fn the_bold_may_be_taken_away_by_name() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Loud", "format": "general" }))
        .expect("the style is made");
    session
        .execute("grid.style.font", &json!({ "workbook": workbook, "name": "Loud", "bold": true }))
        .expect("the style is bold");
    session
        .execute("grid.style.font", &json!({ "workbook": workbook, "name": "Loud", "bold": false }))
        .expect("the bold is taken away");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Loud" }))
        .expect("the cell names the style");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["font"]["bold"], json!(false), "the cell is not bold");
}

/// The **face** alone does not turn the bold away.
#[test]
fn the_face_alone_does_not_turn_the_bold_away() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Loud", "format": "general" }))
        .expect("the style is made");
    session
        .execute("grid.style.font", &json!({ "workbook": workbook, "name": "Loud", "bold": true }))
        .expect("the style is bold");
    session
        .execute("grid.style.font", &json!({ "workbook": workbook, "name": "Loud", "face": "Serif" }))
        .expect("the face is set");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Loud" }))
        .expect("the cell names the style");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["font"]["bold"], json!(true), "the cell is still bold");
    assert_eq!(cell["font"]["name"], json!("Serif"), "and the face is set");
}

/// A style that is **not there** is refused.
#[test]
fn a_style_that_is_not_there_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.style.font", &json!({ "workbook": workbook, "name": "Nothing", "bold": true }));
    assert!(outcome.is_err(), "there is no style called `Nothing`");
}
