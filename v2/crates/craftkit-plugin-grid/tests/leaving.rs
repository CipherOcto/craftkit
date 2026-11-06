//! The **copy** that would leave the sheet.

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

/// A **number** in a cell.
fn set_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the number is set");
}

/// A **formula** in a cell.
fn set_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **formula** a cell holds.
fn formula_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"].get(cell)?.get("formula")?.as_str().map(|text| text.to_string())
}

/// What a cell **holds**.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// A **copy** that would leave the sheet names the error.
#[test]
fn a_copy_that_would_leave_the_sheet_names_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A3", "=A1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A3", "to": "A1:A3" }))
        .expect("the fill is done");
    assert_eq!(formula_of(&session, sheet, "A2"), Some("=#REF!".to_string()), "the copy names the error");
}

/// The **value** of a copy that names the error is the error.
#[test]
fn the_value_of_a_copy_that_names_the_error_is_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A3", "=A1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A3", "to": "A1:A3" }))
        .expect("the fill is done");
    assert_eq!(
        cell_of(&mut session, sheet, "A2")["value"],
        json!({ "error": "#REF!" }),
        "the value is the error the tree writes"
    );
}

/// An **anchored** reference never leaves the sheet.
#[test]
fn an_anchored_reference_never_leaves_the_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_number(&mut session, sheet, "A1", 1.0);
    set_formula(&mut session, sheet, "A3", "=$A$1");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A3", "to": "A2:A3" }))
        .expect("the fill is done");
    assert_eq!(
        formula_of(&session, sheet, "A2"),
        Some("=$A$1".to_string()),
        "the anchored reference is the same cell in every cell"
    );
}

/// A **call** with an argument that leaves names the error.
#[test]
fn a_call_with_an_argument_that_leaves_names_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A3", "=SUM(A1)");
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A3", "to": "A1:A3" }))
        .expect("the fill is done");
    assert_eq!(
        formula_of(&session, sheet, "A2"),
        Some("=SUM(#REF!)".to_string()),
        "the argument names the error"
    );
}

/// A **sorted** formula that leaves the sheet names the error.
#[test]
fn a_sorted_formula_that_leaves_the_sheet_names_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_number(&mut session, sheet, "A2", 3.0);
    set_number(&mut session, sheet, "A3", 2.0);
    set_number(&mut session, sheet, "A4", 1.0);
    set_number(&mut session, sheet, "A5", 0.0);
    set_formula(&mut session, sheet, "B5", "=A1");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A2:B5", "column": "A" }))
        .expect("the range is sorted");
    assert_eq!(formula_of(&session, sheet, "B2"), Some("=#REF!".to_string()), "the formula names the error");
}
