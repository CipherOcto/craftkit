//! The **words of an amount** in Thai.

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

/// The **text** a formula computes.
fn text_of(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The amount in **Thai words**.
#[test]
fn the_amount_is_written_in_thai_words() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        text_of(&mut session, sheet, "A1", "=BAHTTEXT(1234)"),
        json!("หนึ่งพันสองร้อยสามสิบสี่บาทถ้วน"),
        "the tree's own example"
    );
    assert_eq!(text_of(&mut session, sheet, "A2", "=BAHTTEXT(0)"), json!("ศูนย์บาทถ้วน"), "and nothing");
    assert_eq!(
        text_of(&mut session, sheet, "A3", "=BAHTTEXT(21.25)"),
        json!("ยี่สิบเอ็ดบาทยี่สิบห้าสตางค์"),
        "and the satang"
    );
    assert_eq!(
        text_of(&mut session, sheet, "A4", "=BAHTTEXT(1000000)"),
        json!("หนึ่งล้านบาทถ้วน"),
        "and a million"
    );
    assert_eq!(
        text_of(&mut session, sheet, "A5", "=BAHTTEXT(1000001)"),
        json!("หนึ่งล้านเอ็ดบาทถ้วน"),
        "and a million and one"
    );
    assert_eq!(
        text_of(&mut session, sheet, "A6", "=BAHTTEXT(-15.5)"),
        json!("ลบสิบห้าบาทห้าสิบสตางค์"),
        "and a negative amount"
    );
    assert_eq!(
        text_of(&mut session, sheet, "A7", "=BAHTTEXT(1.999)"),
        json!("สองบาทถ้วน"),
        "and the rounding to the satang"
    );
    assert_eq!(
        text_of(&mut session, sheet, "A8", "=BAHTTEXT(\"abc\")"),
        json!({ "error": "#VALUE!" }),
        "and a text that holds no number is an error"
    );
}
