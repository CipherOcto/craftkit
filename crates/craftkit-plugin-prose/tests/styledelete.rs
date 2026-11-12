//! The **deleting** of a style.

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

/// A **style** in a document, and its id.
fn a_style(session: &mut Session, document: u64, name: &str) -> u64 {
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": name }))
        .expect("the style is added");
    outcome.data["id"].as_u64().expect("the style has an id")
}

/// A **paragraph** in a document, and its id.
fn a_paragraph(session: &mut Session, document: u64, text: &str) -> u64 {
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// A style is **taken away**.
#[test]
fn a_style_is_taken_away() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the style");
    let outcome =
        session.execute("prose.style.delete", &json!({ "style": style })).expect("the style is deleted");
    assert_eq!(outcome.data["name"], json!("the style"), "the style that was deleted is named");
    let node = session.document().node(style);
    assert!(node.is_none(), "and the style is gone");
}

/// The blocks that were in the style are **left as they are**.
#[test]
fn the_blocks_that_were_in_the_style_are_left_as_they_are() {
    let mut session = session();
    let document = a_document(&mut session);
    let style = a_style(&mut session, document, "the style");
    let block = a_paragraph(&mut session, document, "the words");
    session
        .execute("prose.block.style", &json!({ "block": block, "style": style }))
        .expect("the block is put in the style");
    let outcome =
        session.execute("prose.style.delete", &json!({ "style": style })).expect("the style is deleted");
    assert_eq!(outcome.data["blocks"], json!(1), "one block was in the style");
    let outcome =
        session.execute("prose.block.character", &json!({ "block": block })).expect("the block is read");
    assert_eq!(outcome.data["style"], json!(null), "and the block has no style now");
}

/// A style that another style is **based on** may not be deleted.
#[test]
fn a_style_that_another_is_based_on_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let base = a_style(&mut session, document, "the base");
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the other", "based_on": base }))
        .expect("the style is added");
    let other = outcome.data["id"].as_u64().expect("the style has an id");
    let outcome = session.execute("prose.style.delete", &json!({ "style": base }));
    assert!(outcome.is_err(), "the other style would lose its base");
    assert!(session.document().node(other).is_some(), "and the other style is still there");
}

/// A node that is **not a style** is refused.
#[test]
fn a_node_that_is_not_a_style_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let block = a_paragraph(&mut session, document, "the words");
    let outcome = session.execute("prose.style.delete", &json!({ "style": block }));
    assert!(outcome.is_err(), "a paragraph is not a style");
}

/// A style that is **not there** is refused.
#[test]
fn a_style_that_is_not_there_is_refused() {
    let mut session = session();
    let _document = a_document(&mut session);
    let outcome = session.execute("prose.style.delete", &json!({ "style": 9999 }));
    assert!(outcome.is_err(), "there is no such style");
}
