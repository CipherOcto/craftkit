//! The **page** of a document.
//!
//! A document is printed on pages, and a page has a size and margins.

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

/// The **page** of a document, as the answer holds it.
fn page_of(session: &mut Session, document: u64) -> Json {
    let outcome =
        session.execute("prose.page.get", &json!({ "document": document })).expect("the page is read");
    outcome.data["page"].clone()
}

/// A document has the **tree's default page**: US Letter with an inch
/// of margin.
#[test]
fn a_document_has_the_trees_default_page() {
    let mut session = session();
    let document = a_document(&mut session);
    let page = page_of(&mut session, document);
    assert_eq!(page["width"], json!(612.0), "the page is wide");
    assert_eq!(page["height"], json!(792.0), "and high");
    assert_eq!(page["top"], json!(72.0), "with an inch of margin");
    assert_eq!(page["left"], json!(72.0), "on the left too");
    assert_eq!(page["landscape"], json!(false), "and it is not a landscape page");
}

/// The page may be **set**.
#[test]
fn the_page_may_be_set() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute(
            "prose.page.set",
            &json!({ "document": document, "width": 595.0, "height": 842.0, "top": 36.0 }),
        )
        .expect("the page is set");
    let page = page_of(&mut session, document);
    assert_eq!(page["width"], json!(595.0), "the page is wide");
    assert_eq!(page["height"], json!(842.0), "and high");
    assert_eq!(page["top"], json!(36.0), "and the top margin is set");
    assert_eq!(page["bottom"], json!(72.0), "and the other margins keep theirs");
}

/// A page that is **wider than it is tall** is a landscape one, and
/// that is derived.
#[test]
fn a_landscape_page_is_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.set", &json!({ "document": document, "width": 842.0, "height": 595.0 }))
        .expect("the page is set");
    let page = page_of(&mut session, document);
    assert_eq!(page["landscape"], json!(true), "the page is a landscape one");
}

/// A page that is **not a page** is refused: a size of nothing, a
/// negative margin and a page too big are all refused.
#[test]
fn a_page_that_is_not_a_page_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.set", &json!({ "document": document, "width": 0.0, "height": 792.0 }))
        .expect_err("the page is refused");
    assert!(error.to_string().contains("not nothing wide"), "the refusal says why: {error}");
    let error = session
        .execute(
            "prose.page.set",
            &json!({ "document": document, "width": 612.0, "height": 792.0, "top": -10.0 }),
        )
        .expect_err("the margin is refused");
    assert!(error.to_string().contains("not negative"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.set", &json!({ "document": document, "width": 99999.0, "height": 792.0 }))
        .expect_err("the page is refused");
    assert!(error.to_string().contains("at most"), "the refusal says why: {error}");
}

/// A document that is **not there** is refused, and so is a node that
/// is not a document.
#[test]
fn a_document_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error =
        session.execute("prose.page.get", &json!({ "document": 999 })).expect_err("the document is refused");
    assert!(error.to_string().contains("there is no document"), "the refusal says why: {error}");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the first words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let error = session
        .execute("prose.page.get", &json!({ "document": paragraph }))
        .expect_err("the paragraph is not a document");
    assert!(error.to_string().contains("not a document"), "the refusal says why: {error}");
}
