//! The **link** of an item: a target outside the document, or a page
//! inside it.

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

/// A **document** with a **spread** in it.
fn a_spread(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    (document, spread)
}

/// An **item** on a spread.
fn an_item(session: &mut Session, spread: u64) -> u64 {
    let outcome = session
        .execute(
            "layout.item.add",
            &json!({
                "spread": spread,
                "name": "the shape",
                "kind": "layout.item.graphic",
                "shape": "rectangle",
                "rect": { "x": 0.0, "y": 0.0, "width": 100.0, "height": 50.0 },
            }),
        )
        .expect("the item is added");
    outcome.data["id"].as_u64().expect("the item has an id")
}

/// A **page** in a spread.
fn a_page(session: &mut Session, spread: u64) -> u64 {
    let outcome = session
        .execute("layout.page.add", &json!({ "spread": spread, "name": "the page" }))
        .expect("the page is added");
    outcome.data["id"].as_u64().expect("the page has an id")
}

/// The **links** of a document.
fn links_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("layout.links", &json!({ "document": document })).expect("the links are read");
    outcome.data["links"].as_array().expect("the links are a list").clone()
}

/// **A link may be put on an item, and it is read back.**
#[test]
fn a_link_may_be_put_on_an_item() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.link", &json!({ "item": item, "target": "https://example.com" }))
        .expect("the link is set");
    let links = links_of(&mut session, document);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["target"], json!("https://example.com"));
    assert_eq!(links[0]["may_be_opened"], json!(true));
    assert_eq!(links[0]["kind"], json!("target"));
}

/// **A mail link may be put on an item.**
#[test]
fn a_mail_link_may_be_put_on_an_item() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.link", &json!({ "item": item, "target": "mailto:ann@example.com" }))
        .expect("the link is set");
    let links = links_of(&mut session, document);
    assert_eq!(links[0]["may_be_opened"], json!(true));
}

/// **A target that may not be opened is refused, and each for its own
/// reason.**
#[test]
fn a_target_that_may_not_be_opened_is_refused() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    for target in
        ["javascript:alert(1)", "data:text/plain,hello", "file:///etc/passwd", "//host/path", "page.html"]
    {
        let outcome = session.execute("layout.item.link", &json!({ "item": item, "target": target }));
        assert!(outcome.is_err(), "the target `{target}` is refused");
    }
}

/// **A link may be taken away.**
#[test]
fn a_link_may_be_taken_away() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.link", &json!({ "item": item, "target": "https://example.com" }))
        .expect("the link is set");
    session
        .execute("layout.item.link", &json!({ "item": item, "target": "" }))
        .expect("the link is taken away");
    assert!(links_of(&mut session, document).is_empty(), "the item has no link");
}

/// **An item may point at a page of the document.**
#[test]
fn an_item_may_point_at_a_page() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let page = a_page(&mut session, spread);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.page.link", &json!({ "item": item, "page": page }))
        .expect("the link is set");
    let links = links_of(&mut session, document);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["kind"], json!("page"));
    assert_eq!(links[0]["page"], json!(page));
    assert_eq!(links[0]["page_name"], json!("the page"));
    assert_eq!(links[0]["may_be_opened"], json!(true), "a link inside the document may be opened");
}

/// **A page of another document is refused.**
#[test]
fn a_page_of_another_document_is_refused() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let (_, other_spread) = a_spread(&mut session);
    let page = a_page(&mut session, other_spread);
    let outcome = session.execute("layout.item.page.link", &json!({ "item": item, "page": page }));
    assert!(outcome.is_err(), "a page of another document is not a page of this one");
}

/// **The link of an item is one thing: a target takes the page away.**
#[test]
fn the_link_of_an_item_is_one_thing() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let page = a_page(&mut session, spread);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.page.link", &json!({ "item": item, "page": page }))
        .expect("the page link is set");
    session
        .execute("layout.item.link", &json!({ "item": item, "target": "https://example.com" }))
        .expect("the target link is set");
    let links = links_of(&mut session, document);
    assert_eq!(links.len(), 1, "the item has one link");
    assert_eq!(links[0]["kind"], json!("target"));
}

/// **A link to a page may be taken away.**
#[test]
fn a_link_to_a_page_may_be_taken_away() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let page = a_page(&mut session, spread);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.page.link", &json!({ "item": item, "page": page }))
        .expect("the link is set");
    session.execute("layout.item.page.link", &json!({ "item": item })).expect("the link is taken away");
    assert!(links_of(&mut session, document).is_empty());
}

/// **A locked item may not be given a link.**
#[test]
fn a_locked_item_may_not_be_given_a_link() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    let outcome =
        session.execute("layout.item.link", &json!({ "item": item, "target": "https://example.com" }));
    assert!(outcome.is_err(), "a locked item may not be edited");
}

/// **An item with no link is not in the list.**
#[test]
fn an_item_with_no_link_is_not_in_the_list() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    an_item(&mut session, spread);
    assert!(links_of(&mut session, document).is_empty());
}
