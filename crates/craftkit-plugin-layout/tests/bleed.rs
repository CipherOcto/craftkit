//! The **bleed** of a document.

use craftkit_sdk::serde_json::{Value as Json, json};
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
        .execute(
            "layout.page.add",
            &json!({ "spread": spread, "name": "the page", "width": 100.0, "height": 200.0 }),
        )
        .expect("the page is added");
    let page = outcome.data["id"].as_u64().expect("the page has an id");
    (document, page)
}

/// The **facts** of a node, as the plan reports them.
fn facts_of(session: &mut Session, node: u64) -> Json {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == node).expect("the node is in the plan");
    entry.value.clone()
}

/// **A document may have a bleed.**
#[test]
fn a_document_may_have_a_bleed() {
    let mut session = session();
    let (document, _page) = a_page(&mut session);
    session
        .execute(
            "layout.document.bleed",
            &json!({ "document": document, "top": 3.0, "bottom": 4.0, "left": 5.0, "right": 6.0 }),
        )
        .expect("the document has a bleed");
    let facts = facts_of(&mut session, document);
    assert_eq!(facts["bleed"]["top"], json!(3.0), "the room at the top is there");
    assert_eq!(facts["bleed"]["right"], json!(6.0), "and the room at the right");
}

/// A side that is **not said** keeps the room it had.
#[test]
fn a_side_that_is_not_said_keeps_the_room_it_had() {
    let mut session = session();
    let (document, _page) = a_page(&mut session);
    session
        .execute(
            "layout.document.bleed",
            &json!({ "document": document, "top": 3.0, "bottom": 4.0, "left": 5.0, "right": 6.0 }),
        )
        .expect("the document has a bleed");
    session
        .execute("layout.document.bleed", &json!({ "document": document, "top": 1.0 }))
        .expect("the top of the bleed is changed");
    let facts = facts_of(&mut session, document);
    assert_eq!(facts["bleed"]["top"], json!(1.0), "the top is the new room");
    assert_eq!(facts["bleed"]["bottom"], json!(4.0), "and the bottom is what it was");
}

/// The **bleed box** of a page is derived.
#[test]
fn the_bleed_box_of_a_page_is_derived() {
    let mut session = session();
    let (document, page) = a_page(&mut session);
    session
        .execute(
            "layout.document.bleed",
            &json!({ "document": document, "top": 3.0, "bottom": 4.0, "left": 5.0, "right": 6.0 }),
        )
        .expect("the document has a bleed");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["bleed"]["x"], json!(-5.0), "the box starts at the left of the bleed");
    assert_eq!(facts["bleed"]["y"], json!(-3.0), "and at the top of the bleed");
    assert_eq!(facts["bleed"]["width"], json!(111.0), "and it is as wide as the page and its bleed");
    assert_eq!(facts["bleed"]["height"], json!(207.0), "and as tall");
}

/// A document with **no bleed** has no room at all.
#[test]
fn a_document_with_no_bleed_has_no_room_at_all() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["bleed"]["x"], json!(0.0), "the box is the box of the page");
    assert_eq!(facts["bleed"]["width"], json!(100.0), "and it is as wide as the page");
}

/// The room of a bleed that is **negative** is refused.
#[test]
fn the_room_of_a_bleed_that_is_negative_is_refused() {
    let mut session = session();
    let (document, _page) = a_page(&mut session);
    let refused = session
        .execute("layout.document.bleed", &json!({ "document": document, "top": -3.0 }))
        .expect_err("the bleed is refused");
    assert!(refused.to_string().contains("negative"), "the refusal says why");
}

/// A node that is **not a document** may not have a bleed.
#[test]
fn a_node_that_is_not_a_document_may_not_have_a_bleed() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    let refused = session
        .execute("layout.document.bleed", &json!({ "document": page, "top": 3.0 }))
        .expect_err("the bleed is refused");
    assert!(refused.to_string().contains("not a document"), "the refusal says why");
}
