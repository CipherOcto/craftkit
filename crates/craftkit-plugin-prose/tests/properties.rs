//! The **properties** of a document.
//!
//! Who wrote it, what it is about, the words a search looks for, and
//! the custom properties a firm puts in it.

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

/// The **properties** of a document, as the command reads them.
fn properties_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("prose.document.properties", &json!({ "document": document }))
        .expect("the properties are read");
    outcome.data.clone()
}

/// The properties of a document are **said**, and the ones that are
/// not named keep what they were.
#[test]
fn a_document_says_what_it_is() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "author": "Ada", "subject": "a scaffold" }),
        )
        .expect("the properties are set");
    assert_eq!(outcome.data["properties"]["author"], json!("Ada"), "the author is said");
    assert_eq!(outcome.data["properties"]["subject"], json!("a scaffold"), "and so is the subject");
    assert!(outcome.data["properties"]["keywords"].is_null(), "and the keywords are nothing");
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "author": "Ada", "category": "tools" }),
        )
        .expect("the properties are set");
    let properties = properties_of(&mut session, document);
    assert_eq!(properties["properties"]["author"], json!("Ada"), "the author is kept");
    assert_eq!(properties["properties"]["subject"], json!("a scaffold"), "and so is the subject");
    assert_eq!(properties["properties"]["category"], json!("tools"), "and the category is said");
}

/// An **empty** value takes the property away.
#[test]
fn an_empty_value_takes_the_property_away() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "author": "Ada", "subject": "a scaffold" }),
        )
        .expect("the properties are set");
    session
        .execute("prose.document.properties", &json!({ "document": document, "author": "  " }))
        .expect("the properties are set");
    let properties = properties_of(&mut session, document);
    assert!(properties["properties"]["author"].is_null(), "the author is gone: {properties}");
    assert_eq!(properties["properties"]["subject"], json!("a scaffold"), "and the subject is kept");
}

/// A **custom** property is a name and a value, and it is taken away
/// by a null and by an empty value.
#[test]
fn a_custom_property_is_a_name_and_a_value() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "custom": { "Project": "Atlas" } }),
        )
        .expect("the properties are set");
    let properties = properties_of(&mut session, document);
    assert_eq!(properties["custom"]["Project"], json!("Atlas"), "the custom property is there");
    session
        .execute("prose.document.properties", &json!({ "document": document, "custom": { "Project": null } }))
        .expect("the properties are set");
    let properties = properties_of(&mut session, document);
    assert!(properties["custom"]["Project"].is_null(), "and it is taken away: {properties}");
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "custom": { "Project": "Atlas" } }),
        )
        .expect("the properties are set");
    session
        .execute("prose.document.properties", &json!({ "document": document, "custom": { "Project": "" } }))
        .expect("the properties are set");
    let properties = properties_of(&mut session, document);
    assert!(properties["custom"]["Project"].is_null(), "and an empty value takes it away too");
}

/// A name is matched **without regard to case**, and it keeps the
/// spelling it has.
#[test]
fn a_name_is_matched_without_regard_to_case() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "custom": { "Project": "Atlas" } }),
        )
        .expect("the properties are set");
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "custom": { "project": "Beacon" } }),
        )
        .expect("the properties are set");
    let properties = properties_of(&mut session, document);
    assert_eq!(properties["custom"]["Project"], json!("Beacon"), "the value is the new one");
    assert!(properties["custom"]["project"].is_null(), "and there is no second one: {properties}");
}

/// A custom property that is **not one** is refused: an empty name, a
/// name that is too long, a value that is not a text, and a value
/// that is too long.
#[test]
fn a_custom_property_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "custom": { "  ": "Atlas" } }))
        .expect_err("the empty name is refused");
    assert!(error.to_string().contains("has a name"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "custom": { "X".repeat(300): "Atlas" } }),
        )
        .expect_err("the long name is refused");
    assert!(error.to_string().contains("at most"), "the refusal says why: {error}");
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "custom": { "Project": 42 } }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("a text or nothing"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "custom": { "Project": "x".repeat(2 * 1024 * 1024) } }),
        )
        .expect_err("the long value is refused");
    assert!(error.to_string().contains("bytes"), "the refusal says why: {error}");
    let properties = properties_of(&mut session, document);
    assert!(
        properties["custom"].as_object().map(|custom| custom.is_empty()).unwrap_or(true),
        "and nothing was kept"
    );
}

/// A node that is **not a document** may not have properties.
#[test]
fn a_node_that_is_not_a_document_may_not_have_properties() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "hello" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let error = session
        .execute("prose.document.properties", &json!({ "document": block, "author": "Ada" }))
        .expect_err("the paragraph is not a document");
    assert!(error.to_string().contains("not a document"), "the refusal says why: {error}");
}

/// The properties of a document are **derived**: they are read out of
/// the document, and they are in its own reading.
#[test]
fn the_properties_are_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "author": "Ada", "custom": { "Project": "Atlas" } }),
        )
        .expect("the properties are set");
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == document && item.kind == "prose.document")
        .expect("the document is in the plan");
    assert_eq!(item.value["author"], json!("Ada"), "the document says who wrote it");
    assert_eq!(item.value["custom"]["Project"], json!("Atlas"), "and the custom properties are there");
}
