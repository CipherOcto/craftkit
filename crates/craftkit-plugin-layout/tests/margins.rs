//! The **margin** of a page.

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

/// **A page may have margins.**
#[test]
fn a_page_may_have_margins() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    session
        .execute(
            "layout.page.margin",
            &json!({ "page": page, "top": 10.0, "bottom": 20.0, "left": 15.0, "right": 25.0 }),
        )
        .expect("the page has margins");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["margin"]["top"], json!(10.0), "the room at the top is there");
    assert_eq!(facts["margin"]["right"], json!(25.0), "and the room at the right");
}

/// The **margin box** of a page is derived.
#[test]
fn the_margin_box_of_a_page_is_derived() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    session
        .execute(
            "layout.page.margin",
            &json!({ "page": page, "top": 10.0, "bottom": 20.0, "left": 15.0, "right": 25.0 }),
        )
        .expect("the page has margins");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["margin_box"]["x"], json!(15.0), "the box starts at the left margin");
    assert_eq!(facts["margin_box"]["y"], json!(10.0), "and at the top margin");
    assert_eq!(facts["margin_box"]["width"], json!(60.0), "and it is as wide as the page and its margins");
    assert_eq!(facts["margin_box"]["height"], json!(170.0), "and as tall");
}

/// A side that is **not said** keeps the room it had.
#[test]
fn a_side_that_is_not_said_keeps_the_room_it_had() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    session
        .execute(
            "layout.page.margin",
            &json!({ "page": page, "top": 10.0, "bottom": 20.0, "left": 15.0, "right": 25.0 }),
        )
        .expect("the page has margins");
    session
        .execute("layout.page.margin", &json!({ "page": page, "top": 1.0 }))
        .expect("the top margin is changed");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["margin"]["top"], json!(1.0), "the top is the new room");
    assert_eq!(facts["margin"]["bottom"], json!(20.0), "and the bottom is what it was");
}

/// A page with **no margins** has the page's box as its margin box.
#[test]
fn a_page_with_no_margins_has_the_pages_box_as_its_margin_box() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["margin_box"]["x"], json!(0.0), "the box starts at the edge");
    assert_eq!(facts["margin_box"]["width"], json!(100.0), "and it is as wide as the page");
}

/// The room of a margin that is **negative** is refused.
#[test]
fn the_room_of_a_margin_that_is_negative_is_refused() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    let refused = session
        .execute("layout.page.margin", &json!({ "page": page, "top": -3.0 }))
        .expect_err("the margin is refused");
    assert!(refused.to_string().contains("negative"), "the refusal says why");
}

/// Margins that leave **no room** are refused.
#[test]
fn margins_that_leave_no_room_are_refused() {
    let mut session = session();
    let (_document, page) = a_page(&mut session);
    let refused = session
        .execute("layout.page.margin", &json!({ "page": page, "left": 60.0, "right": 60.0 }))
        .expect_err("the margins are refused");
    assert!(refused.to_string().contains("no room"), "the refusal says why");
}

/// A node that is **not a page** may not have margins.
#[test]
fn a_node_that_is_not_a_page_may_not_have_margins() {
    let mut session = session();
    let (document, _page) = a_page(&mut session);
    let refused = session
        .execute("layout.page.margin", &json!({ "page": document, "top": 10.0 }))
        .expect_err("the margins are refused");
    assert!(refused.to_string().contains("not a page"), "the refusal says why");
}
