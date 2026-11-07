//! The **revision** of a document.
//!
//! Who touched it last, and how many times it has been revised.

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

/// A **document** to say things about.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A document may say who touched it last and how many times it has
/// been revised.
#[test]
fn a_document_says_who_and_how_many_times() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "last_modified_by": "the editor", "revision": 3 }),
        )
        .expect("the properties are set");
    let properties = outcome.data["properties"].clone();
    assert_eq!(properties["last_modified_by"], json!("the editor"), "the name is kept: {properties:?}");
    assert_eq!(properties["revision"], json!(3), "and the revision");
    // The **document** says what it is, as the evaluator says it.
    let plan = session.evaluate();
    let said = plan
        .items
        .iter()
        .find(|item| item.node == document)
        .map(|item| item.value.clone())
        .expect("the document is evaluated");
    assert_eq!(said["last_modified_by"], json!("the editor"), "the document says who: {said:?}");
    assert_eq!(said["revision"], json!(3), "and how many times");
}

/// A **revision** of zero takes the revision away, and an **empty**
/// name takes the name away.
#[test]
fn nothing_is_nothing() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "last_modified_by": "the editor", "revision": 3 }),
        )
        .expect("the properties are set");
    let outcome = session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "last_modified_by": "  ", "revision": 0 }),
        )
        .expect("the properties are set");
    let properties = outcome.data["properties"].clone();
    assert_eq!(properties["last_modified_by"], json!(null), "the name is gone: {properties:?}");
    assert_eq!(properties["revision"], json!(null), "and the revision");
}

/// A **revision** that is not a whole number is refused.
#[test]
fn a_revision_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "revision": 1.5 }))
        .expect_err("the revision is refused");
    assert!(error.to_string().contains("whole number"), "the refusal says why: {error}");
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "revision": -1 }))
        .expect_err("the revision is refused");
    assert!(error.to_string().contains("whole number"), "the refusal says why: {error}");
}

/// The **revision** is kept when the other properties are set again:
/// a property that is not named keeps what it was.
#[test]
fn a_property_that_is_not_named_keeps_what_it_was() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.properties",
            &json!({ "document": document, "author": "the writer", "revision": 2 }),
        )
        .expect("the properties are set");
    let outcome = session
        .execute("prose.document.properties", &json!({ "document": document, "subject": "the beach" }))
        .expect("the properties are set");
    let properties = outcome.data["properties"].clone();
    assert_eq!(properties["author"], json!("the writer"), "the author is kept: {properties:?}");
    assert_eq!(properties["revision"], json!(2), "and the revision");
}
