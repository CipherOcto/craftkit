//! The **styles** of a document.

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

/// The **styles** of a document, as the answer holds them.
fn styles_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("prose.style.list", &json!({ "document": document })).expect("the styles are read");
    outcome.data["styles"].as_array().expect("the styles are a list").clone()
}

/// The styles of a document are **read**.
#[test]
fn the_styles_of_a_document_are_read() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.style.add", &json!({ "document": document, "name": "the style" }))
        .expect("the style is added");
    let styles = styles_of(&mut session, document);
    assert_eq!(styles.len(), 1, "there is one style");
    assert_eq!(styles[0]["name"], json!("the style"), "and it is the one that was added");
    assert_eq!(styles[0]["kind"], json!("paragraph"), "and it applies to paragraphs");
}

/// A style says the style it is **based on**.
#[test]
fn a_style_says_the_style_it_is_based_on() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the base" }))
        .expect("the base is added");
    let base = outcome.data["id"].as_u64().expect("the style has an id");
    session
        .execute("prose.style.add", &json!({ "document": document, "name": "the other", "based_on": base }))
        .expect("the style is added");
    let styles = styles_of(&mut session, document);
    let other = styles.iter().find(|style| style["name"] == json!("the other")).expect("the style is there");
    assert_eq!(other["based_on"]["name"], json!("the base"), "the base is named");
}

/// The **blocks** that are in a style are named.
#[test]
fn the_blocks_that_are_in_a_style_are_named() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the style" }))
        .expect("the style is added");
    let style = outcome.data["id"].as_u64().expect("the style has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the words" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.style", &json!({ "block": block, "style": style }))
        .expect("the block is put in the style");
    let styles = styles_of(&mut session, document);
    assert_eq!(styles[0]["blocks"], json!([block]), "the block is in the style");
}

/// A node that is **not a document** is refused.
#[test]
fn a_node_that_is_not_a_document_is_refused() {
    let mut session = session();
    let _document = a_document(&mut session);
    let outcome = session.execute("prose.style.list", &json!({ "document": 9999 }));
    assert!(outcome.is_err(), "there is no such document");
}
