//! The **bookmarks** of a document.
//!
//! A bookmark is a name for a place in a document, and a link may
//! point at it.

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

/// The **bookmarks** of a document, as the answer holds them.
fn bookmarks_of(session: &mut Session, document: u64) -> Vec<Json> {
    let outcome = session
        .execute("prose.document.bookmarks", &json!({ "document": document }))
        .expect("the bookmarks are read");
    outcome.data["bookmarks"].as_array().expect("the bookmarks are a list").clone()
}

/// A block may carry a **bookmark**, and the bookmarks of a document
/// are derived.
#[test]
fn a_block_may_carry_a_bookmark() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.block.bookmark", &json!({ "block": paragraph, "name": "the_intro" }))
        .expect("the bookmark is put");
    let bookmarks = bookmarks_of(&mut session, document);
    assert_eq!(bookmarks.len(), 1, "there is one bookmark");
    assert_eq!(bookmarks[0]["block"], json!(paragraph), "and it is on the block");
    assert_eq!(bookmarks[0]["name"], Json::String("the_intro".to_string()), "with its name");
}

/// A name that a bookmark may **not** have is refused.
#[test]
fn a_name_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": paragraph, "name": "1st" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("is not a name a bookmark may have"), "the refusal says why: {error}");
}

/// A name that another block already carries is refused: two places
/// with one name are one place.
#[test]
fn a_name_that_is_already_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "the first words");
    let second = a_paragraph(&mut session, document, "the second words");
    session
        .execute("prose.block.bookmark", &json!({ "block": first, "name": "the_intro" }))
        .expect("the bookmark is put");
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": second, "name": "the_intro" }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("is already in the document"), "the refusal says why: {error}");
}

/// A bookmark may be **taken away**, and a block with no bookmark has
/// none to take away.
#[test]
fn a_bookmark_may_be_taken_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.block.bookmark", &json!({ "block": paragraph, "name": "the_intro" }))
        .expect("the bookmark is put");
    session
        .execute("prose.block.bookmark.delete", &json!({ "block": paragraph }))
        .expect("the bookmark is taken away");
    assert!(bookmarks_of(&mut session, document).is_empty(), "there is no bookmark");
    let error = session
        .execute("prose.block.bookmark.delete", &json!({ "block": paragraph }))
        .expect_err("the bookmark is refused");
    assert!(error.to_string().contains("has no bookmark"), "the refusal says why: {error}");
}

/// A link to a **place in this document** may be opened, and a link to
/// a place that is not there may not.
#[test]
fn a_link_to_a_place_may_be_opened() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.block.bookmark", &json!({ "block": paragraph, "name": "the_intro" }))
        .expect("the bookmark is put");
    let other = a_paragraph(&mut session, document, "the second words");
    session
        .execute("prose.block.runs", &json!({ "block": other, "runs": [{ "len": 3, "link": "#the_intro" }] }))
        .expect("the runs are set");
    let outcome =
        session.execute("prose.block.links", &json!({ "block": other })).expect("the links are read");
    let links = outcome.data["links"].as_array().expect("the links are a list");
    assert_eq!(links[0]["may_be_opened"], json!(true), "the link to the place may be opened");
    session
        .execute("prose.block.runs", &json!({ "block": other, "runs": [{ "len": 3, "link": "#nowhere" }] }))
        .expect("the runs are set");
    let outcome =
        session.execute("prose.block.links", &json!({ "block": other })).expect("the links are read");
    let links = outcome.data["links"].as_array().expect("the links are a list");
    assert_eq!(links[0]["may_be_opened"], json!(false), "the link to nowhere may not");
    let reason = links[0]["reason"].as_str().expect("there is a reason").to_string();
    assert!(reason.contains("no bookmark `nowhere`"), "the reason says so: {reason}");
}

/// A block that is **not there** is refused, and so is a node that is
/// not a block.
#[test]
fn a_block_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": 999, "name": "the_intro" }))
        .expect_err("the block is refused");
    assert!(error.to_string().contains("there is no block"), "the refusal says why: {error}");
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": document, "name": "the_intro" }))
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
}
