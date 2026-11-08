//! The **parts** of a source.
//!
//! The place, the publisher, the journal, the volume, the pages and
//! the address are the parts the tree's entries are written from.

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

/// A **document** with a source in it.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "title": "The Book", "year": "2020" }),
        )
        .expect("the source is said");
    document
}

/// The **source** of a document, as it is read.
fn source_of(session: &mut Session, document: u64) -> Json {
    let outcome = session
        .execute("prose.document.source.read", &json!({ "document": document }))
        .expect("the sources are read");
    outcome.data["sources"][0].clone()
}

/// The parts of a source are **said**.
#[test]
fn the_parts_of_a_source_are_said() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.source.set",
            &json!({
                "document": document,
                "tag": "Riv2020",
                "publisher": "The Press",
                "city": "Oxford",
                "journal": "The Journal",
                "volume": "12",
                "pages": "34-56",
                "url": "https://example.invalid/book",
            }),
        )
        .expect("the parts are said");
    let source = source_of(&mut session, document);
    assert_eq!(source["publisher"], json!("The Press"), "the publisher: {source}");
    assert_eq!(source["city"], json!("Oxford"), "the place: {source}");
    assert_eq!(source["journal"], json!("The Journal"), "the journal: {source}");
    assert_eq!(source["volume"], json!("12"), "the volume: {source}");
    assert_eq!(source["pages"], json!("34-56"), "the pages: {source}");
    assert_eq!(source["url"], json!("https://example.invalid/book"), "the address: {source}");
}

/// A part that is **not said** keeps what it was.
#[test]
fn a_part_that_is_not_said_keeps_what_it_was() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "publisher": "The Press" }),
        )
        .expect("the publisher is said");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "title": "The Other Book" }),
        )
        .expect("the title is said");
    let source = source_of(&mut session, document);
    assert_eq!(source["publisher"], json!("The Press"), "the publisher is still there: {source}");
    assert_eq!(source["title"], json!("The Other Book"), "and the title changed: {source}");
}

/// A part that is said **empty** is not there.
#[test]
fn a_part_that_is_said_empty_is_not_there() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "publisher": "The Press" }),
        )
        .expect("the publisher is said");
    session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "publisher": "" }),
        )
        .expect("the publisher is taken away");
    let source = source_of(&mut session, document);
    assert!(source.get("publisher").is_none(), "the publisher is not there: {source}");
}

/// A part that is **too long** is refused, and the refusal says how
/// long a part may be.
#[test]
fn a_part_that_is_too_long_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let long = "x".repeat(1025);
    let error = session
        .execute(
            "prose.document.source.set",
            &json!({ "document": document, "tag": "Riv2020", "publisher": long }),
        )
        .expect_err("the part is refused");
    let message = error.to_string();
    assert!(message.contains("1024"), "the refusal says the bound: {message}");
}
