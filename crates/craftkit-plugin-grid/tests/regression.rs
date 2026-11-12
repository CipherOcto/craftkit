//! The **line** through a set of numbers.
//!
//! A formula may ask how two sets of numbers move together.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell and read what it computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    value_of(session, sheet, cell)
}

/// Put **numbers** in a column of cells.
fn column(session: &mut Session, sheet: u64, letter: &str, numbers: &[f64]) {
    for (index, number) in numbers.iter().enumerate() {
        let cell = format!("{letter}{}", index + 1);
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": number }))
            .expect("the cell is set");
    }
}

/// How two sets of numbers **move together**.
#[test]
fn the_correlation_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[3.0, 2.0, 4.0, 5.0, 6.0]);
    column(&mut session, sheet, "B", &[9.0, 7.0, 12.0, 15.0, 17.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(CORREL(A1:A5,B1:B5),9)"),
        json!(0.997054486),
        "the correlation"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(PEARSON(A1:A5,B1:B5),9)"),
        json!(0.997054486),
        "and by its other name"
    );
}

/// The **line** through the numbers.
#[test]
fn the_line_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[2.0, 3.0, 9.0, 1.0, 8.0, 7.0, 5.0]);
    column(&mut session, sheet, "B", &[6.0, 5.0, 11.0, 7.0, 5.0, 4.0, 4.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(SLOPE(A1:A7,B1:B7),8)"),
        json!(0.30555556),
        "the slope"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(RSQ(A1:A7,B1:B7),8)"),
        json!(0.05795019),
        "the square of the correlation"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=ROUND(INTERCEPT(A1:A5,B1:B5),8)"),
        json!(0.0483871),
        "and where it crosses the axis"
    );
}

/// The **error** of the line and its **prediction**.
#[test]
fn the_error_and_the_prediction_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[2.0, 3.0, 9.0, 1.0, 8.0, 7.0, 5.0]);
    column(&mut session, sheet, "B", &[6.0, 5.0, 11.0, 7.0, 5.0, 4.0, 4.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(STEYX(A1:A7,B1:B7),8)"),
        json!(3.30571895),
        "the standard error"
    );
    column(&mut session, sheet, "F", &[6.0, 7.0, 9.0, 15.0, 21.0]);
    column(&mut session, sheet, "G", &[20.0, 28.0, 31.0, 38.0, 40.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(FORECAST(30,F1:F5,G1:G5),7)"),
        json!(10.6072531),
        "and the prediction"
    );
}

/// How the two sets move together: the **covariance**.
#[test]
fn the_covariance_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[3.0, 2.0, 4.0, 5.0, 6.0]);
    column(&mut session, sheet, "B", &[9.0, 7.0, 12.0, 15.0, 17.0]);
    assert_eq!(computed(&mut session, sheet, "D1", "=COVARIANCE.P(A1:A5,B1:B5)"), json!(5.2), "of the whole");
    assert_eq!(
        computed(&mut session, sheet, "D2", "=COVAR(A1:A5,B1:B5)"),
        json!(5.2),
        "and by its other name"
    );
    column(&mut session, sheet, "F", &[2.0, 4.0, 8.0]);
    column(&mut session, sheet, "G", &[5.0, 11.0, 12.0]);
    assert_eq!(
        computed(&mut session, sheet, "D3", "=ROUND(COVARIANCE.S(F1:F3,G1:G3),9)"),
        json!(9.666666667),
        "and of a sample"
    );
}

/// A pair that is **not two numbers** is skipped.
#[test]
fn a_pair_that_is_not_two_numbers_is_skipped() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[2.0, 100.0, 6.0]);
    column(&mut session, sheet, "B", &[1.0, 3.0]);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": "x" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": 3.0 }))
        .expect("the cell is set");
    assert_eq!(
        computed(&mut session, sheet, "D1", "=SLOPE(A1:A3,B1:B3)"),
        json!(2.0),
        "and the line is of the pairs that are numbers"
    );
}

/// The tree's **errors**: two sets of different lengths, and a line
/// that does not rise.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[1.0, 2.0]);
    column(&mut session, sheet, "B", &[1.0, 2.0, 3.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=CORREL(A1:A2,B1:B3)"),
        json!({ "error": "#N/A" }),
        "two sets of different lengths are missing"
    );
    column(&mut session, sheet, "F", &[1.0, 1.0]);
    column(&mut session, sheet, "G", &[1.0, 2.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=CORREL(F1:F2,G1:G2)"),
        json!({ "error": "#DIV/0!" }),
        "and a line that does not rise is a division by nothing"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=STEYX(F1:F2,G1:G2)"),
        json!({ "error": "#DIV/0!" }),
        "and the error of a line wants three pairs"
    );
}
