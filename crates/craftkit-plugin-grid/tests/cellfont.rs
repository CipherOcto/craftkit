//! The **font** of a cell: the face of its text, the size of it, and
//! the marks that are put on it.
//!
//! The tree's rules: the bold, the italic and the strike are buttons,
//! and a button pressed twice does the opposite; the underline and the
//! place of the text are taken away when they are named again; and the
//! size is rounded to the nearest half.

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

/// The **font** of a cell, as the sheet keeps it.
fn font_of(session: &Session, sheet: u64, cell: &str) -> Option<craftkit_sdk::serde_json::Value> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let font = node.data["cells"][cell]["font"].clone();
    if font.is_null() { None } else { Some(font) }
}

/// The text is **bold**, and the button pressed twice takes it away.
#[test]
fn the_text_is_bold_and_the_button_pressed_twice_takes_it_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is bold");
    assert_eq!(outcome.data["bold"], json!(true), "the text is bold");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["bold"],
        json!(true),
        "and the sheet keeps it"
    );
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is bold again");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["bold"],
        json!(false),
        "and the bold is away"
    );
}

/// The bold may be **named** as well.
#[test]
fn the_bold_may_be_named_as_well() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the cell is bold");
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the cell is bold again");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["bold"],
        json!(true),
        "the bold stays"
    );
}

/// The text is **italic**, and the text is **struck through**.
#[test]
fn the_text_is_italic_and_the_text_is_struck_through() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font.italic", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is italic");
    session
        .execute("grid.cell.font.strike", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is struck through");
    let font = font_of(&session, sheet, "A1").expect("the cell has a font");
    assert_eq!(font["italic"], json!(true), "the text is italic");
    assert_eq!(font["strike"], json!(true), "and struck through");
}

/// The **underline** is taken away when it is named again.
#[test]
fn the_underline_is_taken_away_when_it_is_named_again() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "underline": "double" }))
        .expect("the cell is underlined");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["underline"],
        json!("double"),
        "the text has two lines under it"
    );
    session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "underline": "double" }))
        .expect("the cell is underlined again");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["underline"],
        json!("none"),
        "and the underline is away"
    );
}

/// The **place** of the text goes back to the baseline when it is named
/// again.
#[test]
fn the_place_of_the_text_goes_back_to_the_baseline() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "vert": "superscript" }))
        .expect("the text is above the line");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["vert"],
        json!("superscript"),
        "the text is above the line"
    );
    session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "vert": "superscript" }))
        .expect("the text is above the line again");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["vert"],
        json!("baseline"),
        "and back on the line"
    );
}

/// The **face** of the text is named.
#[test]
fn the_face_of_the_text_is_named() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "name": "Iowan" }))
        .expect("the cell has a face");
    assert_eq!(outcome.data["name"], json!("Iowan"), "the face is named");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["name"],
        json!("Iowan"),
        "and the sheet keeps it"
    );
}

/// The **size** of the text is rounded to the nearest half.
#[test]
fn the_size_of_the_text_is_rounded_to_the_nearest_half() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "size": 11.3 }))
        .expect("the cell has a size");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["size"],
        json!(11.5),
        "the size is rounded to the nearest half"
    );
}

/// A size that is **not one of them** is refused.
#[test]
fn a_size_that_is_not_one_of_them_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for size in [0.0, 500.0, -3.0] {
        let outcome =
            session.execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "size": size }));
        assert!(outcome.is_err(), "the size {size} is refused: {outcome:?}");
    }
}

/// A name that is **not an underline** is refused, and so is a name
/// that is not a place.
#[test]
fn a_name_that_is_not_an_underline_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "underline": "wavy" }));
    assert!(outcome.is_err(), "there is no underline of that name: {outcome:?}");
    let outcome =
        session.execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "vert": "sideways" }));
    assert!(outcome.is_err(), "there is no place of that name: {outcome:?}");
}

/// The font **stays** when the cell's value changes.
#[test]
fn the_font_stays_when_the_cells_value_changes() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is bold");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the title" }))
        .expect("the cell is set");
    assert_eq!(
        font_of(&session, sheet, "A1").expect("the cell has a font")["bold"],
        json!(true),
        "the text is still bold"
    );
}
