//! The **pages** of a spread may be deleted and moved.
//!
//! The pages close up when a page is deleted: the items beyond it
//! move left by its width.

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

/// A **spread** with **three** pages and an **item** on the third.
fn a_spread(session: &mut Session) -> (u64, Vec<u64>, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut pages = Vec::new();
    for name in ["first", "second", "third"] {
        let outcome = session
            .execute("layout.page.add", &json!({ "spread": spread, "name": name, "width": 100 }))
            .expect("the page is added");
        pages.push(outcome.data["id"].as_u64().expect("the page has an id"));
    }
    // An **item** that stands beyond the second page.
    let outcome = session
        .execute(
            "layout.item.add",
            &json!({ "spread": spread, "name": "the item", "rect": { "x": 250, "y": 10, "width": 20, "height": 20 } }),
        )
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (spread, pages, item)
}

/// The **pages** of a spread, in the order they stand in.
fn pages_of(session: &Session, spread: u64) -> Vec<u64> {
    let node = session.document().node(spread).expect("the spread is there");
    node.children.iter().filter(|child| child.kind == "layout.page").map(|child| child.id).collect()
}

/// The **place** an item stands at.
fn x_of(session: &Session, item: u64) -> f64 {
    let node = session.document().node(item).expect("the item is there");
    node.data["rect"]["x"].as_f64().unwrap_or_default()
}

/// A page is **deleted**, and the items beyond it move left by its
/// width: the pages close up.
#[test]
fn a_page_is_deleted_and_the_pages_close_up() {
    let mut session = session();
    let (spread, pages, item) = a_spread(&mut session);
    assert_eq!(x_of(&session, item), 250.0, "the item stands where it does");
    session.execute("layout.page.delete", &json!({ "page": pages[1] })).expect("the page is deleted");
    assert_eq!(pages_of(&session, spread), vec![pages[0], pages[2]], "the page is gone");
    assert_eq!(x_of(&session, item), 150.0, "and the item moved left by the width of the page");
}

/// A document needs **at least one page**.
#[test]
fn a_document_needs_at_least_one_page() {
    let mut session = session();
    let (_spread, pages, _item) = a_spread(&mut session);
    session.execute("layout.page.delete", &json!({ "page": pages[1] })).expect("the page is deleted");
    session.execute("layout.page.delete", &json!({ "page": pages[2] })).expect("the page is deleted");
    let outcome = session
        .execute("layout.page.delete", &json!({ "page": pages[0] }))
        .expect_err("the last page may not be deleted");
    assert!(outcome.to_string().contains("at least one page"), "the refusal says why: {outcome}");
}

/// A page is **moved** to another place among the pages of its spread.
#[test]
fn a_page_is_moved() {
    let mut session = session();
    let (spread, pages, _item) = a_spread(&mut session);
    session.execute("layout.page.move", &json!({ "page": pages[2], "to": 0 })).expect("the page is moved");
    assert_eq!(pages_of(&session, spread), vec![pages[2], pages[0], pages[1]], "the page is at the front");
    let outcome = session
        .execute("layout.page.move", &json!({ "page": pages[0], "to": 7 }))
        .expect_err("a place that is not there is refused");
    assert!(outcome.to_string().contains("the place is a number"), "the refusal says why: {outcome}");
}
