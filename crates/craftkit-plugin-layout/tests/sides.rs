//! The **side** of a page.

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
fn a_page(session: &mut Session) -> u64 {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the book" }))
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
    outcome.data["id"].as_u64().expect("the page has an id")
}

/// The **facts** of a node, as the plan reports them.
fn facts_of(session: &mut Session, node: u64) -> Json {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == node).expect("the node is in the plan");
    entry.value.clone()
}

/// A page that says nothing is a **single** page.
#[test]
fn a_page_that_says_nothing_is_a_single_page() {
    let mut session = session();
    let page = a_page(&mut session);
    assert_eq!(facts_of(&mut session, page)["side"], json!("single"), "the page is on its own");
}

/// A page may be a **left-hand** page.
#[test]
fn a_page_may_be_a_left_hand_page() {
    let mut session = session();
    let page = a_page(&mut session);
    session.execute("layout.page.side", &json!({ "page": page, "side": "left" })).expect("the side is said");
    assert_eq!(facts_of(&mut session, page)["side"], json!("left"), "the page is the left one");
}

/// A page may be a **right-hand** page.
#[test]
fn a_page_may_be_a_right_hand_page() {
    let mut session = session();
    let page = a_page(&mut session);
    session.execute("layout.page.side", &json!({ "page": page, "side": "right" })).expect("the side is said");
    assert_eq!(facts_of(&mut session, page)["side"], json!("right"), "the page is the right one");
}

/// The **inside** margin of a single page is at its left.
#[test]
fn the_inside_margin_of_a_single_page_is_at_its_left() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.margin", &json!({ "page": page, "left": 10.0, "right": 20.0 }))
        .expect("the page has margins");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["inside"], json!(10.0), "the room at the spine is the left one");
    assert_eq!(facts["outside"], json!(20.0), "and the other is the outside");
}

/// The **inside** margin of a right-hand page is at its left.
#[test]
fn the_inside_margin_of_a_right_hand_page_is_at_its_left() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.margin", &json!({ "page": page, "left": 10.0, "right": 20.0 }))
        .expect("the page has margins");
    session.execute("layout.page.side", &json!({ "page": page, "side": "right" })).expect("the side is said");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["inside"], json!(10.0), "the room at the spine is the left one");
    assert_eq!(facts["outside"], json!(20.0), "and the other is the outside");
}

/// The **inside** margin of a left-hand page is at its right.
#[test]
fn the_inside_margin_of_a_left_hand_page_is_at_its_right() {
    let mut session = session();
    let page = a_page(&mut session);
    session
        .execute("layout.page.margin", &json!({ "page": page, "left": 10.0, "right": 20.0 }))
        .expect("the page has margins");
    session.execute("layout.page.side", &json!({ "page": page, "side": "left" })).expect("the side is said");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["inside"], json!(20.0), "the spine is on the right of a left-hand page");
    assert_eq!(facts["outside"], json!(10.0), "and the outside is the other edge");
}

/// A side that is **not a side** is refused.
#[test]
fn a_side_that_is_not_a_side_is_refused() {
    let mut session = session();
    let page = a_page(&mut session);
    let refused = session
        .execute("layout.page.side", &json!({ "page": page, "side": "middle" }))
        .expect_err("the side is refused");
    assert!(refused.to_string().contains("not a side"), "the refusal says why: {refused}");
}

/// A node that is **not a page** may not have a side.
#[test]
fn a_node_that_is_not_a_page_may_not_have_a_side() {
    let mut session = session();
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the book" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let refused = session
        .execute("layout.page.side", &json!({ "page": document, "side": "left" }))
        .expect_err("the side is refused");
    assert!(refused.to_string().contains("not a page"), "the refusal says why: {refused}");
}
