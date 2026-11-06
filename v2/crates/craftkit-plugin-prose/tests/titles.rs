//! The **title** of a document.

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

/// The **title** of a document, as the evaluator says it.
fn title_of(session: &Session, document: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == document)
        .and_then(|item| item.value.get("title").cloned())
        .unwrap_or(Json::Null)
}

/// The **rules** the document breaks.
fn rules_of(session: &mut Session, document: u64) -> Vec<String> {
    let outcome = session
        .execute("prose.accessibility", &json!({ "document": document }))
        .expect("the document is checked");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .map(|issue| issue["rule"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// A document that says nothing has **no** title.
#[test]
fn a_document_that_says_nothing_has_no_title() {
    let mut session = session();
    let document = a_document(&mut session);
    assert_eq!(title_of(&session, document), Json::Null, "the document says nothing");
}

/// A document **says** what it is.
#[test]
fn a_document_says_what_it_is() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "the red door" }))
        .expect("the title is put");
    assert_eq!(title_of(&session, document), json!("the red door"), "the document says what it is");
}

/// The title is **changed**, not added twice.
#[test]
fn the_title_is_changed_and_not_added_twice() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "the red door" }))
        .expect("the title is put");
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "the blue window" }))
        .expect("the title is put again");
    assert_eq!(title_of(&session, document), json!("the blue window"), "the title is the new one");
}

/// An **empty** title takes the title away.
#[test]
fn an_empty_title_takes_the_title_away() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "the red door" }))
        .expect("the title is put");
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "" }))
        .expect("the title is taken away");
    assert_eq!(title_of(&session, document), Json::Null, "the document says nothing again");
}

/// A document with no title is a **tip**.
#[test]
fn a_document_with_no_title_is_a_tip() {
    let mut session = session();
    let document = a_document(&mut session);
    let rules = rules_of(&mut session, document);
    assert!(rules.contains(&"docTitle".to_string()), "the rule is broken: {rules:?}");
}

/// A document with a title breaks **nothing**.
#[test]
fn a_document_with_a_title_breaks_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.title", &json!({ "document": document, "title": "the red door" }))
        .expect("the title is put");
    let rules = rules_of(&mut session, document);
    assert!(!rules.contains(&"docTitle".to_string()), "the document says what it is: {rules:?}");
}

/// A node that is **not a document** is refused.
#[test]
fn a_node_that_is_not_a_document_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let refused = session
        .execute("prose.document.title", &json!({ "document": paragraph, "title": "the red door" }))
        .expect_err("the title is refused");
    assert!(refused.to_string().contains("not a document"), "the refusal says why: {refused}");
}
