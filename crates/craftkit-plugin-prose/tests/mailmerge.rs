//! The **mail merge** of a document: the fields of a document filled
//! with the rows of a data source.

use craftkit_sdk::serde_json::json;
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

/// A **document** with a paragraph in it, and the id of the paragraph.
fn a_document(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("prose.document.add", &json!({ "name": "letter" })).expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "Dear X, welcome." }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    (document, block)
}

/// The **fields** the document holds.
fn fields_of(session: &mut Session, block: u64) -> Vec<String> {
    let outcome =
        session.execute("prose.block.field.read", &json!({ "block": block })).expect("the fields are read");
    outcome.data["fields"]
        .as_array()
        .expect("the fields are a list")
        .iter()
        .map(|field| field["code"].as_str().expect("a code").to_string())
        .collect()
}

/// **A merge field is filled with the value of its column.**
#[test]
fn a_merge_field_is_filled_with_the_value_of_its_column() {
    let mut session = session();
    let (_document, block) = a_document(&mut session);
    // The paragraph holds "Dear X, welcome." and the field covers the
    // "X".
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 5.0, "end": 6.0, "code": "MERGEFIELD FirstName" }),
        )
        .expect("the field is added");
    let outcome = session
        .execute("prose.merge", &json!({ "headers": ["FirstName", "City"], "rows": [["Ann", "Lisbon"]] }))
        .expect("the rows are merged");
    let rows = outcome.data["rows"].as_array().expect("the rows are a list");
    assert_eq!(rows.len(), 1, "one row is merged: {outcome:?}");
    let blocks = rows[0]["blocks"].as_array().expect("the blocks are a list");
    let text = blocks[0]["text"].as_str().expect("the text is a string");
    assert_eq!(text, "Dear Ann, welcome.", "the field holds the value: {outcome:?}");
}

/// **The name of a column is not case-sensitive.**
#[test]
fn the_name_of_a_column_is_not_case_sensitive() {
    let mut session = session();
    let (_document, block) = a_document(&mut session);
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 5.0, "end": 6.0, "code": "mergefield firstname" }),
        )
        .expect("the field is added");
    let outcome = session
        .execute("prose.merge", &json!({ "headers": ["FIRSTNAME"], "rows": [["Ann"]] }))
        .expect("the rows are merged");
    let rows = outcome.data["rows"].as_array().expect("the rows are a list");
    let blocks = rows[0]["blocks"].as_array().expect("the blocks are a list");
    assert_eq!(blocks[0]["text"], json!("Dear Ann, welcome."), "the field holds the value: {outcome:?}");
}

/// **A column the data source does not have is an empty value.**
#[test]
fn a_column_the_data_source_does_not_have_is_an_empty_value() {
    let mut session = session();
    let (_document, block) = a_document(&mut session);
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 5.0, "end": 6.0, "code": "MERGEFIELD Nothing" }),
        )
        .expect("the field is added");
    let outcome = session
        .execute("prose.merge", &json!({ "headers": ["FirstName"], "rows": [["Ann"]] }))
        .expect("the rows are merged");
    let rows = outcome.data["rows"].as_array().expect("the rows are a list");
    let blocks = rows[0]["blocks"].as_array().expect("the blocks are a list");
    assert_eq!(blocks[0]["text"], json!("Dear , welcome."), "the value is empty: {outcome:?}");
}

/// **A field that is not a merge field is left alone.**
#[test]
fn a_field_that_is_not_a_merge_field_is_left_alone() {
    let mut session = session();
    let (_document, block) = a_document(&mut session);
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 5.0, "end": 6.0, "code": "DATE" }),
        )
        .expect("the field is added");
    let outcome = session
        .execute("prose.merge", &json!({ "headers": ["FirstName"], "rows": [["Ann"]] }))
        .expect("the rows are merged");
    assert_eq!(outcome.data["fields"], json!([]), "no field is a merge field: {outcome:?}");
    let rows = outcome.data["rows"].as_array().expect("the rows are a list");
    let blocks = rows[0]["blocks"].as_array().expect("the blocks are a list");
    assert_eq!(blocks[0]["text"], json!("Dear X, welcome."), "and the text is as it was: {outcome:?}");
}

/// **The address block is the tree's own.**
#[test]
fn the_address_block_is_the_trees_own() {
    let mut session = session();
    a_document(&mut session);
    let outcome = session
        .execute(
            "prose.merge",
            &json!({
                "headers": ["FirstName", "LastName", "City", "Country"],
                "rows": [["Ann", "Lee", "Lisbon", "PT"]],
            }),
        )
        .expect("the rows are merged");
    let rows = outcome.data["rows"].as_array().expect("the rows are a list");
    let address = rows[0]["address"].as_str().expect("the address is a string");
    assert_eq!(address, "Ann Lee\nLisbon\nPT", "the address is the tree's: {outcome:?}");
}

/// **The rows that are not lists are refused.**
#[test]
fn the_rows_that_are_not_lists_are_refused() {
    let mut session = session();
    a_document(&mut session);
    let refusal = session
        .execute("prose.merge", &json!({ "headers": ["FirstName"], "rows": ["Ann"] }))
        .expect_err("the rows are refused");
    let message = refusal.to_string();
    assert!(message.contains("rows"), "the refusal says so: {message}");
}

/// **The fields of the document are said.**
#[test]
fn the_fields_of_the_document_are_said() {
    let mut session = session();
    let (_document, block) = a_document(&mut session);
    session
        .execute(
            "prose.block.field.add",
            &json!({ "block": block, "start": 5.0, "end": 6.0, "code": "MERGEFIELD FirstName" }),
        )
        .expect("the field is added");
    let codes = fields_of(&mut session, block);
    assert_eq!(codes.len(), 1, "the document has one field: {codes:?}");
    let outcome = session
        .execute("prose.merge", &json!({ "headers": ["FirstName"], "rows": [["Ann"]] }))
        .expect("the rows are merged");
    assert_eq!(outcome.data["fields"], json!(["FirstName"]), "and the merge field is said: {outcome:?}");
}
