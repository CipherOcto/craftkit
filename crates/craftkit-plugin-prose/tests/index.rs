//! The **index** of a document.

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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A **paragraph** in a document, and its id.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// The **index** of a document, as the answer holds it.
fn index_of(session: &mut Session, document: u64) -> Vec<Json> {
    let outcome =
        session.execute("prose.index.read", &json!({ "document": document })).expect("the index is read");
    outcome.data["groups"].as_array().expect("the groups are a list").clone()
}

/// A block may be **marked** with an entry, and the index of a
/// document is derived.
#[test]
fn a_block_may_be_marked_with_an_entry() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.index.mark", &json!({ "block": paragraph, "entry": "Apple" }))
        .expect("the block is marked");
    let groups = index_of(&mut session, document);
    assert_eq!(groups.len(), 1, "there is one group");
    assert_eq!(groups[0]["letter"], json!("A"), "the letter of the entry");
    let entries = groups[0]["entries"].as_array().expect("the entries are a list");
    assert_eq!(entries.len(), 1, "and one entry under it");
    assert_eq!(entries[0]["entry"], json!("Apple"), "the entry is read");
    assert_eq!(entries[0]["blocks"], json!([paragraph]), "and the block it is on is named");
}

/// The entries are gathered under their **letters**, and the letters
/// come first.
#[test]
fn the_entries_are_gathered_under_their_letters() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "the first words");
    let second = a_paragraph(&mut session, document, "the second words");
    let third = a_paragraph(&mut session, document, "the third words");
    let fourth = a_paragraph(&mut session, document, "the fourth words");
    for (block, entry) in [(first, "apricot"), (second, "Apple"), (third, "Banana"), (fourth, "7 Wonders")] {
        session
            .execute("prose.index.mark", &json!({ "block": block, "entry": entry }))
            .expect("the block is marked");
    }
    let groups = index_of(&mut session, document);
    assert_eq!(groups.len(), 3, "there are three groups");
    assert_eq!(groups[0]["letter"], json!("A"), "the letters come first");
    assert_eq!(groups[1]["letter"], json!("B"), "and they are in the alphabet's order");
    assert_eq!(groups[2]["letter"], json!("7"), "and the other characters come after them");
    let entries = groups[0]["entries"].as_array().expect("the entries are a list");
    assert_eq!(entries.len(), 2, "the letter holds two entries");
    assert_eq!(entries[0]["entry"], json!("Apple"), "and they are in the alphabet's order too");
    assert_eq!(entries[1]["entry"], json!("apricot"), "and the second one is the other");
}

/// The same mark **twice** is one mark.
#[test]
fn the_same_mark_twice_is_one_mark() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    for _ in 0..2 {
        session
            .execute("prose.index.mark", &json!({ "block": paragraph, "entry": "Apple" }))
            .expect("the block is marked");
    }
    let groups = index_of(&mut session, document);
    let entries = groups[0]["entries"].as_array().expect("the entries are a list");
    assert_eq!(entries.len(), 1, "the mark is one mark");
    assert_eq!(entries[0]["blocks"], json!([paragraph]), "and it is on the block");
}

/// An entry that says **nothing** takes the marks away.
#[test]
fn an_empty_entry_takes_the_marks_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.index.mark", &json!({ "block": paragraph, "entry": "Apple" }))
        .expect("the block is marked");
    session
        .execute("prose.index.mark", &json!({ "block": paragraph, "entry": "" }))
        .expect("the marks are taken away");
    let groups = index_of(&mut session, document);
    assert_eq!(groups.len(), 0, "the document has no index at all");
}

/// A **subentry** goes under its entry.
#[test]
fn a_subentry_goes_under_its_entry() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.index.mark", &json!({ "block": paragraph, "entry": "Apple", "subentry": "Pippin" }))
        .expect("the block is marked");
    let groups = index_of(&mut session, document);
    let entries = groups[0]["entries"].as_array().expect("the entries are a list");
    assert_eq!(entries.len(), 1, "there is one entry");
    assert_eq!(entries[0]["entry"], json!("Apple"), "the entry is read");
    assert_eq!(entries[0]["subentry"], json!("Pippin"), "and the subentry under it");
    let subentries = entries[0]["subentries"].as_array().expect("the subentries are a list");
    assert_eq!(subentries.len(), 1, "and it is a subentry of its own");
    assert_eq!(subentries[0]["subentry"], json!("Pippin"), "the subentry is read");
    assert_eq!(subentries[0]["blocks"], json!([paragraph]), "and the block it is on is named");
}

/// A node that is **not a document** is refused.
#[test]
fn a_node_that_is_not_a_document_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    let outcome = session.execute("prose.index.read", &json!({ "document": paragraph }));
    assert!(outcome.is_err(), "the paragraph is not a document and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("not a document"), "and it says why: {error}");
}
