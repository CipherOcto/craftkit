//! The **running heads** of a document: the header and the footer.

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

/// A **document**, and its id.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the report" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// The **header** of the document, as the command reads it.
fn header_of(session: &mut Session, document: u64) -> Json {
    let outcome = session
        .execute("prose.document.header.read", &json!({ "document": document }))
        .expect("the header is read");
    outcome.data.clone()
}

/// The **footer** of the document, as the command reads it.
fn footer_of(session: &mut Session, document: u64) -> Json {
    let outcome = session
        .execute("prose.document.footer.read", &json!({ "document": document }))
        .expect("the footer is read");
    outcome.data.clone()
}

/// A document may carry a **header**, and it is read back.
#[test]
fn a_document_may_carry_a_header() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.header", &json!({ "document": document, "header": "the report" }))
        .expect("the header is set");
    let header = header_of(&mut session, document);
    assert_eq!(header["header"], json!("the report"), "the header is read back");
}

/// An **empty** header takes the header away.
#[test]
fn an_empty_header_takes_the_header_away() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.header", &json!({ "document": document, "header": "the report" }))
        .expect("the header is set");
    session
        .execute("prose.document.header", &json!({ "document": document, "header": "" }))
        .expect("the header is taken away");
    let header = header_of(&mut session, document);
    assert_eq!(header["header"], Json::Null, "the header is gone");
}

/// A document may carry a **footer**, and it is read back.
#[test]
fn a_document_may_carry_a_footer() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.document.footer", &json!({ "document": document, "footer": "page one" }))
        .expect("the footer is set");
    let footer = footer_of(&mut session, document);
    assert_eq!(footer["footer"], json!("page one"), "the footer is read back");
}

/// A node that is **not a document** may not have a header.
#[test]
fn a_node_that_is_not_a_document_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome =
        session.execute("prose.document.header", &json!({ "document": document, "header": "the report" }));
    assert!(outcome.is_ok(), "a document may have a header");
    // A **style** is not a document.
    let style = session
        .execute("prose.style.add", &json!({ "document": document, "name": "body" }))
        .expect("the style is added");
    let style = style.data["id"].as_u64().expect("the style has an id");
    let outcome =
        session.execute("prose.document.header", &json!({ "document": style, "header": "the report" }));
    assert!(outcome.is_err(), "a style is not a document and the command refuses it");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("not a document"), "and it says why: {message}");
}
