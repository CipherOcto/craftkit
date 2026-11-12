//! The **database functions**: a table of records and a table of
//! criteria.

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

/// The **orchard**: the tree's own table of trees.
fn orchard(session: &mut Session, sheet: u64) {
    let rows: Vec<[Json; 5]> = vec![
        [json!("Tree"), json!("Height"), json!("Age"), json!("Yield"), json!("Profit")],
        [json!("Apple"), json!(18.0), json!(20.0), json!(14.0), json!(105.0)],
        [json!("Pear"), json!(12.0), json!(12.0), json!(10.0), json!(96.0)],
        [json!("Cherry"), json!(13.0), json!(14.0), json!(9.0), json!(105.0)],
        [json!("Apple"), json!(14.0), json!(15.0), json!(10.0), json!(75.0)],
        [json!("Pear"), json!(9.0), json!(8.0), json!(8.0), json!(76.8)],
        [json!("Apple"), json!(8.0), json!(9.0), json!(6.0), json!(45.0)],
    ];
    for (index, row) in rows.iter().enumerate() {
        for (column, cell) in row.iter().enumerate() {
            let letter = (b'A' + u8::try_from(column).unwrap_or(0)) as char;
            let place = format!("{letter}{}", index + 1);
            value(session, sheet, &place, cell.clone());
        }
    }
}

/// A **criteria** table: a header row and one row of conditions.
fn criteria(session: &mut Session, sheet: u64, row: usize, header: &[&str], conditions: &[Option<Json>]) {
    for (column, label) in header.iter().enumerate() {
        let letter = (b'G' + u8::try_from(column).unwrap_or(0)) as char;
        value(session, sheet, &format!("{letter}{row}"), json!(label));
    }
    for (column, condition) in conditions.iter().enumerate() {
        let Some(condition) = condition else { continue };
        let letter = (b'G' + u8::try_from(column).unwrap_or(0)) as char;
        value(session, sheet, &format!("{letter}{}", row + 1), condition.clone());
    }
}

/// The **answers** of the orchard under the tree's own criteria.
#[test]
fn the_records_that_meet_the_criteria_are_aggregated() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    orchard(&mut session, sheet);
    criteria(
        &mut session,
        sheet,
        1,
        &["Tree", "Height", "Age", "Yield", "Profit", "Height"],
        &[Some(json!("=Apple")), Some(json!(">10")), None, None, None, Some(json!("<16"))],
    );
    // A second row of criteria: any pear. The two rows are one
    // criteria table, the table at G1.
    value(&mut session, sheet, "G3", json!("=Pear"));
    assert_eq!(
        computed(&mut session, sheet, "N1", "=ROUND(DSUM(A1:E7,\"Profit\",G1:L3),8)"),
        json!(247.8),
        "the sum of the profits"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=DCOUNT(A1:E7,\"Age\",G1:L3)"),
        json!(3.0),
        "the count of the ages"
    );
    assert_eq!(
        computed(&mut session, sheet, "N3", "=DCOUNTA(A1:E7,\"Profit\",G1:L3)"),
        json!(3.0),
        "the count of the profits"
    );
    assert_eq!(
        computed(&mut session, sheet, "N4", "=DMAX(A1:E7,\"Profit\",G1:L3)"),
        json!(96.0),
        "the largest profit"
    );
    assert_eq!(
        computed(&mut session, sheet, "N5", "=DMIN(A1:E7,\"Profit\",G1:L3)"),
        json!(75.0),
        "the smallest profit"
    );
    assert_eq!(
        computed(&mut session, sheet, "N6", "=DPRODUCT(A1:E7,\"Yield\",G1:L3)"),
        json!(800.0),
        "the product of the yields"
    );
    assert_eq!(
        computed(&mut session, sheet, "N7", "=ROUND(DAVERAGE(A1:E7,\"Yield\",G1:L3),8)"),
        json!(9.33333333),
        "the mean of the yields"
    );
    assert_eq!(
        computed(&mut session, sheet, "N8", "=ROUND(DSTDEV(A1:E7,\"Yield\",G1:L3),8)"),
        json!(1.15470054),
        "the spread of the yields"
    );
    assert_eq!(
        computed(&mut session, sheet, "N9", "=ROUND(DSTDEVP(A1:E7,\"Yield\",G1:L3),8)"),
        json!(0.94280904),
        "the spread of the whole table"
    );
    assert_eq!(
        computed(&mut session, sheet, "N10", "=ROUND(DVAR(A1:E7,\"Yield\",G1:L3),8)"),
        json!(1.33333333),
        "the variance of the yields"
    );
    assert_eq!(
        computed(&mut session, sheet, "N11", "=ROUND(DVARP(A1:E7,\"Yield\",G1:L3),8)"),
        json!(0.88888889),
        "the variance of the whole table"
    );
    assert_eq!(
        computed(&mut session, sheet, "N12", "=ROUND(DSUM(A1:E7,5,G1:L3),8)"),
        json!(247.8),
        "the field said by its number"
    );
    assert_eq!(
        computed(&mut session, sheet, "N13", "=DCOUNT(A1:E7,Z1,G1:L3)"),
        json!(3.0),
        "the count of the records"
    );
    assert_eq!(
        computed(&mut session, sheet, "N14", "=DSUM(A1:E7,\"Nope\",G1:L3)"),
        json!({ "error": "#VALUE!" }),
        "a field that names no column is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "N15", "=DSUM(A1:E7,9,G1:L3)"),
        json!({ "error": "#VALUE!" }),
        "and a field that is past the last column is an error"
    );
}

/// The **conditions** of a criteria table.
#[test]
fn the_conditions_of_a_criteria_table_are_met() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    orchard(&mut session, sheet);
    criteria(&mut session, sheet, 6, &["Tree", "Height"], &[Some(json!("Apple")), None]);
    criteria(&mut session, sheet, 9, &["Tree", "Height"], &[Some(json!("P")), None]);
    criteria(&mut session, sheet, 12, &["Tree", "Height"], &[Some(json!("=P")), None]);
    criteria(&mut session, sheet, 15, &["Tree", "Height"], &[None, Some(json!(">=13"))]);
    criteria(&mut session, sheet, 18, &["Tree", "Height"], &[None, Some(json!(18.0))]);
    criteria(&mut session, sheet, 21, &["Tree", "Height"], &[None, None]);
    criteria(&mut session, sheet, 24, &["Tree", "Height"], &[Some(json!("*e*")), Some(json!("<10"))]);
    criteria(&mut session, sheet, 27, &["Tree"], &[None]);
    criteria(&mut session, sheet, 30, &["Color"], &[Some(json!("red"))]);
    assert_eq!(
        computed(&mut session, sheet, "N1", "=ROUND(DSUM(A1:E7,\"Profit\",G6:H7),8)"),
        json!(225.0),
        "the sum of the profits of the apples"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=DCOUNT(A1:E7,\"Age\",G9:H10)"),
        json!(2.0),
        "plain text matches what begins with it"
    );
    assert_eq!(
        computed(&mut session, sheet, "N3", "=DCOUNT(A1:E7,\"Age\",G12:H13)"),
        json!(0.0),
        "and an equality does not"
    );
    assert_eq!(
        computed(&mut session, sheet, "N4", "=ROUND(DSUM(A1:E7,\"Yield\",G15:H16),8)"),
        json!(33.0),
        "the sum of the yields of the tall trees"
    );
    assert_eq!(
        computed(&mut session, sheet, "N5", "=ROUND(DSUM(A1:E7,\"Yield\",G18:H19),8)"),
        json!(14.0),
        "the sum of the yields of the tree of that height"
    );
    assert_eq!(
        computed(&mut session, sheet, "N6", "=ROUND(DSUM(A1:E7,\"Yield\",G21:H22),8)"),
        json!(57.0),
        "and the sum of the yields of every tree"
    );
    assert_eq!(
        computed(&mut session, sheet, "N7", "=ROUND(DSUM(A1:E7,\"Yield\",G24:H25),8)"),
        json!(14.0),
        "the sum of the yields of the short trees with an e"
    );
    assert_eq!(
        computed(&mut session, sheet, "N8", "=DCOUNTA(A1:E7,\"tree\",G27:G28)"),
        json!(6.0),
        "a header and no conditions match every record"
    );
    assert_eq!(
        computed(&mut session, sheet, "N9", "=DCOUNT(A1:E7,\"Age\",G30:G31)"),
        json!(0.0),
        "a label that names no field never matches"
    );
}

/// The **value** of the one record that matches.
#[test]
fn the_value_of_the_one_record_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    orchard(&mut session, sheet);
    criteria(&mut session, sheet, 6, &["Tree"], &[Some(json!("=Cherry"))]);
    criteria(&mut session, sheet, 9, &["Tree"], &[Some(json!("=Apple"))]);
    criteria(&mut session, sheet, 12, &["Tree"], &[Some(json!("=Plum"))]);
    criteria(&mut session, sheet, 15, &["Tree"], &[Some(json!("=Pear"))]);
    assert_eq!(
        computed(&mut session, sheet, "N1", "=DGET(A1:E7,\"Yield\",G6:G7)"),
        json!(9.0),
        "the yield of the cherry"
    );
    assert_eq!(
        computed(&mut session, sheet, "N2", "=DGET(A1:E7,\"Yield\",G9:G10)"),
        json!({ "error": "#NUM!" }),
        "and three apples are too many"
    );
    assert_eq!(
        computed(&mut session, sheet, "N3", "=DGET(A1:E7,\"Yield\",G12:G13)"),
        json!({ "error": "#VALUE!" }),
        "and a plum is not there"
    );
    assert_eq!(
        computed(&mut session, sheet, "N4", "=DAVERAGE(A1:E7,\"Yield\",G12:G13)"),
        json!({ "error": "#DIV/0!" }),
        "and the mean of nothing is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "N5", "=DMAX(A1:E7,\"Yield\",G12:G13)"),
        json!(0.0),
        "and the largest of nothing is nothing"
    );
    assert_eq!(
        computed(&mut session, sheet, "N6", "=DPRODUCT(A1:E7,\"Yield\",G12:G13)"),
        json!(0.0),
        "and the product of nothing is nothing"
    );
    assert_eq!(
        computed(&mut session, sheet, "N7", "=DSTDEV(A1:E7,\"Yield\",G6:G7)"),
        json!({ "error": "#DIV/0!" }),
        "and the spread of one number is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "N8", "=DSTDEVP(A1:E7,\"Yield\",G6:G7)"),
        json!(0.0),
        "and the spread of the whole table is nothing"
    );
    assert_eq!(
        computed(&mut session, sheet, "N9", "=DCOUNTA(A1:E7,Z1,G15:G16)"),
        json!(2.0),
        "and the count of the records is the count"
    );
}
