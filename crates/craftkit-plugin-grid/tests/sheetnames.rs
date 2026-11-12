//! The **name** of a sheet: its rules, and the formulas that follow it.

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

/// A **workbook** with a sheet in it, and their ids.
fn a_sheet(session: &mut Session, name: &str) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": name }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **name** of a sheet, as the document holds it.
fn name_of(session: &Session, sheet: u64) -> String {
    session.document().node(sheet).expect("the sheet is there").name.clone()
}

/// The **formula** of a cell, as the document holds it.
fn formula_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data
        .get("cells")
        .and_then(|cells| cells.get(cell))
        .and_then(|one| one.get("formula"))
        .and_then(|formula| formula.as_str())
        .map(|formula| formula.to_string())
}

/// A sheet is **renamed**.
#[test]
fn a_sheet_is_renamed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session, "Sheet1");
    let outcome = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "Ledger" }))
        .expect("the sheet is renamed");
    assert_eq!(outcome.data["old_name"].as_str(), Some("Sheet1"), "the old name is said");
    assert_eq!(outcome.data["name"].as_str(), Some("Ledger"), "and the new one");
    assert_eq!(name_of(&session, sheet), "Ledger", "and the sheet carries it");
}

/// A name that is **blank** is refused.
#[test]
fn a_blank_name_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session, "Sheet1");
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "   " }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("a sheet has a name"), "it says why: {message}");
    assert_eq!(name_of(&session, sheet), "Sheet1", "and nothing was written");
}

/// A name that is **too long** is refused.
#[test]
fn a_name_that_is_too_long_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session, "Sheet1");
    let long = "x".repeat(32);
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": long }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("at most 31 characters"), "it says why: {message}");
}

/// A name that holds a **forbidden character** is refused.
#[test]
fn a_name_with_a_forbidden_character_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session, "Sheet1");
    for bad in [":", "\\", "/", "?", "*", "[", "]"] {
        let error = session
            .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": format!("a{bad}b") }))
            .expect_err("the name is refused");
        let message = error.to_string();
        assert!(message.contains("may not hold"), "it says why: {message}");
    }
}

/// A name that **begins or ends** with an apostrophe is refused.
#[test]
fn a_name_with_an_apostrophe_at_the_end_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session, "Sheet1");
    for bad in ["'Ledger", "Ledger'"] {
        let error = session
            .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": bad }))
            .expect_err("the name is refused");
        let message = error.to_string();
        assert!(message.contains("apostrophe"), "it says why: {message}");
    }
}

/// A name that is **taken** is refused, and the case does not save it.
#[test]
fn a_name_that_is_taken_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "Sheet1");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Ledger" }))
        .expect("the sheet is added");
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "Ledger" }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("already there"), "it says why: {message}");
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "ledger" }))
        .expect_err("the name is refused");
    let message = error.to_string();
    assert!(message.contains("already there"), "and the case does not save it: {message}");
}

/// The **formulas** that name the sheet follow it.
#[test]
fn the_formulas_that_name_the_sheet_follow_it() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "Sheet1");
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Two" }))
        .expect("the sheet is added");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=Sheet1!B2" }))
        .expect("the formula is set");
    session
        .execute(
            "grid.formula.set",
            &json!({ "sheet": sheet, "cell": "A2", "formula": "=SUM(Sheet1!B1:B4)" }),
        )
        .expect("the formula is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A3", "formula": "=B1+C1" }))
        .expect("the formula is set");
    let outcome = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "Ledger" }))
        .expect("the sheet is renamed");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(2), "two formulas followed");
    let formula = formula_of(&session, sheet, "A1").expect("the formula is there");
    assert!(formula.contains("Ledger!B2"), "the reference follows: {formula}");
    let formula = formula_of(&session, sheet, "A2").expect("the formula is there");
    assert!(formula.contains("Ledger!B1:B4"), "and the range follows: {formula}");
    let formula = formula_of(&session, sheet, "A3").expect("the formula is there");
    assert_eq!(formula, "=B1+C1", "and a formula that does not name it is left alone");
}

/// A **name** that stands for a range in the sheet follows it.
#[test]
fn a_name_that_stands_for_a_range_follows_it() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session, "Sheet1");
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1:B4" }),
        )
        .expect("the name is set");
    session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "Ledger" }))
        .expect("the sheet is renamed");
    let node = session.document().node(workbook).expect("the workbook is there");
    let refers_to = node.data["names"]["Sales"].as_str().expect("the name is there");
    assert_eq!(refers_to, "Ledger!A1:B4", "the range names the new sheet");
}

/// A sheet that is **not a sheet** may not be renamed.
#[test]
fn a_sheet_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session, "Sheet1");
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": workbook, "name": "Ledger" }))
        .expect_err("the rename is refused");
    let message = error.to_string();
    assert!(message.contains("not a sheet"), "it says why: {message}");
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": 9999, "name": "Ledger" }))
        .expect_err("the rename is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}

/// The **rules** are the rules of the name: a sheet that is **added**
/// with a bad name is refused too.
#[test]
fn a_sheet_that_is_added_with_a_bad_name_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session, "Sheet1");
    let error = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "a/b" }))
        .expect_err("the sheet is refused");
    let message = error.to_string();
    assert!(message.contains("may not hold"), "it says why: {message}");
    let error = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "sheet1" }))
        .expect_err("the sheet is refused");
    let message = error.to_string();
    assert!(message.contains("already there"), "and a taken name is refused: {message}");
}
