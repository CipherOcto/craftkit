//! The **way** a table totals.

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

/// A **sheet** in a workbook.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **cell** of a sheet, holding a value.
fn a_cell(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **table** over the range, with its header row of text, and a row
/// of numbers under it.
fn a_table(session: &mut Session, sheet: u64) {
    a_cell(session, sheet, "A1", json!("amount"));
    a_cell(session, sheet, "A2", json!(10.0));
    a_cell(session, sheet, "A3", json!(20.0));
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "the table", "range": "A1:A4" }))
        .expect("the table is created");
}

/// The **totals** of a table, as the command reads them.
fn totals_of(session: &mut Session, sheet: u64, way: &str) -> Json {
    session
        .execute(
            "grid.table.totals",
            &json!({ "sheet": sheet, "name": "the table", "totals": true, "way": way }),
        )
        .expect("the way is said");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("the fields are read");
    outcome.data["sums"].as_array().expect("the sums are a list")[0]["total"].clone()
}

/// The way a table totals is **said**.
#[test]
fn the_way_a_table_totals_is_said() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.totals",
            &json!({ "sheet": sheet, "name": "the table", "totals": true, "way": "max" }),
        )
        .expect("the way is said");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("the fields are read");
    assert_eq!(outcome.data["way"], json!("max"), "the way is said");
}

/// The **sum** is the way a table totals by default.
#[test]
fn the_sum_is_the_way_a_table_totals_by_default() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "the table", "totals": true }))
        .expect("the total row is said");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("the fields are read");
    assert_eq!(outcome.data["way"], json!("sum"), "the way is the sum");
}

/// The **average** of the numbers.
#[test]
fn the_average_of_the_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let total = totals_of(&mut session, sheet, "average");
    assert_eq!(total, json!(15.0), "the average of ten and twenty is fifteen");
}

/// The **count** of the cells that hold something.
#[test]
fn the_count_of_the_cells_that_hold_something() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let total = totals_of(&mut session, sheet, "count");
    assert_eq!(total, json!(2.0), "there are two cells with something in them");
}

/// The **count** of the numbers.
#[test]
fn the_count_of_the_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    a_cell(&mut session, sheet, "A3", json!("the words"));
    let total = totals_of(&mut session, sheet, "count-numbers");
    assert_eq!(total, json!(1.0), "there is one number");
}

/// The **largest** of the numbers.
#[test]
fn the_largest_of_the_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    assert_eq!(totals_of(&mut session, sheet, "max"), json!(20.0), "the largest is twenty");
}

/// The **smallest** of the numbers.
#[test]
fn the_smallest_of_the_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    assert_eq!(totals_of(&mut session, sheet, "min"), json!(10.0), "the smallest is ten");
}

/// The **variance** of the numbers.
#[test]
fn the_variance_of_the_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let total = totals_of(&mut session, sheet, "variance");
    assert_eq!(total, json!(50.0), "the variance of ten and twenty is fifty");
}

/// The **standard deviation** of the numbers.
#[test]
fn the_standard_deviation_of_the_numbers() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let total = totals_of(&mut session, sheet, "standard-deviation");
    assert!(
        (total.as_f64().unwrap_or_default() - 50.0_f64.sqrt()).abs() < 0.0001,
        "the deviation is the root of the variance"
    );
}

/// A column with **one number** has no variance.
#[test]
fn a_column_with_one_number_has_no_variance() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_cell(&mut session, sheet, "A1", json!("amount"));
    a_cell(&mut session, sheet, "A2", json!(10.0));
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "the table", "range": "A1:A3" }))
        .expect("the table is created");
    let total = totals_of(&mut session, sheet, "variance");
    assert_eq!(total, Json::Null, "the variance of one number is nothing");
}

/// The way **none** gives no total at all.
#[test]
fn the_way_none_gives_no_total_at_all() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let total = totals_of(&mut session, sheet, "none");
    assert_eq!(total, Json::Null, "there is no total");
}

/// A way that is **not a way** is refused.
#[test]
fn a_way_that_is_not_a_way_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let refused = session
        .execute(
            "grid.table.totals",
            &json!({ "sheet": sheet, "name": "the table", "totals": true, "way": "median" }),
        )
        .expect_err("the way is refused");
    assert!(refused.to_string().contains("not a way to total"), "the refusal says why: {refused}");
}

/// The way that is **not said** is left as it was.
#[test]
fn the_way_that_is_not_said_is_left_as_it_was() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.totals",
            &json!({ "sheet": sheet, "name": "the table", "totals": true, "way": "max" }),
        )
        .expect("the way is said");
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("nothing is said");
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("the fields are read");
    assert_eq!(outcome.data["way"], json!("max"), "the way is left");
    assert_eq!(outcome.data["totals"], json!(true), "and so is the total row");
}
