//! The **data table** of a sheet: the table of what the sheet would
//! say for a list of inputs.
//!
//! The tree's own: the range holds the inputs and the formulas, and
//! the results are written into the body of the range — as values, and
//! not as a live formula.

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

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **formula** in a cell.
fn a_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **cell** of a sheet, as the document carries it.
fn cell_of(session: &Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell].clone()
}

/// The **column** of inputs: the values down the first column and the
/// formulas in the first row.
#[test]
fn the_column_of_inputs_is_tried_down_the_table() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "B1", 2.0);
    a_formula(&mut session, sheet, "E2", "=B1*10");
    a_number(&mut session, sheet, "D3", 1.0);
    a_number(&mut session, sheet, "D4", 2.0);
    let outcome = session
        .execute("grid.data.table", &json!({ "sheet": sheet, "range": "D2:E4", "column_input": "B1" }))
        .expect("the table is written");
    assert_eq!(outcome.data["mode"], json!("column"), "it is a column of inputs");
    assert_eq!(outcome.data["cells"], json!(2), "two cells are written");
    assert_eq!(cell_of(&session, sheet, "E3")["value"], json!(10.0), "the first result is there");
    assert_eq!(cell_of(&session, sheet, "E4")["value"], json!(20.0), "and the second one");
}

/// The **row** of inputs: the values across the first row and the
/// formulas down the first column.
#[test]
fn the_row_of_inputs_is_tried_across_the_table() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "B1", 3.0);
    // The formulas are down the first column of the **body**: the tree's
    // own layout.
    a_formula(&mut session, sheet, "D3", "=B1*10");
    a_number(&mut session, sheet, "E2", 1.0);
    a_number(&mut session, sheet, "F2", 2.0);
    let outcome = session
        .execute("grid.data.table", &json!({ "sheet": sheet, "range": "D2:F3", "row_input": "B1" }))
        .expect("the table is written");
    assert_eq!(outcome.data["mode"], json!("row"), "it is a row of inputs");
    assert_eq!(cell_of(&session, sheet, "E3")["value"], json!(10.0), "the first result is there");
    assert_eq!(cell_of(&session, sheet, "F3")["value"], json!(20.0), "and the second one");
}

/// The table of **two variables**: the formula is in the corner.
#[test]
fn the_table_of_two_variables_is_written() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    // The two **input cells**: the row of inputs goes into the row
    // input and the column of inputs into the column input.
    a_number(&mut session, sheet, "B1", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    // The **corner** holds the formula, and it reads a cell that is
    // not an input.
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "D1", "=A1+B1+B2");
    a_number(&mut session, sheet, "D2", 10.0);
    a_number(&mut session, sheet, "D3", 20.0);
    a_number(&mut session, sheet, "E1", 2.0);
    a_number(&mut session, sheet, "F1", 3.0);
    let outcome = session
        .execute(
            "grid.data.table",
            &json!({ "sheet": sheet, "range": "D1:F3", "row_input": "B1", "column_input": "B2" }),
        )
        .expect("the table is written");
    assert_eq!(outcome.data["mode"], json!("two-variable"), "it is a table of two variables");
    assert_eq!(cell_of(&session, sheet, "E2")["value"], json!(13.0), "the first result is there");
    assert_eq!(cell_of(&session, sheet, "F2")["value"], json!(14.0), "and the second one");
    assert_eq!(cell_of(&session, sheet, "E3")["value"], json!(23.0), "and the third one");
}

/// The results are **static**: they are values, and not formulas.
#[test]
fn the_results_are_static() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "B1", 2.0);
    a_formula(&mut session, sheet, "E2", "=B1*10");
    a_number(&mut session, sheet, "D3", 3.0);
    session
        .execute("grid.data.table", &json!({ "sheet": sheet, "range": "D2:E3", "column_input": "B1" }))
        .expect("the table is written");
    let written = cell_of(&session, sheet, "E3");
    assert_eq!(written["value"], json!(30.0), "the result is there");
    assert!(written.get("formula").is_none(), "and it is not a formula");
}

/// A table with **no input** is refused.
#[test]
fn a_table_with_no_input_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.data.table", &json!({ "sheet": sheet, "range": "D2:E4" }))
        .expect_err("the table is refused");
    assert!(error.to_string().contains("give the row input"), "the refusal says so: {error}");
}

/// An **input cell inside the table** is refused.
#[test]
fn an_input_cell_inside_the_table_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.data.table", &json!({ "sheet": sheet, "range": "D2:E4", "column_input": "D3" }))
        .expect_err("the table is refused");
    assert!(error.to_string().contains("is inside the table"), "the refusal says so: {error}");
}
