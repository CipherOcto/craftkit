//! The **background** of a page.
//!
//! The colour a page is painted with, and the watermark it carries.

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

/// The **page** of a document, as the command reads it.
fn page_of(session: &mut Session, document: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("prose.page.get", &json!({ "document": document })).expect("the page is read");
    outcome.data.clone()
}

/// The page is **painted** with a colour, and the colour is read back.
#[test]
fn the_page_is_painted_with_a_colour() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.page.background", &json!({ "document": document, "colour": "#ffcc00" }))
        .expect("the page is painted");
    assert_eq!(outcome.data["colour"], json!("#ffcc00"), "the colour is the one that was given");
    let page = page_of(&mut session, document);
    assert_eq!(page["colour"], json!("#ffcc00"), "and the page is read back with it");
}

/// A colour that is **not a colour** is refused.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.background", &json!({ "document": document, "colour": "orange" }))
        .expect_err("the colour is refused");
    assert!(error.to_string().contains('#'), "the refusal says why: {error}");
    let page = page_of(&mut session, document);
    assert!(page["colour"].is_null(), "and the page is not painted: {page}");
}

/// An **empty** colour takes the colour of the page away.
#[test]
fn an_empty_colour_takes_the_colour_away() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.background", &json!({ "document": document, "colour": "#ffcc00" }))
        .expect("the page is painted");
    session
        .execute("prose.page.background", &json!({ "document": document, "colour": "" }))
        .expect("the colour is taken away");
    let page = page_of(&mut session, document);
    assert!(page["colour"].is_null(), "the page is not painted: {page}");
}

/// A **watermark** is put on the page, and it is the tree's: diagonal
/// and grey.
#[test]
fn a_watermark_is_put_on_the_page() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.page.watermark", &json!({ "document": document, "text": "DRAFT" }))
        .expect("the watermark is put");
    assert_eq!(outcome.data["watermark"]["text"], json!("DRAFT"), "the watermark says what it was told");
    assert_eq!(outcome.data["watermark"]["diagonal"], json!(true), "and it runs across the page");
    assert_eq!(outcome.data["watermark"]["colour"], json!("#c0c0c0"), "and it is the tree's grey");
    let page = page_of(&mut session, document);
    assert_eq!(page["watermark"]["text"], json!("DRAFT"), "and the page is read back with it");
}

/// A watermark that is given a **new text** and no colour keeps the
/// colour it has.
#[test]
fn a_watermark_keeps_the_colour_it_has() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.page.watermark",
            &json!({ "document": document, "text": "DRAFT", "colour": "#112233", "diagonal": false }),
        )
        .expect("the watermark is put");
    session
        .execute("prose.page.watermark", &json!({ "document": document, "text": "COPY" }))
        .expect("the watermark is changed");
    let page = page_of(&mut session, document);
    assert_eq!(page["watermark"]["text"], json!("COPY"), "the watermark says the new text");
    assert_eq!(page["watermark"]["colour"], json!("#112233"), "and it keeps the colour it had");
    assert_eq!(page["watermark"]["diagonal"], json!(false), "and the way it runs");
}

/// An **empty** text takes the watermark away.
#[test]
fn an_empty_text_takes_the_watermark_away() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.watermark", &json!({ "document": document, "text": "DRAFT" }))
        .expect("the watermark is put");
    session
        .execute("prose.page.watermark", &json!({ "document": document, "text": "  " }))
        .expect("the watermark is taken away");
    let page = page_of(&mut session, document);
    assert!(page["watermark"].is_null(), "the document has no watermark: {page}");
}

/// A watermark whose text is **too long** is refused.
#[test]
fn a_watermark_that_says_too_much_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let text = "x".repeat(201);
    let error = session
        .execute("prose.page.watermark", &json!({ "document": document, "text": text }))
        .expect_err("the watermark is refused");
    assert!(error.to_string().contains("200"), "the refusal says why: {error}");
    let page = page_of(&mut session, document);
    assert!(page["watermark"].is_null(), "and there is no watermark: {page}");
}
