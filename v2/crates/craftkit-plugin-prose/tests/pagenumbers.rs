//! The **numbering** of the pages.
//!
//! The format the numbers are written in, and the number the first
//! page has.

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

/// The pages are numbered in **decimal** until they are told
/// otherwise.
#[test]
fn the_pages_are_numbered_in_decimal() {
    let mut session = session();
    let document = a_document(&mut session);
    let page = page_of(&mut session, document);
    assert_eq!(page["numbers"]["format"], json!("decimal"), "the format is decimal: {page}");
    assert_eq!(page["numbers"]["start"], json!(1), "and the first page is one");
    assert_eq!(page["numbers"]["first"], json!("1"), "and it says so");
}

/// The **format** is one of the tree's own, and the **first** page's
/// number is **derived**: it is the start, written in the format.
#[test]
fn the_first_page_is_derived_in_the_format() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.numbers", &json!({ "document": document, "format": "lowerRoman", "start": 4 }))
        .expect("the pages are numbered");
    let page = page_of(&mut session, document);
    assert_eq!(page["numbers"]["format"], json!("lowerRoman"), "the format is said: {page}");
    assert_eq!(page["numbers"]["start"], json!(4), "and the start");
    assert_eq!(page["numbers"]["first"], json!("iv"), "and the first page is four in roman");
}

/// The **formats** are the tree's own, and the spelling of one
/// decides.
#[test]
fn the_formats_are_the_trees_own() {
    let mut session = session();
    let document = a_document(&mut session);
    for (format, first) in [
        ("decimal", "4"),
        ("lowerRoman", "iv"),
        ("upperRoman", "IV"),
        ("lowerLetter", "d"),
        ("upperLetter", "D"),
    ] {
        session
            .execute("prose.page.numbers", &json!({ "document": document, "format": format, "start": 4 }))
            .expect("the pages are numbered");
        let page = page_of(&mut session, document);
        assert_eq!(page["numbers"]["first"], json!(first), "the format {format} says {first}");
    }
}

/// A part that is **not named** keeps what it was, and a start that
/// is **outside the range** is refused — the tree clamps it.
#[test]
fn a_start_outside_the_range_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    session
        .execute("prose.page.numbers", &json!({ "document": document, "format": "lowerRoman", "start": 4 }))
        .expect("the pages are numbered");
    session
        .execute("prose.page.numbers", &json!({ "document": document, "start": 9 }))
        .expect("the pages are numbered");
    let page = page_of(&mut session, document);
    assert_eq!(page["numbers"]["format"], json!("lowerRoman"), "the format is kept: {page}");
    assert_eq!(page["numbers"]["start"], json!(9), "and the start is the new one");
    let error = session
        .execute("prose.page.numbers", &json!({ "document": document, "start": 100_001 }))
        .expect_err("the start is refused");
    assert!(error.to_string().contains("from one to 100000"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.numbers", &json!({ "document": document, "start": 0 }))
        .expect_err("the start is refused");
    assert!(error.to_string().contains("from one to 100000"), "the refusal says why: {error}");
}

/// A **format** that is not one of the tree's is refused by name, and
/// a node that is **not a document** is refused.
#[test]
fn a_format_that_is_not_one_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.numbers", &json!({ "document": document, "format": "greek" }))
        .expect_err("the format is refused");
    assert!(error.to_string().contains("no format"), "the refusal says why: {error}");
    let error = session
        .execute("prose.page.numbers", &json!({ "document": 9999, "format": "decimal" }))
        .expect_err("there is no document");
    assert!(error.to_string().contains("no document"), "the refusal says why: {error}");
}
