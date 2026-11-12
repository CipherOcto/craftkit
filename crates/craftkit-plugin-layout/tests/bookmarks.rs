//! The **bookmarks** of a layout document.
//!
//! A bookmark names a page, and the page is counted from one.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the layout battery.
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

/// A **document** with a **spread** and **two** pages in it.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    for name in ["first", "second"] {
        session
            .execute("layout.page.add", &json!({ "spread": spread, "name": name }))
            .expect("the page is added");
    }
    document
}

/// The **bookmarks** of a document, as the answer holds them.
fn bookmarks_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("layout.document.bookmarks", &json!({ "document": document }))
        .expect("the bookmarks are read");
    outcome.data["bookmarks"].as_array().expect("the bookmarks are a list").clone()
}

/// A bookmark names a **page**, counted from one.
#[test]
fn a_bookmark_names_a_page() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("layout.bookmark.add", &json!({ "document": document, "name": "the cover", "page": 2 }))
        .expect("the bookmark is added");
    assert_eq!(outcome.data["page"], json!(2), "the bookmark names the second page: {outcome:?}");
    let bookmarks = bookmarks_of(&mut session, document);
    assert_eq!(bookmarks.len(), 1, "there is one bookmark: {bookmarks:?}");
    assert_eq!(bookmarks[0]["name"], json!("the cover"), "and it is named: {bookmarks:?}");
    assert_eq!(bookmarks[0]["page"], json!(2), "and it names the page: {bookmarks:?}");
}

/// A page that is **not there** is refused by name.
#[test]
fn a_page_that_is_not_there_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("layout.bookmark.add", &json!({ "document": document, "page": 9 }))
        .expect_err("the page is refused");
    assert!(outcome.to_string().contains("there is no page 9"), "the refusal says why: {outcome}");
}

/// A bookmark with **no name** is called `Bookmark N`, the tree's own
/// name.
#[test]
fn a_bookmark_with_no_name_is_called_bookmark_n() {
    let mut session = session();
    let document = a_document(&mut session);
    session.execute("layout.bookmark.add", &json!({ "document": document })).expect("the bookmark is added");
    session.execute("layout.bookmark.add", &json!({ "document": document })).expect("the bookmark is added");
    let bookmarks = bookmarks_of(&mut session, document);
    assert_eq!(bookmarks[0]["name"], json!("Bookmark 1"), "the first is the first: {bookmarks:?}");
    assert_eq!(bookmarks[1]["name"], json!("Bookmark 2"), "and the second is the second: {bookmarks:?}");
}

/// A bookmark may be **renamed**, and the name is not empty.
#[test]
fn a_bookmark_is_renamed() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("layout.bookmark.add", &json!({ "document": document, "name": "the cover" }))
        .expect("the bookmark is added");
    let bookmark = outcome.data["id"].as_u64().expect("the bookmark has an id");
    session
        .execute("layout.bookmark.rename", &json!({ "bookmark": bookmark, "name": "the title" }))
        .expect("the bookmark is renamed");
    let bookmarks = bookmarks_of(&mut session, document);
    assert_eq!(bookmarks[0]["name"], json!("the title"), "the bookmark has the new name: {bookmarks:?}");
    let outcome = session
        .execute("layout.bookmark.rename", &json!({ "bookmark": bookmark, "name": "  " }))
        .expect_err("an empty name is refused");
    assert!(outcome.to_string().contains("a bookmark has a name"), "the refusal says why: {outcome}");
}

/// A bookmark is **deleted**.
#[test]
fn a_bookmark_is_deleted() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("layout.bookmark.add", &json!({ "document": document, "name": "the cover" }))
        .expect("the bookmark is added");
    let bookmark = outcome.data["id"].as_u64().expect("the bookmark has an id");
    session
        .execute("layout.bookmark.delete", &json!({ "bookmark": bookmark }))
        .expect("the bookmark is deleted");
    let bookmarks = bookmarks_of(&mut session, document);
    assert!(bookmarks.is_empty(), "the document has no bookmark: {bookmarks:?}");
}
