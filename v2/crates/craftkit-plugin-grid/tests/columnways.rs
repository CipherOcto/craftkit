//! The **way of a column** of a table.

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

/// A **table** of two columns: `amount` with numbers under it, and
/// `who` with words.
fn a_table(session: &mut Session, sheet: u64) {
    a_cell(session, sheet, "A1", json!("amount"));
    a_cell(session, sheet, "A2", json!(10.0));
    a_cell(session, sheet, "A3", json!(20.0));
    a_cell(session, sheet, "B1", json!("who"));
    a_cell(session, sheet, "B2", json!("one"));
    a_cell(session, sheet, "B3", json!("two"));
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "the table", "range": "A1:B4" }))
        .expect("the table is created");
}

/// The **fields** of a table, as the command reads them.
fn fields_of(session: &mut Session, sheet: u64) -> Json {
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("the fields are read");
    outcome.data["fields"].clone()
}

/// The **total** of a column of a table.
fn total_of(session: &mut Session, sheet: u64, column: &str) -> Json {
    let outcome = session
        .execute("grid.table.fields", &json!({ "sheet": sheet, "name": "the table" }))
        .expect("the fields are read");
    let sums = outcome.data["sums"].as_array().expect("the sums are a list");
    let sum = sums.iter().find(|sum| sum["name"] == json!(column)).expect("the column has a total");
    sum["total"].clone()
}

/// The **way** of a column of a table.
fn way_of(session: &mut Session, sheet: u64, column: &str) -> Json {
    let fields = fields_of(session, sheet);
    let fields = fields.as_array().expect("the fields are a list");
    let field = fields.iter().find(|field| field["name"] == json!(column)).expect("the column is there");
    field["way"].clone()
}

/// A column's **own way** is said.
#[test]
fn a_columns_own_way_is_said() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "max" }),
        )
        .expect("the way is said");
    assert_eq!(way_of(&mut session, sheet, "amount"), json!("max"), "the way is the column's own");
}

/// A column with **no way of its own** is totalled in the way of the
/// table.
#[test]
fn a_column_with_no_way_of_its_own_is_totalled_in_the_way_of_the_table() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "the table", "way": "average" }))
        .expect("the way is said");
    assert_eq!(way_of(&mut session, sheet, "amount"), json!("average"), "the table's way is the column's");
    assert_eq!(way_of(&mut session, sheet, "who"), json!("average"), "and the other one too");
}

/// A column's **own way** is the way of that column and not of the
/// table.
#[test]
fn a_columns_own_way_is_the_way_of_that_column_and_not_of_the_table() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "the table", "way": "average" }))
        .expect("the way is said");
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "max" }),
        )
        .expect("the way is said");
    assert_eq!(way_of(&mut session, sheet, "amount"), json!("max"), "the column keeps its own way");
    assert_eq!(way_of(&mut session, sheet, "who"), json!("average"), "the other one has the table's");
}

/// The **totals** follow the way of each column.
#[test]
fn the_totals_follow_the_way_of_each_column() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute("grid.table.totals", &json!({ "sheet": sheet, "name": "the table", "totals": true }))
        .expect("the total row is said");
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "average" }),
        )
        .expect("the way is said");
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "who", "way": "count" }),
        )
        .expect("the way is said");
    assert_eq!(total_of(&mut session, sheet, "amount"), json!(15.0), "the average of ten and twenty");
    assert_eq!(total_of(&mut session, sheet, "who"), json!(2.0), "there are two words");
}

/// A column that is **not a column** of the table is refused.
#[test]
fn a_column_that_is_not_a_column_of_the_table_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let refused = session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "when", "way": "max" }),
        )
        .expect_err("the column is refused");
    assert!(refused.to_string().contains("no column called `when`"), "the refusal says why: {refused}");
}

/// A name that **two columns share** is refused.
#[test]
fn a_name_that_two_columns_share_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    a_cell(&mut session, sheet, "B1", json!("amount"));
    let refused = session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "max" }),
        )
        .expect_err("the name is refused");
    assert!(refused.to_string().contains("does not say which one"), "the refusal says why: {refused}");
}

/// A way that is **not a way** is refused.
#[test]
fn a_way_that_is_not_a_way_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let refused = session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "median" }),
        )
        .expect_err("the way is refused");
    assert!(refused.to_string().contains("not a way to total"), "the refusal says why: {refused}");
}

/// An **empty way** takes the column's own way away.
#[test]
fn an_empty_way_takes_the_columns_own_way_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "max" }),
        )
        .expect("the way is said");
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "" }),
        )
        .expect("the way is taken away");
    assert_eq!(way_of(&mut session, sheet, "amount"), json!("sum"), "the column falls back to the table");
}

/// The way of a column belongs to the **column** and not to its name.
#[test]
fn the_way_of_a_column_belongs_to_the_column_and_not_to_its_name() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "max" }),
        )
        .expect("the way is said");
    a_cell(&mut session, sheet, "A1", json!("sum"));
    assert_eq!(way_of(&mut session, sheet, "sum"), json!("max"), "the way is with the column still");
}

/// The ways of the columns that **stay** are kept when the table is
/// resized.
#[test]
fn the_ways_of_the_columns_that_stay_are_kept_when_the_table_is_resized() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "amount", "way": "max" }),
        )
        .expect("the way is said");
    session
        .execute("grid.table.resize", &json!({ "sheet": sheet, "name": "the table", "range": "A1:B6" }))
        .expect("the table is resized");
    assert_eq!(way_of(&mut session, sheet, "amount"), json!("max"), "the way stays with the column");
}

/// The way of a column that is **not there any more** is gone.
#[test]
fn the_way_of_a_column_that_is_not_there_any_more_is_gone() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the table", "column": "who", "way": "max" }),
        )
        .expect("the way is said");
    session
        .execute("grid.table.resize", &json!({ "sheet": sheet, "name": "the table", "range": "A1:A4" }))
        .expect("the table is resized");
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert_eq!(tables[0]["ways"].as_array().expect("the ways are a list").len(), 0, "the way is gone");
}

/// A table that is **not there** is refused.
#[test]
fn a_table_that_is_not_there_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_table(&mut session, sheet);
    let refused = session
        .execute(
            "grid.table.way",
            &json!({ "sheet": sheet, "name": "the other table", "column": "amount", "way": "max" }),
        )
        .expect_err("the table is refused");
    assert!(refused.to_string().contains("no table `the other table`"), "the refusal says why: {refused}");
}
