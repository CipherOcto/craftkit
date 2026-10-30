//! The spreadsheet battery: a grid, and a graph of **areas**.
//!
//! Three things are proved here: a grid is not a tree and its cells are
//! in the payload; a precedent is an area and not a cell; and a formula's
//! value is derived and never written down.

use craftkit_sdk::Ecosystem;
#[cfg(feature = "grid")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A workbook with a sheet.
#[cfg(feature = "grid")]
fn a_workbook() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    session
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "grid")]
#[test]
fn the_grid_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("grid.workbook.add").is_some(), "the battery is in");
    assert!(ecosystem.kinds.get("grid.sheet").is_some());
    assert!(ecosystem.evaluators.get("grid.calc").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "grid"))]
#[test]
fn the_grid_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("grid.workbook.add").is_none(), "the battery is out");
}

/// A cell holds a value, and the cell is **in the sheet's payload**.
#[cfg(feature = "grid")]
#[test]
fn a_cell_is_in_the_sheets_payload() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 42 })).expect("set");
    let sheet = session.document().node(2).expect("the sheet is there");
    assert_eq!(sheet.data["cells"]["A1"]["value"], json!(42.0));
    // A cell is not a node: the whole of the tree is the workbook and
    // the sheet, and the cell is in the sheet's payload.
    assert_eq!(session.document().walk().count(), 2, "the workbook and the sheet");
}

/// A formula's value is **derived**: it is computed from the cells it
/// reads, and it is never written down.
#[cfg(feature = "grid")]
#[test]
fn a_formulas_value_is_derived() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 2 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A2", "value": 3 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B1", "formula": "=A1+A2" }))
        .expect("the formula is set");
    let sheet = session.document().node(2).expect("the sheet is there");
    assert!(
        sheet.data["cells"]["B1"].get("value").is_none(),
        "the value is not written down: {}",
        sheet.data["cells"]["B1"]
    );
    let plan = session.evaluate();
    let calc: Vec<_> = plan.items.iter().filter(|item| item.kind == "grid.calc").collect();
    assert_eq!(calc[0].value["cells"]["B1"], json!(5.0), "the value is computed: {plan:?}");
}

/// And it is computed **again** when the cells it reads change.
#[cfg(feature = "grid")]
#[test]
fn the_value_is_computed_again_when_a_cell_changes() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 2 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B1", "formula": "=A1*10" }))
        .expect("the formula is set");
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 5 })).expect("set");
    let plan = session.evaluate();
    let calc: Vec<_> = plan.items.iter().filter(|item| item.kind == "grid.calc").collect();
    assert_eq!(calc[0].value["cells"]["B1"], json!(50.0), "the value followed the cell: {plan:?}");
}

/// A precedent is an **area**: `SUM(A1:B4)` reads sixteen cells and one
/// range.
#[cfg(feature = "grid")]
#[test]
fn a_precedent_is_an_area() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "C1", "formula": "=SUM(A1:B4)" }))
        .expect("the formula is set");
    let outcome = session.execute("grid.precedents", &json!({ "sheet": 2, "cell": "C1" })).expect("read");
    let areas = outcome.data["precedents"].as_array().expect("there are areas");
    assert_eq!(areas.len(), 1, "one range: {outcome:?}");
    assert_eq!(areas[0]["area"], json!("A1:B4"));
    assert_eq!(areas[0]["cells"], json!(8), "four rows of two");
}

/// The dependents of a cell are the cells whose formulas **cover** it.
#[cfg(feature = "grid")]
#[test]
fn the_dependents_are_the_cells_that_cover_it() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "C1", "formula": "=SUM(A1:B4)" }))
        .expect("the formula is set");
    let outcome = session.execute("grid.dependents", &json!({ "sheet": 2, "cell": "B3" })).expect("read");
    let dependents = outcome.data["dependents"].as_array().expect("there are dependents");
    assert_eq!(dependents.len(), 1, "{outcome:?}");
    assert_eq!(dependents[0]["cell"], json!("C1"));
    // A cell the range does not cover has no dependents.
    let outcome = session.execute("grid.dependents", &json!({ "sheet": 2, "cell": "C5" })).expect("read");
    assert_eq!(outcome.data["dependents"].as_array().expect("there are dependents").len(), 0);
}

/// A formula that reads itself is refused **when it is set**.
#[cfg(feature = "grid")]
#[test]
fn a_formula_that_reads_itself_is_refused() {
    let mut session = a_workbook();
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=A1+1" }))
        .expect_err("the formula reads itself");
    assert!(error.to_string().contains("reads itself"), "{error}");
    // And nothing was written.
    let sheet = session.document().node(2).expect("the sheet is there");
    assert!(
        sheet.data["cells"].as_object().map(|cells| cells.is_empty()).unwrap_or(true),
        "{:?}",
        sheet.data["cells"]
    );
}

/// And so is a cycle of two.
#[cfg(feature = "grid")]
#[test]
fn a_cycle_of_two_is_refused() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=B1+1" }))
        .expect("the first formula is set");
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B1", "formula": "=A1+1" }))
        .expect_err("the two read each other");
    assert!(error.to_string().contains("reads itself"), "{error}");
}

/// A formula that does not parse is refused, and it says where.
#[cfg(feature = "grid")]
#[test]
fn a_formula_that_does_not_parse_is_refused() {
    let mut session = a_workbook();
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=A1+" }))
        .expect_err("the formula is broken");
    assert!(!error.to_string().is_empty(), "{error}");
}

/// An address that is not an address is refused.
#[cfg(feature = "grid")]
#[test]
fn an_address_that_is_not_an_address_is_refused() {
    let mut session = a_workbook();
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": 2, "cell": "nonsense", "value": 1 }))
        .expect_err("the address is not an address");
    assert!(error.to_string().contains("is not a cell"), "{error}");
}

/// The functions work: a sum over a range, and an average.
#[cfg(feature = "grid")]
#[test]
fn the_functions_work() {
    let mut session = a_workbook();
    for (cell, value) in [("A1", 1), ("A2", 2), ("A3", 3), ("A4", 4)] {
        session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": cell, "value": value })).expect("set");
    }
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B1", "formula": "=SUM(A1:A4)" }))
        .expect("the formula is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B2", "formula": "=AVERAGE(A1:A4)" }))
        .expect("the formula is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B3", "formula": "=MAX(A1:A4)" }))
        .expect("the formula is set");
    let plan = session.evaluate();
    let calc: Vec<_> = plan.items.iter().filter(|item| item.kind == "grid.calc").collect();
    assert_eq!(calc[0].value["cells"]["B1"], json!(10.0));
    assert_eq!(calc[0].value["cells"]["B2"], json!(2.5));
    assert_eq!(calc[0].value["cells"]["B3"], json!(4.0));
}

/// A division by zero is an **error value**, not a crash and not a
/// number.
#[cfg(feature = "grid")]
#[test]
fn a_division_by_zero_is_an_error_value() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=1/0" }))
        .expect("the formula is set");
    let plan = session.evaluate();
    let calc: Vec<_> = plan.items.iter().filter(|item| item.kind == "grid.calc").collect();
    assert_eq!(calc[0].value["cells"]["A1"]["error"], json!("#DIV/0!"));
}

/// A function that does not exist is an error value too.
#[cfg(feature = "grid")]
#[test]
fn a_function_that_does_not_exist_is_an_error_value() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=NOPE(1)" }))
        .expect("the formula is set");
    let plan = session.evaluate();
    let calc: Vec<_> = plan.items.iter().filter(|item| item.kind == "grid.calc").collect();
    assert!(
        calc[0].value["cells"]["A1"]["error"].as_str().unwrap_or_default().contains("NOPE"),
        "{:?}",
        calc[0].value["cells"]["A1"]
    );
}

/// The undo takes a cell back with it.
#[cfg(feature = "grid")]
#[test]
fn the_undo_takes_a_cell_back() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 1 })).expect("set");
    session.undo().expect("the undo works");
    let sheet = session.document().node(2).expect("the sheet is there");
    assert!(
        sheet.data["cells"].as_object().map(|cells| cells.is_empty()).unwrap_or(true),
        "{:?}",
        sheet.data["cells"]
    );
}

/// A cell that is emptied is gone.
#[cfg(feature = "grid")]
#[test]
fn a_cell_that_is_emptied_is_gone() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 1 })).expect("set");
    session.execute("grid.cell.clear", &json!({ "sheet": 2, "cell": "A1" })).expect("cleared");
    let sheet = session.document().node(2).expect("the sheet is there");
    assert!(
        sheet.data["cells"].as_object().map(|cells| cells.is_empty()).unwrap_or(true),
        "{:?}",
        sheet.data["cells"]
    );
}
