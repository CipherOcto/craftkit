//! The **cross-reference** of a document: a block that stands for the
//! text of a bookmarked block.

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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A **paragraph** in a document.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// A **bookmark** on a block.
fn a_bookmark(session: &mut Session, block: u64, name: &str) {
    session
        .execute("prose.block.bookmark", &json!({ "block": block, "name": name }))
        .expect("the bookmark is set");
}

/// The **text** a block shows.
fn text_of(session: &mut Session, block: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.block.reference.read", &json!({ "block": block })).expect("the text is read");
    outcome.data["text"].clone()
}

/// **A reference may be added, and it shows the text it points at.**
#[test]
fn a_reference_may_be_added() {
    let mut session = session();
    let document = a_document(&mut session);
    let target = a_paragraph(&mut session, document, "the first text");
    a_bookmark(&mut session, target, "here");
    let outcome = session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    assert_eq!(text_of(&mut session, block), json!("the first text"));
}

/// **The text of a reference is derived: it follows the block it
/// points at.**
#[test]
fn the_text_of_a_reference_is_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    let target = a_paragraph(&mut session, document, "the first text");
    a_bookmark(&mut session, target, "here");
    let outcome = session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    session
        .execute("prose.block.paragraph.text", &json!({ "block": target, "text": "another text" }))
        .expect("the text is changed");
    assert_eq!(text_of(&mut session, block), json!("another text"), "the reference follows");
}

/// **The blocks of the document show the derived text.**
#[test]
fn the_blocks_show_the_derived_text() {
    let mut session = session();
    let document = a_document(&mut session);
    let target = a_paragraph(&mut session, document, "the first text");
    a_bookmark(&mut session, target, "here");
    session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let plan = session.evaluate();
    let entry = plan
        .items
        .iter()
        .find(|item| item.node == document && item.kind == "prose.blocks")
        .expect("the document is in the plan");
    let blocks = entry.value["blocks"].as_array().expect("the blocks are a list");
    let reference = blocks
        .iter()
        .find(|block| block["kind"] == json!("prose.block.reference"))
        .expect("the reference is there");
    assert_eq!(reference["text"], json!("the first text"));
}

/// **A bookmark that is not there is refused.**
#[test]
fn a_bookmark_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let target = a_paragraph(&mut session, document, "the first text");
    a_bookmark(&mut session, target, "here");
    let outcome =
        session.execute("prose.block.reference.add", &json!({ "document": document, "target": "there" }));
    assert!(outcome.is_err(), "a bookmark that is not there is refused");
}

/// **A reference may be pointed at another bookmark.**
#[test]
fn a_reference_may_be_pointed_at_another_bookmark() {
    let mut session = session();
    let document = a_document(&mut session);
    let first = a_paragraph(&mut session, document, "the first text");
    a_bookmark(&mut session, first, "here");
    let second = a_paragraph(&mut session, document, "the second text");
    a_bookmark(&mut session, second, "there");
    let outcome = session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    session
        .execute("prose.block.reference.set", &json!({ "block": block, "target": "there" }))
        .expect("the reference is pointed again");
    assert_eq!(text_of(&mut session, block), json!("the second text"));
}

/// **A reference to a bookmark that is not there is refused.**
#[test]
fn a_reference_to_a_bookmark_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let target = a_paragraph(&mut session, document, "the first text");
    a_bookmark(&mut session, target, "here");
    let outcome = session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    let outcome =
        session.execute("prose.block.reference.set", &json!({ "block": block, "target": "nowhere" }));
    assert!(outcome.is_err(), "a bookmark that is not there is refused");
}

/// **A block with no text shows nothing.**
#[test]
fn a_block_with_no_text_shows_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    let target = a_paragraph(&mut session, document, "");
    a_bookmark(&mut session, target, "here");
    let outcome = session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    let text = text_of(&mut session, block);
    assert!(text.is_null() || text == json!(""), "a block with no text shows nothing: {text}");
}

/// **A block that is not a reference may not be pointed again.**
#[test]
fn a_block_that_is_not_a_reference_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let block = a_paragraph(&mut session, document, "the text");
    let outcome = session.execute("prose.block.reference.set", &json!({ "block": block, "target": "here" }));
    assert!(outcome.is_err(), "a paragraph is not a reference");
}
