//! A quote **in** a quoted thing is written twice.

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

/// The **value** a formula computes, and the **formula** as it is now.
fn computed(
    session: &mut Session,
    sheet: u64,
    cell: &str,
    formula: &str,
) -> (craftkit_sdk::serde_json::Value, Option<String>) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    (outcome.data["value"].clone(), outcome.data["formula"].as_str().map(|text| text.to_string()))
}

/// A **text** with a quote in it is written with the quote twice, and
/// the tree reads it.
#[test]
fn a_text_with_a_quote_in_it_is_read() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let (value, formula) = computed(&mut session, sheet, "A1", "=\"say \"\"hi\"\"\"");
    assert_eq!(value, json!("say \"hi\""), "and the text holds one quote");
    assert_eq!(
        formula.as_deref(),
        Some("=\"say \"\"hi\"\"\""),
        "and the formula is written back as it was read"
    );
}

/// A sheet's name with a quote in it is written with the quote twice,
/// and the tree reads it.
#[test]
fn a_sheet_with_a_quote_in_its_name_is_read() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Bob's Q1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 9 }))
        .expect("the cell is set");
    let (value, _) = computed(&mut session, sheet, "B1", "='Bob''s Q1'!A1");
    assert_eq!(value, json!(9.0), "and the formula reads the sheet");
}
