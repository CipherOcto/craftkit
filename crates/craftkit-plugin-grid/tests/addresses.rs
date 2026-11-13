//! The **address** of a cell, and the **link** to it.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **address** of a cell is written as text, with the tree's
/// anchoring.
#[test]
fn the_address_of_a_cell_is_written_as_text() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=ADDRESS(1,1)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!("$A$1"), "the first cell");
    formula(&mut session, sheet, "A2", "=ADDRESS(2,3,2)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!("C$2"), "the row is anchored");
    formula(&mut session, sheet, "A3", "=ADDRESS(2,3,3)");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!("$C2"), "the column is anchored");
    formula(&mut session, sheet, "A4", "=ADDRESS(2,28,4)");
    assert_eq!(value_of(&mut session, sheet, "A4"), json!("AB2"), "neither is anchored");
}

/// The address may be written in the **`R1C1`** style.
#[test]
fn the_address_may_be_written_in_the_r1c1_style() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=ADDRESS(2,3,1,FALSE)");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!("R2C3"), "both are anchored");
    formula(&mut session, sheet, "A2", "=ADDRESS(2,3,4,FALSE)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!("R[2]C[3]"), "and neither is");
}

/// The address may name a **sheet**, and a sheet whose name needs
/// quotes is quoted.
#[test]
fn the_address_may_name_a_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=ADDRESS(1,1,1,TRUE,\"Sheet2\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!("Sheet2!$A$1"), "a plain name");
    formula(&mut session, sheet, "A2", "=ADDRESS(1,1,1,TRUE,\"My Sheet\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!("'My Sheet'!$A$1"), "a name that needs quotes");
    formula(&mut session, sheet, "A3", "=ADDRESS(1,1,1,TRUE,\"\")");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!("!$A$1"), "and no name at all");
}

/// A row or a column that is **not there** is an error, and not a
/// silent zero.
#[test]
fn a_row_or_a_column_that_is_not_there_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=ADDRESS(0,1)");
    assert_eq!(value_of(&mut session, sheet, "A1")["error"], Json::String("#VALUE!".to_string()), "no row");
    formula(&mut session, sheet, "A2", "=ADDRESS(1,16385)");
    assert_eq!(
        value_of(&mut session, sheet, "A2")["error"],
        Json::String("#VALUE!".to_string()),
        "no column"
    );
    formula(&mut session, sheet, "A3", "=ADDRESS(1,1,5)");
    assert_eq!(
        value_of(&mut session, sheet, "A3")["error"],
        Json::String("#VALUE!".to_string()),
        "no anchoring"
    );
}

/// The **friendly name** of a link is what the cell shows, and the link
/// itself is shown when there is no name.
#[test]
fn the_friendly_name_of_a_link_is_what_the_cell_shows() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=HYPERLINK(\"https://example.invalid/\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!("https://example.invalid/"), "the link itself");
    formula(&mut session, sheet, "A2", "=HYPERLINK(\"https://example.invalid/\",\"Click\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!("Click"), "the friendly name");
}

/// **Any** value is written as text, and the **strict** form writes a
/// text in quotes.
#[test]
fn any_value_is_written_as_text() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=VALUETOTEXT(\"ab\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!("ab"), "a text is the text");
    formula(&mut session, sheet, "A2", "=VALUETOTEXT(\"ab\",1)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!("\"ab\""), "and the strict form quotes it");
    formula(&mut session, sheet, "A3", "=VALUETOTEXT(1.5)");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!("1.5"), "a number is the number");
    formula(&mut session, sheet, "A4", "=VALUETOTEXT(TRUE)");
    assert_eq!(value_of(&mut session, sheet, "A4"), json!("TRUE"), "a boolean is its name");
}

/// The **full-width** characters are written as **half-width** ones.
#[test]
fn the_full_width_characters_are_written_as_half_width_ones() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=ASC(\"ＡＢＣ\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!("ABC"), "the letters");
    formula(&mut session, sheet, "A2", "=ASC(\"１２３\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!("123"), "the digits");
    formula(&mut session, sheet, "A3", "=ASC(\"abc\")");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!("abc"), "and a half-width text is itself");
}
