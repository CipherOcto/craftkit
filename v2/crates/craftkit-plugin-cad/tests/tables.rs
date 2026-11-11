//! The **table** of a drawing: a grid of cells, and the tree's own
//! rules for the merging of them.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_cad::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the cad battery.
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

/// A **drawing** with a **block** in it.
fn a_block(session: &mut Session) -> u64 {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    outcome.data["id"].as_u64().expect("the block has an id")
}

/// A **table** in a block.
fn a_table(session: &mut Session, block: u64, rows: i64, columns: i64) -> u64 {
    let outcome = session
        .execute(
            "cad.table.add",
            &json!({
                "block": block,
                "name": "the table",
                "at": { "x": 0.0, "y": 0.0 },
                "rows": rows,
                "columns": columns,
            }),
        )
        .expect("the table is added");
    outcome.data["table"].as_u64().expect("the table has an id")
}

/// The **cells** of a table, as the payload holds them.
fn cells(session: &Session, table: u64) -> Vec<Vec<Json>> {
    let node = session.document().node(table).expect("the table is there");
    node.data["geometry"]["cells"]
        .as_array()
        .expect("the table has cells")
        .iter()
        .map(|row| row.as_array().expect("a row of cells").clone())
        .collect()
}

/// The **text** of a cell.
fn text_of(session: &Session, table: u64, row: usize, column: usize) -> String {
    cells(session, table)[row][column]["text"].as_str().expect("the cell has a text").to_string()
}

/// The **merge** a cell holds, if it holds one.
fn merge_of(session: &Session, table: u64, row: usize, column: usize) -> Option<Vec<u64>> {
    cells(session, table)[row][column]["merged"]
        .as_array()
        .map(|span| span.iter().map(|value| value.as_u64().expect("a number")).collect())
}

/// A table is a **grid** of cells.
#[test]
fn a_table_is_a_grid_of_cells() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 3, 2);
    let cells = cells(&session, table);
    assert_eq!(cells.len(), 3, "the table has three rows");
    assert_eq!(cells[0].len(), 2, "and two columns");
    assert_eq!(text_of(&session, table, 0, 0), "", "and the cells are empty");
}

/// The **text** of a cell is written.
#[test]
fn the_text_of_a_cell_is_written() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    session
        .execute("cad.table.set", &json!({ "table": table, "row": 1, "column": 0, "text": "hello" }))
        .expect("the cell is written");
    assert_eq!(text_of(&session, table, 1, 0), "hello", "the cell says it");
}

/// A **merge** joins the texts and holds its span.
#[test]
fn a_merge_joins_the_texts() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    session
        .execute("cad.table.set", &json!({ "table": table, "row": 0, "column": 0, "text": "one" }))
        .expect("the cell is written");
    session
        .execute("cad.table.set", &json!({ "table": table, "row": 0, "column": 1, "text": "two" }))
        .expect("the cell is written");
    session
        .execute(
            "cad.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 1, "columns": 2 }),
        )
        .expect("the cells are merged");
    assert_eq!(text_of(&session, table, 0, 0), "one two", "the texts are joined into the anchor");
    assert_eq!(merge_of(&session, table, 0, 0), Some(vec![1, 2]), "and the anchor holds the merge");
    assert_eq!(merge_of(&session, table, 0, 1), None, "and the covered cell holds none");
}

/// Writing into a **covered** cell writes its anchor.
#[test]
fn writing_into_a_covered_cell_writes_its_anchor() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    session
        .execute(
            "cad.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 2, "columns": 1 }),
        )
        .expect("the cells are merged");
    session
        .execute("cad.table.set", &json!({ "table": table, "row": 1, "column": 0, "text": "hello" }))
        .expect("the cell is written");
    assert_eq!(text_of(&session, table, 0, 0), "hello", "the anchor says it");
    assert_eq!(text_of(&session, table, 1, 0), "", "and the covered cell does not");
}

/// A merge of **one cell** is refused.
#[test]
fn a_merge_of_one_cell_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    let error = session
        .execute(
            "cad.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 1, "columns": 1 }),
        )
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("merge at least two cells"), "the refusal says so: {error}");
}

/// A merge that goes **past the edge** of the table is refused: the
/// tree clamps it and this tree does not.
#[test]
fn a_merge_past_the_edge_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    let error = session
        .execute(
            "cad.table.merge",
            &json!({ "table": table, "row": 1, "column": 1, "rows": 2, "columns": 2 }),
        )
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("past the edge"), "the refusal says so: {error}");
}

/// An **unmerge** takes the merge away.
#[test]
fn an_unmerge_takes_the_merge_away() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    session
        .execute(
            "cad.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 1, "columns": 2 }),
        )
        .expect("the cells are merged");
    session
        .execute("cad.table.unmerge", &json!({ "table": table, "row": 0, "column": 0 }))
        .expect("the cells are unmerged");
    assert_eq!(merge_of(&session, table, 0, 0), None, "the merge is gone");
}

/// A row is **inserted before** the row that is named, and the merges
/// move with the rows.
#[test]
fn a_row_is_inserted_before_the_row_that_is_named() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 2, 2);
    session
        .execute("cad.table.set", &json!({ "table": table, "row": 1, "column": 0, "text": "hello" }))
        .expect("the cell is written");
    session
        .execute("cad.table.insert.row", &json!({ "table": table, "row": 0 }))
        .expect("the row is inserted");
    let cells = cells(&session, table);
    assert_eq!(cells.len(), 3, "the table has another row");
    assert_eq!(text_of(&session, table, 2, 0), "hello", "and the text moved down");
}

/// A table keeps **at least one row**.
#[test]
fn a_table_keeps_at_least_one_row() {
    let mut session = session();
    let block = a_block(&mut session);
    let table = a_table(&mut session, block, 1, 2);
    let error = session
        .execute("cad.table.delete.row", &json!({ "table": table, "row": 0 }))
        .expect_err("the row is not deleted");
    assert!(error.to_string().contains("at least one row"), "the refusal says so: {error}");
}

/// A table that is asked for **too many rows** is refused: the tree
/// clamps it and this tree does not.
#[test]
fn a_table_with_too_many_rows_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let error = session
        .execute(
            "cad.table.add",
            &json!({
                "block": block,
                "name": "the table",
                "at": { "x": 0.0, "y": 0.0 },
                "rows": 2001,
                "columns": 2,
            }),
        )
        .expect_err("the table is refused");
    assert!(error.to_string().contains("at most 2000 rows"), "the refusal says so: {error}");
}
