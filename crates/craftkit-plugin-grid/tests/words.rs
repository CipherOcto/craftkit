//! A **name** is a word a formula can carry.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: i64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> craftkit_sdk::serde_json::Value {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// A name of the workbook with a **dot** in it is a word, and a formula
/// carries it.
#[test]
fn a_name_with_a_dot_is_a_word() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", 5);
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "My.Sales", "refers_to": "Sheet1!A1" }),
        )
        .expect("the name is written");
    let answer = computed(&mut session, sheet, "B1", "=SUM(My.Sales)");
    assert_eq!(answer, json!(5.0), "and the formula reads the name");
}

/// A sheet's name with a **dot** in it is a word, and a formula carries
/// it.
#[test]
fn a_sheet_with_a_dot_in_its_name_is_a_word() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Q1.2024" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    value(&mut session, sheet, "A1", 7);
    let answer = computed(&mut session, sheet, "B1", "=Q1.2024!A1");
    assert_eq!(answer, json!(7.0), "and the formula reads the sheet");
}

/// A table's name with a **dot** in it is a word, and a formula carries
/// it.
#[test]
fn a_table_with_a_dot_in_its_name_is_a_word() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "who" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": "amount" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 3 }))
        .expect("the cell is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Orders.Data", "range": "A1:B2" }))
        .expect("the table is made");
    let answer = computed(&mut session, sheet, "D1", "=SUM(Orders.Data[amount])");
    assert_eq!(answer, json!(3.0), "and the formula reads the column of the table");
}

/// A table's name is a **name**, and the names of the workbook obey the
/// same rule.
#[test]
fn a_table_name_is_a_name() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    // A name with a **space** is not a word: a formula writes it before
    // the bracket, and a formula has no space in a word.
    let refused = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "My Orders", "range": "A1:B2" }))
        .expect_err("the name is refused");
    assert!(refused.to_string().contains("not a character"), "the refusal says why: {refused}");
    // A name with a **hyphen** is not a word either: it would read as
    // an operator.
    let refused = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "A-B", "range": "A1:B2" }))
        .expect_err("the name is refused");
    assert!(refused.to_string().contains("not a character"), "the refusal says why: {refused}");
    // A name that begins with a **digit** is not a word.
    let refused = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "1Orders", "range": "A1:B2" }))
        .expect_err("the name is refused");
    assert!(refused.to_string().contains("begins with a letter"), "the refusal says why: {refused}");
    // A name that is too **long** is refused.
    let long = "A".repeat(256);
    let refused = session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": long, "range": "A1:B2" }))
        .expect_err("the name is refused");
    assert!(refused.to_string().contains("at most 255"), "the refusal says why: {refused}");
}

/// A name with a letter that is not an **ASCII** letter is a word, and
/// a formula carries it.
#[test]
fn a_name_with_a_letter_of_another_alphabet_is_a_word() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", 5);
    session
        .execute(
            "grid.name.set",
            &json!({ "workbook": workbook, "name": "Ökonomie", "refers_to": "Sheet1!A1" }),
        )
        .expect("the name is written");
    let answer = computed(&mut session, sheet, "B1", "=SUM(Ökonomie)");
    assert_eq!(answer, json!(5.0), "and the formula reads the name");
}

/// A **sheet** with a letter of another alphabet in its name is a word,
/// and a formula carries it.
#[test]
fn a_sheet_with_a_letter_of_another_alphabet_is_a_word() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Ünidad.2024" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    value(&mut session, sheet, "A1", 7);
    let answer = computed(&mut session, sheet, "B1", "=Ünidad.2024!A1");
    assert_eq!(answer, json!(7.0), "and the formula reads the sheet");
}
