//! The **tests of a hypothesis**: a t-test, a chi-square test, an
//! F-test and a z-test.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put **numbers** in a row of cells.
fn row(session: &mut Session, sheet: u64, letter: &str, numbers: &[f64]) {
    for (index, number) in numbers.iter().enumerate() {
        let place = format!("{letter}{}", index + 1);
        value(session, sheet, &place, json!(number));
    }
}

/// The **t-test** of two sets.
#[test]
fn the_two_sets_are_compared() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    row(&mut session, sheet, "A", &[3.0, 4.0, 5.0, 8.0, 9.0, 1.0, 2.0, 4.0, 5.0]);
    row(&mut session, sheet, "B", &[6.0, 19.0, 3.0, 2.0, 14.0, 4.0, 5.0, 17.0, 1.0]);
    assert_eq!(
        computed(&mut session, sheet, "N1", "=ROUND(T.TEST(A1:A9,B1:B9,2,1),6)"),
        json!(0.196016),
        "the pairs"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=ROUND(TTEST(A1:A9,B1:B9,2,1),6)"),
        json!(0.196016),
        "and by its other name"
    );
    assert_eq!(
        computed(&mut session, sheet, "N3", "=ROUND(T.TEST(A1:A9,B1:B9,2,2),6)"),
        json!(0.191996),
        "and the two groups"
    );
    assert_eq!(
        computed(&mut session, sheet, "N4", "=ROUND(T.TEST(A1:A9,B1:B9,1,3),4)"),
        json!(0.1011),
        "and the two groups of an unknown size"
    );
    assert_eq!(
        computed(&mut session, sheet, "N5", "=T.TEST(A1:A9,B1:B9,3,1)"),
        json!({ "error": "#NUM!" }),
        "and three tails are an error"
    );
}

/// The **chi-square test** of what was observed and what was
/// expected.
#[test]
fn the_observation_and_the_expectation_are_compared() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let observed = [[58.0, 35.0], [11.0, 25.0], [10.0, 23.0]];
    let expected = [[45.35, 47.65], [17.56, 18.44], [16.09, 16.91]];
    for (index, row) in observed.iter().enumerate() {
        for (column, number) in row.iter().enumerate() {
            let letter = (b'A' + u8::try_from(column).unwrap_or(0)) as char;
            value(&mut session, sheet, &format!("{letter}{}", index + 1), json!(number));
        }
    }
    for (index, row) in expected.iter().enumerate() {
        for (column, number) in row.iter().enumerate() {
            let letter = (b'D' + u8::try_from(column).unwrap_or(0)) as char;
            value(&mut session, sheet, &format!("{letter}{}", index + 1), json!(number));
        }
    }
    assert_eq!(
        computed(&mut session, sheet, "N1", "=ROUND(CHISQ.TEST(A1:B3,D1:E3),6)"),
        json!(0.000308),
        "the table"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=ROUND(CHITEST(A1:B3,D1:E3),6)"),
        json!(0.000308),
        "and by its other name"
    );
    assert_eq!(
        computed(&mut session, sheet, "N3", "=CHISQ.TEST(A1:B3,D1:E2)"),
        json!({ "error": "#N/A" }),
        "and two tables of different shapes have no answer"
    );
}

/// The **F-test** of two sets.
#[test]
fn the_two_spreads_are_compared() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    row(&mut session, sheet, "C", &[6.0, 7.0, 9.0, 15.0, 21.0]);
    row(&mut session, sheet, "F", &[20.0, 28.0, 31.0, 38.0, 40.0]);
    assert_eq!(
        computed(&mut session, sheet, "N1", "=ROUND(F.TEST(C1:C5,F1:F5),8)"),
        json!(0.64831785),
        "the two spreads"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=ROUND(FTEST(C1:C5,F1:F5),8)"),
        json!(0.64831785),
        "and by its other name"
    );
}

/// The **z-test** of a set and a number.
#[test]
fn the_mean_is_compared_with_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    row(&mut session, sheet, "H", &[3.0, 6.0, 7.0, 8.0, 6.0, 5.0, 4.0, 2.0, 1.0, 9.0]);
    assert_eq!(
        computed(&mut session, sheet, "N1", "=ROUND(Z.TEST(H1:H10,4),6)"),
        json!(0.090574),
        "the mean of the set"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=ROUND(ZTEST(H1:H10,6),6)"),
        json!(0.863043),
        "and by its other name"
    );
}
