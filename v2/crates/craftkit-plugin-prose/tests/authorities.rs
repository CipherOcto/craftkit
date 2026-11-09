//! The **table of authorities** of a document.
//!
//! An authority is marked on a block, and the table of the authorities
//! is derived from the marks. The rules are the tree's own.

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

/// A **document** with **two** blocks in it, and their ids.
fn a_document(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let mut blocks = Vec::new();
    for text in ["the first paragraph", "the second paragraph"] {
        let outcome = session
            .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
            .expect("the block is added");
        blocks.push(outcome.data["id"].as_u64().expect("the block has an id"));
    }
    (document, blocks[0], blocks[1])
}

/// The **authorities** of a document, as the answer holds them.
fn authorities_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("prose.document.authorities", &json!({ "document": document }))
        .expect("the authorities are read");
    outcome.data["authorities"].as_array().expect("the authorities are a list").clone()
}

/// A block is **marked** with an authority, and the table of the
/// authorities of the document is derived from the marks.
#[test]
fn a_block_is_marked_with_an_authority() {
    let mut session = session();
    let (document, first, _second) = a_document(&mut session);
    let outcome = session
        .execute("prose.authority.mark", &json!({ "block": first, "entry": "Smith v. Jones, 42 U.S. 1" }))
        .expect("the block is marked");
    assert_eq!(outcome.data["entry"], json!("Smith v. Jones, 42 U.S. 1"), "the entry is kept: {outcome:?}");
    let authorities = authorities_of(&mut session, document);
    assert_eq!(authorities.len(), 1, "there is one authority: {authorities:?}");
    assert_eq!(
        authorities[0]["entry"],
        json!("Smith v. Jones, 42 U.S. 1"),
        "and it is the entry: {authorities:?}"
    );
    assert_eq!(authorities[0]["blocks"], json!([first]), "on the block it is on: {authorities:?}");
}

/// The **quotes** are taken out of the entry, and an entry that says
/// nothing is refused.
#[test]
fn the_entry_is_the_tree_s_own() {
    let mut session = session();
    let (document, first, _second) = a_document(&mut session);
    session
        .execute("prose.authority.mark", &json!({ "block": first, "entry": "\"The Case\"" }))
        .expect("the block is marked");
    let authorities = authorities_of(&mut session, document);
    assert_eq!(authorities[0]["entry"], json!("The Case"), "the quotes are gone: {authorities:?}");
    let error = session
        .execute("prose.authority.mark", &json!({ "block": first, "entry": "  " }))
        .expect_err("the entry is refused");
    assert!(error.to_string().contains("an authority says what it is"), "it says why: {error}");
}

/// The **same entry twice is one entry**, and the blocks it is marked
/// on are listed once each.
#[test]
fn the_same_entry_twice_is_one_entry() {
    let mut session = session();
    let (document, first, second) = a_document(&mut session);
    for block in [first, second] {
        session
            .execute("prose.authority.mark", &json!({ "block": block, "entry": "The Case" }))
            .expect("the block is marked");
    }
    // The same entry on the same block again is one mark.
    let outcome = session
        .execute("prose.authority.mark", &json!({ "block": first, "entry": "The Case" }))
        .expect("the block is marked");
    assert_eq!(outcome.data["added"], json!(false), "it is not marked twice: {outcome:?}");
    let authorities = authorities_of(&mut session, document);
    assert_eq!(authorities.len(), 1, "there is one authority: {authorities:?}");
    assert_eq!(authorities[0]["blocks"], json!([first, second]), "on both blocks: {authorities:?}");
}

/// The entries are **sorted**, as the tree sorts them.
#[test]
fn the_entries_are_sorted() {
    let mut session = session();
    let (document, first, second) = a_document(&mut session);
    session
        .execute("prose.authority.mark", &json!({ "block": first, "entry": "Zebra v. Smith" }))
        .expect("the block is marked");
    session
        .execute("prose.authority.mark", &json!({ "block": second, "entry": "adams v. Baker" }))
        .expect("the block is marked");
    let authorities = authorities_of(&mut session, document);
    assert_eq!(authorities.len(), 2, "there are two authorities: {authorities:?}");
    assert_eq!(authorities[0]["entry"], json!("adams v. Baker"), "the first comes first: {authorities:?}");
    assert_eq!(authorities[1]["entry"], json!("Zebra v. Smith"), "and the second: {authorities:?}");
}
