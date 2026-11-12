//! The **comment** on a block.
//!
//! A comment is something somebody said about a piece of a document:
//! who said it and what they said.

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

/// The **comment** on a block, as the answer holds it.
fn comment_of(session: &mut Session, block: u64) -> Json {
    let outcome =
        session.execute("prose.comment.get", &json!({ "block": block })).expect("the comment is read");
    outcome.data["comment"].clone()
}

/// A comment may be **put** on a block and **read**.
#[test]
fn a_comment_may_be_put_on_a_block_and_read() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute(
            "prose.comment.set",
            &json!({ "block": paragraph, "author": "Ada", "text": "is this the title?" }),
        )
        .expect("the comment is put");
    let comment = comment_of(&mut session, paragraph);
    assert_eq!(comment["author"], Json::String("Ada".to_string()), "the author is said");
    assert_eq!(comment["text"], Json::String("is this the title?".to_string()), "and the words");
}

/// A block with **no comment** has no comment, and that is said as
/// nothing.
#[test]
fn a_block_with_no_comment_has_no_comment() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    assert_eq!(comment_of(&mut session, paragraph), Json::Null, "there is no comment");
}

/// A comment put on a block again **replaces** the one that was there.
#[test]
fn a_comment_put_again_replaces_the_one_that_was_there() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.comment.set", &json!({ "block": paragraph, "author": "Ada", "text": "first" }))
        .expect("the comment is put");
    session
        .execute("prose.comment.set", &json!({ "block": paragraph, "author": "Grace", "text": "second" }))
        .expect("the comment is put again");
    let comment = comment_of(&mut session, paragraph);
    assert_eq!(comment["author"], Json::String("Grace".to_string()), "the new author is said");
    assert_eq!(comment["text"], Json::String("second".to_string()), "and the new words");
}

/// A comment may be **taken away**.
#[test]
fn a_comment_may_be_taken_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.comment.set", &json!({ "block": paragraph, "author": "Ada", "text": "why?" }))
        .expect("the comment is put");
    session
        .execute("prose.comment.delete", &json!({ "block": paragraph }))
        .expect("the comment is taken away");
    assert_eq!(comment_of(&mut session, paragraph), Json::Null, "the comment is gone");
}

/// A comment with **no author** is refused: a comment nobody made is
/// not a comment.
#[test]
fn a_comment_with_no_author_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    let error = session
        .execute("prose.comment.set", &json!({ "block": paragraph, "author": "  ", "text": "why?" }))
        .expect_err("the comment is refused");
    assert!(error.to_string().contains("a comment has an author"), "the refusal says why: {error}");
}

/// A comment that is **taken away** that is not there is refused.
#[test]
fn a_comment_that_is_not_there_may_not_be_taken_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    let error = session
        .execute("prose.comment.delete", &json!({ "block": paragraph }))
        .expect_err("the comment is refused");
    assert!(error.to_string().contains("has no comment"), "the refusal says why: {error}");
}

/// The comment goes with the **block**: a block that is taken away
/// takes its comment with it.
#[test]
fn the_comment_goes_with_the_block() {
    let mut session = session();
    let document = a_document(&mut session);
    let paragraph = a_paragraph(&mut session, document, "the first words");
    session
        .execute("prose.comment.set", &json!({ "block": paragraph, "author": "Ada", "text": "why?" }))
        .expect("the comment is put");
    session.execute("prose.block.remove", &json!({ "block": paragraph })).expect("the block is taken away");
    let node = session.document().node(document).expect("the document is there");
    let comments = node.data["comments"].clone();
    assert_eq!(comments.get(paragraph.to_string().as_str()), None, "the comment went with the block");
}

/// A node that is **not there** is refused, and so is a node that is
/// not a block.
#[test]
fn a_node_that_is_not_a_block_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error =
        session.execute("prose.comment.get", &json!({ "block": 999 })).expect_err("the block is refused");
    assert!(error.to_string().contains("there is no block"), "the refusal says why: {error}");
    let error = session
        .execute("prose.comment.get", &json!({ "block": document }))
        .expect_err("the document is not a block");
    assert!(error.to_string().contains("not a block"), "the refusal says why: {error}");
}
