//! The **rule of numbers** of the aggregates: `SUM`, `AVERAGE`, `MIN`,
//! `MAX` and `COUNT`.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// The **error** a formula gives.
fn error_of(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> String {
    let value = computed(session, sheet, cell, formula);
    value["error"].as_str().unwrap_or_default().to_string()
}

/// A **text** that was given directly and names a number **is** a
/// number.
#[test]
fn a_text_that_names_a_number_is_a_number() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "C1", "=SUM(\"5\")");
    assert_eq!(answer, json!(5.0), "the sum of the text is the sum of the number");
    let answer = computed(&mut session, sheet, "C2", "=AVERAGE(\"5\")");
    assert_eq!(answer, json!(5.0), "and the average of it is the number");
    let answer = computed(&mut session, sheet, "C3", "=COUNT(\"5\")");
    assert_eq!(answer, json!(1.0), "and it is a number that is counted");
}

/// A **text** that was given directly and names nothing is `#VALUE!`.
#[test]
fn a_text_that_names_nothing_is_refused() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let error = error_of(&mut session, sheet, "C1", "=SUM(\"abc\")");
    assert_eq!(error, "#VALUE!", "the sum of a text that names nothing is refused");
}

/// A **text** in a **range** is not a number: the cells of a range are
/// read as they are.
#[test]
fn a_text_in_a_range_is_not_a_number() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("5"));
    value(&mut session, sheet, "A2", json!(3));
    let answer = computed(&mut session, sheet, "C1", "=SUM(A1:A2)");
    assert_eq!(answer, json!(3.0), "the text of the range is not a number");
    let answer = computed(&mut session, sheet, "C2", "=COUNT(A1:A2)");
    assert_eq!(answer, json!(1.0), "and it is not counted");
}

/// An **error** in a range is the error of the whole aggregate.
#[test]
fn an_error_in_a_range_is_the_error_of_the_aggregate() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A3", "formula": "=1/0" }))
        .expect("the formula is set");
    let error = error_of(&mut session, sheet, "C1", "=SUM(A1:A3)");
    assert_eq!(error, "#DIV/0!", "the sum of a range with an error is the error");
    let error = error_of(&mut session, sheet, "C2", "=AVERAGE(A1:A3)");
    assert_eq!(error, "#DIV/0!", "and the average of it is the error");
}
