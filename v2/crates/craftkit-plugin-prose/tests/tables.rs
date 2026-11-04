//! The **cells** of a table: a grid of values.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// A **table** in a document, and its id.
fn a_table(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [["a", "b"], ["c", "d"]] }))
        .expect("the table is added");
    outcome.data["id"].as_u64().expect("the table has an id")
}

/// The **rows** of a table.
fn rows_of(session: &mut Session, table: u64) -> Vec<Vec<Json>> {
    let node = session.document().node(table).expect("the table is in the document");
    node.data
        .get("rows")
        .and_then(|rows| rows.as_array())
        .cloned()
        .expect("the rows are a list")
        .into_iter()
        .map(|row| row.as_array().cloned().expect("a row is a list of cells"))
        .collect()
}

/// A **cell** may be read.
#[test]
fn a_cell_may_be_read() {
    let mut session = session();
    let table = a_table(&mut session);
    let outcome = session
        .execute("prose.table.cell.get", &json!({ "table": table, "row": 1, "column": 0 }))
        .expect("the cell is read");
    assert_eq!(outcome.data["value"], json!("c"), "the cell holds what it holds");
}

/// A **cell** may be written.
#[test]
fn a_cell_may_be_written() {
    let mut session = session();
    let table = a_table(&mut session);
    session
        .execute("prose.table.cell.set", &json!({ "table": table, "row": 1, "column": 0, "value": "z" }))
        .expect("the cell is written");
    let rows = rows_of(&mut session, table);
    assert_eq!(rows[1][0], json!("z"), "the cell holds the new value");
    assert_eq!(rows[0][0], json!("a"), "and the other cells are as they were");
}

/// A **row** may be put in a table, and the rows below it move down.
#[test]
fn a_row_may_be_put_in_a_table() {
    let mut session = session();
    let table = a_table(&mut session);
    session.execute("prose.table.row.add", &json!({ "table": table, "at": 1 })).expect("the row is put in");
    let rows = rows_of(&mut session, table);
    assert_eq!(rows.len(), 3, "the table has three rows");
    assert_eq!(rows[0][0], json!("a"), "the first row is where it was");
    assert_eq!(rows[1], vec![Json::Null, Json::Null], "the new row is empty");
    assert_eq!(rows[2][0], json!("c"), "and the row below it moved down");
}

/// A **row** may be taken out of a table.
#[test]
fn a_row_may_be_taken_out_of_a_table() {
    let mut session = session();
    let table = a_table(&mut session);
    session
        .execute("prose.table.row.remove", &json!({ "table": table, "row": 0 }))
        .expect("the row is taken out");
    let rows = rows_of(&mut session, table);
    assert_eq!(rows.len(), 1, "the table has one row");
    assert_eq!(rows[0][0], json!("c"), "and it is the row that was below");
}

/// A **column** may be put in a table, and the columns beside it move
/// right.
#[test]
fn a_column_may_be_put_in_a_table() {
    let mut session = session();
    let table = a_table(&mut session);
    session
        .execute("prose.table.column.add", &json!({ "table": table, "at": 0 }))
        .expect("the column is put in");
    let rows = rows_of(&mut session, table);
    assert_eq!(rows[0].len(), 3, "the row has three cells");
    assert_eq!(rows[0][0], Json::Null, "the new column is empty");
    assert_eq!(rows[0][1], json!("a"), "and the column beside it moved right");
    assert_eq!(rows[1][1], json!("c"), "in every row of the table");
}

/// A **column** may be taken out of a table.
#[test]
fn a_column_may_be_taken_out_of_a_table() {
    let mut session = session();
    let table = a_table(&mut session);
    session
        .execute("prose.table.column.remove", &json!({ "table": table, "column": 0 }))
        .expect("the column is taken out");
    let rows = rows_of(&mut session, table);
    assert_eq!(rows[0].len(), 1, "the row has one cell");
    assert_eq!(rows[0][0], json!("b"), "and it is the cell that was beside");
}

/// A cell that is **not there** is refused.
#[test]
fn a_cell_that_is_not_there_is_refused() {
    let mut session = session();
    let table = a_table(&mut session);
    let value = session
        .execute("prose.table.cell.get", &json!({ "table": table, "row": 5, "column": 0 }))
        .expect_err("the cell is refused");
    assert!(value.to_string().contains("no row"), "the refusal says why, and it says {}", value);
}

/// A table whose rows are **not the same length** is refused: it is
/// not a grid, and a grid is what a cell needs.
#[test]
fn a_table_whose_rows_are_not_the_same_length_is_refused() {
    let mut session = session();
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let value = session
        .execute("prose.block.table.add", &json!({ "document": document, "rows": [["a", "b"], ["c"]] }))
        .expect_err("the table is refused");
    assert!(value.to_string().contains("same length"), "the refusal says why, and it says {}", value);
}

/// A **row** that is not a whole number is refused.
#[test]
fn a_row_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let table = a_table(&mut session);
    let value = session
        .execute("prose.table.row.remove", &json!({ "table": table, "row": 1.5 }))
        .expect_err("the row is refused");
    assert!(value.to_string().contains("whole number"), "the refusal says why, and it says {}", value);
}
