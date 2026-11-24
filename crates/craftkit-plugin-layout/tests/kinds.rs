//! The **kinds** of the layout battery: what a document, a spread, a
//! style, a bookmark, an article and a guide mean.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

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

/// A **document** with a **spread** and a **page** in it.
fn a_page(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let outcome = session
        .execute("layout.page.add", &json!({ "spread": spread, "name": "the page" }))
        .expect("the page is added");
    let page = outcome.data["id"].as_u64().expect("the page has an id");
    (document, page)
}

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// A **document** is a kind, and it means what it holds.
#[test]
fn a_document_means_what_it_holds() {
    let mut session = session();
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let node = session.document().node(document).expect("the document is there");
    assert_eq!(node.kind, "layout.document", "the node is a document");
    let said = said_of(&session, document);
    assert_eq!(said["spreads"], json!(0), "the document says how many spreads it holds");
    assert_eq!(said["layers"], json!(0), "and how many layers");
    assert_eq!(said["stories"], json!(0), "and how many stories");
    assert_eq!(said["assets"], json!(0), "and how many assets");
}

/// A **spread** is a kind, and it means its pages and its items.
#[test]
fn a_spread_means_its_pages_and_its_items() {
    let mut session = session();
    let (document, _) = a_page(&mut session);
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the second spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let node = session.document().node(spread).expect("the spread is there");
    assert_eq!(node.kind, "layout.spread", "the node is a spread");
    let said = said_of(&session, spread);
    assert_eq!(said["pages"], json!(0), "the spread says how many pages it holds");
    assert_eq!(said["items"], json!(0), "and how many items");
}

/// A **style** is a kind, and it means its paint and the paint of its
/// base under it.
#[test]
fn a_style_means_its_paint_and_the_paint_of_its_base() {
    let mut session = session();
    let (document, _) = a_page(&mut session);
    let outcome = session
        .execute("layout.style.add", &json!({ "document": document, "name": "the house style" }))
        .expect("the style is added");
    let style = outcome.data["style"].as_u64().expect("the style has an id");
    let node = session.document().node(style).expect("the style is there");
    assert_eq!(node.kind, "layout.style", "the node is a style");
    let said = said_of(&session, style);
    assert_eq!(said["base"], json!(null), "the style has no base under it");
    assert!(said.get("fill").is_some(), "the style says its fill");
    assert!(said.get("stroke").is_some(), "and its stroke");
}

/// A **bookmark** is a kind, and it means its name and the page it
/// names.
#[test]
fn a_bookmark_means_its_name_and_the_page_it_names() {
    let mut session = session();
    let (document, _) = a_page(&mut session);
    let outcome = session
        .execute("layout.bookmark.add", &json!({ "document": document, "name": "the cover", "page": 1 }))
        .expect("the bookmark is added");
    let bookmark = outcome.data["id"].as_u64().expect("the bookmark has an id");
    let node = session.document().node(bookmark).expect("the bookmark is there");
    assert_eq!(node.kind, "layout.bookmark", "the node is a bookmark");
    let said = said_of(&session, bookmark);
    assert_eq!(said["name"], json!("the cover"), "the bookmark says its name");
    assert_eq!(said["page"], json!(1), "and the page it names");
}

/// An **article** is a kind, and it means its name and its items.
#[test]
fn an_article_means_its_name_and_its_items() {
    let mut session = session();
    let (document, _) = a_page(&mut session);
    let outcome = session
        .execute("layout.article.new", &json!({ "document": document, "name": "the piece" }))
        .expect("the article is added");
    let article = outcome.data["id"].as_u64().expect("the article has an id");
    let node = session.document().node(article).expect("the article is there");
    assert_eq!(node.kind, "layout.article", "the node is an article");
    let said = said_of(&session, article);
    assert_eq!(said["name"], json!("the piece"), "the article says its name");
    assert_eq!(said["export"], json!(true), "and whether it is exported");
}

/// A **guide** is a kind, and it means how it lies and where it stands.
#[test]
fn a_guide_means_how_it_lies_and_where_it_stands() {
    let mut session = session();
    let (_, page) = a_page(&mut session);
    let outcome = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "vertical", "position": 100.0 }))
        .expect("the guide is added");
    let guide = outcome.data["guides"][0]["id"].as_u64().expect("the guide has an id");
    let node = session.document().node(guide).expect("the guide is there");
    assert_eq!(node.kind, "layout.guide", "the node is a guide");
    let said = said_of(&session, guide);
    assert_eq!(said["orientation"], json!("vertical"), "the guide says how it lies");
    assert_eq!(said["position"], json!(100.0), "and where it stands");
}
