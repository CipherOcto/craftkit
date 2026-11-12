//! The **ordinals** of a fill.
//!
//! The tree's rule: a text with a number in it is filled as a text with
//! a number in it, and when the number is written with an English
//! ordinal ending the ending is rewritten to suit the new number —
//! `1st Period` is filled as `2nd Period`.

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

/// Put a **text** in a cell.
fn put(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// The **text** of a cell, as the payload holds it.
fn text_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].as_str()?.to_string())
}

/// Fill a range from a cell, and say what the cells hold.
fn filled(session: &mut Session, sheet: u64, from: &str, to: &str) -> Vec<String> {
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": from, "to": to }))
        .expect("the range is filled");
    let cells = ["A1", "A2", "A3", "A4"];
    cells.iter().filter_map(|cell| text_of(session, sheet, cell)).collect()
}

/// The ordinal ending is rewritten to suit the number.
#[test]
fn the_ordinal_ending_is_rewritten() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "1st Period");
    assert_eq!(
        filled(&mut session, sheet, "A1", "A1:A4"),
        vec!["1st Period", "2nd Period", "3rd Period", "4th Period"],
        "the endings are rewritten"
    );
}

/// The eleventh, the twelfth and the thirteenth end in **th**.
#[test]
fn the_eleventh_twelfth_and_thirteenth_end_in_th() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "11th Period");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A4" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A1").as_deref(), Some("11th Period"), "the eleventh");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("12th Period"), "the twelfth");
    assert_eq!(text_of(&session, sheet, "A3").as_deref(), Some("13th Period"), "the thirteenth");
    assert_eq!(text_of(&session, sheet, "A4").as_deref(), Some("14th Period"), "the fourteenth");
}

/// The ending keeps the **case** it was written in.
#[test]
fn the_ending_keeps_its_case() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "1ST Period");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("2ND Period"), "the ending is in capitals");
}

/// A text after the ending that starts a **word** is not an ordinal.
#[test]
fn a_text_that_starts_a_word_is_not_an_ordinal() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "1stx");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("2stx"), "the ending is kept as it is");
}

/// The number of a text with a number in it is the **last** run of
/// digits: `1st2` is the text `1st` and the number `2`.
#[test]
fn the_number_is_the_last_run_of_digits() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "1st2");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("1st3"), "the two counts on");
}

/// A text with **no** ordinal ending is filled as it is.
#[test]
fn a_text_with_no_ordinal_is_filled_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "Item 1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A3" }))
        .expect("the range is filled");
    assert_eq!(text_of(&session, sheet, "A2").as_deref(), Some("Item 2"), "the number counts on");
    assert_eq!(text_of(&session, sheet, "A3").as_deref(), Some("Item 3"), "and on again");
}

/// The **number** is still a number: the ending does not swallow it.
#[test]
fn the_number_is_still_a_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "1st Period");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:A2" }))
        .expect("the range is filled");
    let node = session.document().node(sheet).expect("the sheet is there");
    let value: &Value = &node.data["cells"]["A2"]["value"];
    assert!(value.is_string(), "the cell holds a text");
    assert_eq!(value.as_str(), Some("2nd Period"), "and the text is the whole of it");
}
