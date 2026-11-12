//! The **merged cells** of a table.

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

/// A **document** with a table in it, and the table's id.
fn a_table(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute(
            "prose.block.table.add",
            &json!({
                "document": document,
                "rows": [["one", "two"], ["three", "four"]],
            }),
        )
        .expect("the table is added");
    let table = outcome.data["id"].as_u64().expect("the table has an id");
    (document, table)
}

/// The **merges** of a table, as the command reads them.
fn merges_of(session: &mut Session, table: u64) -> Vec<Json> {
    let outcome =
        session.execute("prose.table.merges", &json!({ "table": table })).expect("the merges are read");
    outcome.data["merges"].as_array().expect("the merges are a list").clone()
}

/// The **value** of a cell of a table.
fn cell_of(session: &mut Session, table: u64, row: u64, column: u64) -> Json {
    let outcome = session
        .execute("prose.table.cell.get", &json!({ "table": table, "row": row, "column": column }))
        .expect("the cell is read");
    outcome.data["value"].clone()
}

/// Two cells are **merged** into one.
#[test]
fn two_cells_are_merged_into_one() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    session
        .execute(
            "prose.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 2, "columns": 1 }),
        )
        .expect("the cells are merged");
    let merges = merges_of(&mut session, table);
    assert_eq!(merges.len(), 1, "there is one merge");
    assert_eq!(merges[0]["rows"], json!(2), "and it reaches over two rows");
}

/// The cells a merge covers are **emptied**, and the top left one
/// keeps its value.
#[test]
fn the_cells_a_merge_covers_are_emptied() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    session
        .execute(
            "prose.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 2, "columns": 1 }),
        )
        .expect("the cells are merged");
    assert_eq!(cell_of(&mut session, table, 0, 0), json!("one"), "the value is kept");
    assert_eq!(cell_of(&mut session, table, 1, 0), Json::Null, "and the covered cell is empty");
}

/// A merge of **one cell** is refused.
#[test]
fn a_merge_of_one_cell_is_refused() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    let outcome = session.execute("prose.table.merge", &json!({ "table": table, "row": 0, "column": 0 }));
    assert!(outcome.is_err(), "one cell is not a merge");
}

/// A merge that reaches **outside the table** is refused.
#[test]
fn a_merge_outside_the_table_is_refused() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    let outcome = session.execute(
        "prose.table.merge",
        &json!({ "table": table, "row": 1, "column": 0, "rows": 3, "columns": 1 }),
    );
    assert!(outcome.is_err(), "the merge reaches outside the table");
}

/// A merge that **overlaps** another merge is refused.
#[test]
fn a_merge_that_overlaps_another_is_refused() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    session
        .execute(
            "prose.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 2, "columns": 1 }),
        )
        .expect("the cells are merged");
    let outcome = session.execute(
        "prose.table.merge",
        &json!({ "table": table, "row": 1, "column": 0, "rows": 2, "columns": 1 }),
    );
    assert!(outcome.is_err(), "two cells that are one cell are not one cell");
}

/// A merged cell may be **split**.
#[test]
fn a_merged_cell_may_be_split() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    session
        .execute(
            "prose.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 2, "columns": 1 }),
        )
        .expect("the cells are merged");
    session
        .execute("prose.table.split", &json!({ "table": table, "row": 1, "column": 0 }))
        .expect("the cell is split");
    let merges = merges_of(&mut session, table);
    assert_eq!(merges.len(), 0, "there is no merge now");
}

/// A cell that is **not merged** may not be split.
#[test]
fn a_cell_that_is_not_merged_may_not_be_split() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    let outcome = session.execute("prose.table.split", &json!({ "table": table, "row": 0, "column": 0 }));
    assert!(outcome.is_err(), "there is no merge there");
}

/// A row that holds the value of a **merged cell** may not be taken
/// out.
#[test]
fn a_row_that_holds_a_merged_value_may_not_be_taken_out() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    session
        .execute(
            "prose.table.merge",
            &json!({ "table": table, "row": 0, "column": 0, "rows": 2, "columns": 1 }),
        )
        .expect("the cells are merged");
    let outcome = session.execute("prose.table.row.remove", &json!({ "table": table, "row": 0 }));
    assert!(outcome.is_err(), "the value of the merge would go with the row");
}

/// A row that goes in **beside** a merge moves it.
#[test]
fn a_row_that_goes_in_beside_a_merge_moves_it() {
    let mut session = session();
    let (_document, table) = a_table(&mut session);
    session
        .execute(
            "prose.table.merge",
            &json!({ "table": table, "row": 1, "column": 0, "rows": 1, "columns": 2 }),
        )
        .expect("the cells are merged");
    session.execute("prose.table.row.add", &json!({ "table": table, "at": 0 })).expect("the row is put in");
    let merges = merges_of(&mut session, table);
    assert_eq!(merges[0]["row"], json!(2), "the merge moved down");
    assert_eq!(merges[0]["columns"], json!(2), "and it still reaches across two columns");
}
