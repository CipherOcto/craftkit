//! The **table of contents** of a document.
//!
//! The entries are the headings of the document, with the level each
//! of them is at, and they are derived.

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

/// A **style** with a **level**, and its id.
fn a_heading(session: &mut Session, document: u64, name: &str, level: u64) -> u64 {
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": name, "style_kind": "paragraph" }))
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    session
        .execute("prose.style.level", &json!({ "style": style, "level": level }))
        .expect("the level is set");
    style
}

/// A **document** with headings in it, and the document's id.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let one = a_heading(session, document, "the first heading", 1);
    let two = a_heading(session, document, "the second heading", 2);
    let deep = a_heading(session, document, "the deep heading", 4);
    // A **paragraph** that is not a heading.
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a plain paragraph" }))
        .expect("the paragraph is added");
    let _plain = outcome.data["id"].as_u64().expect("the paragraph has an id");
    for (style, text) in [(one, "the first"), (two, "the second"), (deep, "the deep")] {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
            .expect("the paragraph is added");
        let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
        session
            .execute("prose.block.style", &json!({ "block": block, "style": style }))
            .expect("the style is set");
    }
    document
}

/// The **entries** of the table of contents, and the levels that were
/// asked for.
fn contents_of(session: &mut Session, document: u64, levels: Option<u64>) -> Vec<Json> {
    let mut params = json!({ "document": document });
    if let Some(levels) = levels {
        params["levels"] = json!(levels);
    }
    let outcome = session.execute("prose.document.contents", &params).expect("the table is read");
    outcome.data["entries"].as_array().expect("the entries are a list").clone()
}

/// The table of contents is the **headings** of the document, in the
/// order they stand.
#[test]
fn the_table_of_contents_is_the_headings_of_the_document() {
    let mut session = session();
    let document = a_document(&mut session);
    let entries = contents_of(&mut session, document, None);
    assert_eq!(entries.len(), 2, "the two headings of the first three levels: {entries:?}");
    assert_eq!(entries[0]["text"], Json::from("the first"), "the first: {entries:?}");
    assert_eq!(entries[0]["level"], json!(1), "and its level: {entries:?}");
    assert_eq!(entries[1]["text"], Json::from("the second"), "the second: {entries:?}");
    assert_eq!(entries[1]["level"], json!(2), "and its level: {entries:?}");
}

/// The levels of the table are the **tree's**: three of them, one to
/// three, unless the reader asks for others.
#[test]
fn the_levels_of_the_table_are_the_trees() {
    let mut session = session();
    let document = a_document(&mut session);
    let entries = contents_of(&mut session, document, Some(4));
    assert_eq!(entries.len(), 3, "the deep heading is in the table: {entries:?}");
    assert_eq!(entries[2]["text"], Json::from("the deep"), "and it is the third: {entries:?}");
    assert_eq!(entries[2]["level"], json!(4), "and its level: {entries:?}");
}

/// A level that is **not a whole number from one to nine** is
/// refused.
#[test]
fn a_level_that_is_not_a_level_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.document.contents", &json!({ "document": document, "levels": 12 }))
        .expect_err("the level is refused");
    assert!(outcome.to_string().contains("a level is a whole number"), "the refusal says why: {outcome}");
}

/// The **number** a heading shows is in the table, and a heading in a
/// list of bullets has none: the tree's own rule.
#[test]
fn the_number_of_a_heading_is_in_the_table() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_heading(&mut session, document, "a numbered heading", 1);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a numbered one" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.style", &json!({ "block": block, "style": style }))
        .expect("the style is set");
    let outcome = session
        .execute("prose.list.add", &json!({ "document": document, "kind": "numbered" }))
        .expect("the list is added");
    let list = outcome.data["id"].as_u64().expect("the list has an id");
    session
        .execute("prose.block.numbering", &json!({ "block": block, "list": list }))
        .expect("the block is numbered");
    let entries = contents_of(&mut session, document, None);
    let entry = entries.last().expect("the heading is in the table");
    assert_eq!(entry["number"], json!("1."), "the number is in the table: {entries:?}");
}
