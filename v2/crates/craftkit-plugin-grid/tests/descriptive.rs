//! The **shape** of a set of numbers, further.
//!
//! A formula may ask how a set of numbers is spread out: its
//! deviations, its means, its skew, its ranks and its percentiles.

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

/// The **deviations** of a set and its **means**.
#[test]
fn the_deviations_and_the_means_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[4.0, 5.0, 8.0, 7.0, 11.0, 4.0, 3.0]);
    assert_eq!(computed(&mut session, sheet, "D1", "=DEVSQ(A1:A7)"), json!(48.0), "the sum of the squares");
    column(&mut session, sheet, "B", &[4.0, 5.0, 6.0, 7.0, 5.0, 4.0, 3.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(AVEDEV(B1:B7),9)"),
        json!(1.020408163),
        "the average deviation"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=ROUND(GEOMEAN(A1:A7),9)"),
        json!(5.47698697),
        "the geometric mean"
    );
    assert_eq!(
        computed(&mut session, sheet, "D4", "=ROUND(HARMEAN(A1:A7),9)"),
        json!(5.028375962),
        "and the harmonic one"
    );
}

/// The **trimmed** mean and the **skew** of a set.
#[test]
fn the_trimmed_mean_and_the_skew_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[4.0, 5.0, 6.0, 7.0, 2.0, 3.0, 4.0, 5.0, 1.0, 2.0, 3.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=ROUND(TRIMMEAN(A1:A11,0.2),9)"),
        json!(3.777777778),
        "the trimmed mean"
    );
    column(&mut session, sheet, "B", &[3.0, 4.0, 5.0, 2.0, 3.0, 4.0, 5.0, 6.0, 4.0, 7.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=ROUND(KURT(B1:B10),8)"),
        json!(-0.15179964),
        "the kurtosis"
    );
    assert_eq!(computed(&mut session, sheet, "D3", "=ROUND(SKEW(B1:B10),8)"), json!(0.35954307), "the skew");
    assert_eq!(
        computed(&mut session, sheet, "D4", "=ROUND(SKEW.P(B1:B10),8)"),
        json!(0.30319334),
        "and the skew of the whole"
    );
}

/// The **place** of a number in a list.
#[test]
fn the_place_of_a_number_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[89.0, 88.0, 92.0, 101.0, 94.0, 97.0, 95.0]);
    assert_eq!(computed(&mut session, sheet, "D1", "=RANK.AVG(94,A1:A7)"), json!(4.0), "the place");
    assert_eq!(
        computed(&mut session, sheet, "D2", "=RANK.EQ(101,A1:A7)"),
        json!(1.0),
        "and the place of the greatest"
    );
    column(&mut session, sheet, "B", &[7.0, 3.5, 3.5, 1.0, 2.0]);
    assert_eq!(computed(&mut session, sheet, "D3", "=RANK.EQ(3.5,B1:B5,1)"), json!(3.0), "and of a tie");
    assert_eq!(
        computed(&mut session, sheet, "D4", "=RANK.AVG(3.5,B1:B5,1)"),
        json!(3.5),
        "and the place of a tie"
    );
}

/// The **percentiles**, the tree's two ways.
#[test]
fn the_percentiles_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[1.0, 3.0, 2.0, 4.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=PERCENTILE.INC(A1:A4,0.3)"),
        json!(1.9),
        "the inclusive one"
    );
    column(&mut session, sheet, "B", &[1.0, 2.0, 3.0, 6.0, 6.0, 6.0, 7.0, 8.0, 9.0]);
    assert_eq!(
        computed(&mut session, sheet, "D2", "=PERCENTILE.EXC(B1:B9,0.25)"),
        json!(2.5),
        "the exclusive one"
    );
    column(&mut session, sheet, "C", &[1.0, 2.0, 4.0, 7.0, 8.0, 9.0, 10.0, 12.0]);
    assert_eq!(computed(&mut session, sheet, "D3", "=QUARTILE.INC(C1:C8,1)"), json!(3.5), "the quartile");
    column(&mut session, sheet, "D", &[6.0, 7.0, 15.0, 36.0, 39.0, 40.0, 41.0, 42.0, 43.0, 47.0, 49.0]);
    assert_eq!(
        computed(&mut session, sheet, "E1", "=QUARTILE.EXC(D1:D11,1)"),
        json!(15.0),
        "and the exclusive one"
    );
}

/// The **place** of a number in the set, as a percentage.
#[test]
fn the_percentage_place_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[13.0, 12.0, 11.0, 8.0, 4.0, 3.0, 2.0, 1.0, 1.0, 1.0]);
    assert_eq!(computed(&mut session, sheet, "D1", "=PERCENTRANK.INC(A1:A10,2)"), json!(0.333), "the place");
    assert_eq!(
        computed(&mut session, sheet, "D2", "=PERCENTRANK(A1:A10,5)"),
        json!(0.583),
        "and the older name"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=PERCENTRANK.INC(A1:A10,5,1)"),
        json!(0.5),
        "and to one digit"
    );
    column(&mut session, sheet, "B", &[1.0, 2.0, 3.0, 6.0, 6.0, 6.0, 7.0, 8.0, 9.0]);
    assert_eq!(
        computed(&mut session, sheet, "D4", "=ROUND(PERCENTRANK.EXC(B1:B9,7),3)"),
        json!(0.7),
        "and the exclusive one"
    );
}

/// The tree's **errors**: a set too small for its shape, a number that
/// is not positive and a place that is not there.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    column(&mut session, sheet, "A", &[1.0, 2.0, 3.0]);
    assert_eq!(
        computed(&mut session, sheet, "D1", "=KURT(A1:A3)"),
        json!({ "error": "#DIV/0!" }),
        "a set of three has no kurtosis"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=GEOMEAN(A1:A3,0)"),
        json!({ "error": "#NUM!" }),
        "and a geometric mean wants every number positive"
    );
    column(&mut session, sheet, "B", &[89.0, 88.0, 92.0]);
    assert_eq!(
        computed(&mut session, sheet, "D3", "=RANK(1,B1:B3)"),
        json!({ "error": "#N/A" }),
        "and a place that is not there is missing"
    );
    assert_eq!(
        computed(&mut session, sheet, "D4", "=STANDARDIZE(42,40,0)"),
        json!({ "error": "#NUM!" }),
        "and a deviation of nothing is an error"
    );
}
