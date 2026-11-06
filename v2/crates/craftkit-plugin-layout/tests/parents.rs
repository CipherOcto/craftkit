//! The **parent** of a page.

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
fn a_page(session: &mut Session) -> (u64, u64, u64) {
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
    let page = outcome.data["id"].as_u64().expect("the page has an id");
    (document, spread, page)
}

/// Another **page** in the same spread.
fn another_page(session: &mut Session, spread: u64) -> u64 {
    let outcome = session
        .execute(
            "layout.page.add",
            &json!({ "spread": spread, "name": "the other page", "width": 100.0, "height": 200.0 }),
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

/// A page may **follow** another page.
#[test]
fn a_page_may_follow_another_page() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect("the page follows the other one");
    assert_eq!(facts_of(&mut session, page)["parent"], json!(parent), "the page says who it follows");
}

/// The **margins** of a page are its own if it has them.
#[test]
fn the_margins_of_a_page_are_its_own_if_it_has_them() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    session
        .execute("layout.page.margin", &json!({ "page": parent, "left": 10.0, "right": 10.0 }))
        .expect("the parent has margins");
    session
        .execute("layout.page.margin", &json!({ "page": page, "left": 20.0, "right": 20.0 }))
        .expect("the page has margins of its own");
    session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect("the page follows the other one");
    assert_eq!(facts_of(&mut session, page)["margin"]["left"], json!(20.0), "the page keeps its own room");
}

/// A page that has no margins of its own takes the **parent's**.
#[test]
fn a_page_that_has_no_margins_of_its_own_takes_the_parents() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    session
        .execute("layout.page.margin", &json!({ "page": parent, "left": 10.0, "right": 10.0 }))
        .expect("the parent has margins");
    session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect("the page follows the other one");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["margin"]["left"], json!(10.0), "the room is the parent's");
    assert_eq!(facts["margin_box"]["x"], json!(10.0), "and the box follows it");
}

/// The **columns** of a page are inherited too.
#[test]
fn the_columns_of_a_page_are_inherited_too() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    session
        .execute("layout.page.columns", &json!({ "page": parent, "count": 2, "gutter": 10.0 }))
        .expect("the parent has columns");
    session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect("the page follows the other one");
    assert_eq!(facts_of(&mut session, page)["columns"]["count"], json!(2), "the columns are the parent's");
}

/// The **grandparent** is followed when the parent has nothing.
#[test]
fn the_grandparent_is_followed_when_the_parent_has_nothing() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    let grandparent = another_page(&mut session, spread);
    session
        .execute("layout.page.margin", &json!({ "page": grandparent, "left": 30.0, "right": 30.0 }))
        .expect("the grandparent has margins");
    session
        .execute("layout.page.parent", &json!({ "page": parent, "parent": grandparent }))
        .expect("the parent follows the grandparent");
    session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect("the page follows the parent");
    assert_eq!(facts_of(&mut session, page)["margin"]["left"], json!(30.0), "the room is the grandparent's");
}

/// The **parent** may be taken away.
#[test]
fn the_parent_may_be_taken_away() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    session
        .execute("layout.page.margin", &json!({ "page": parent, "left": 10.0, "right": 10.0 }))
        .expect("the parent has margins");
    session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect("the page follows the other one");
    session.execute("layout.page.parent.clear", &json!({ "page": page })).expect("the parent is taken away");
    let facts = facts_of(&mut session, page);
    assert_eq!(facts["parent"], Json::Null, "the page follows no page");
    assert_eq!(facts["margin"]["left"], json!(0.0), "and it has no room of its own");
}

/// A page may not be its own parent.
#[test]
fn a_page_may_not_be_its_own_parent() {
    let mut session = session();
    let (_document, _spread, page) = a_page(&mut session);
    let refused = session
        .execute("layout.page.parent", &json!({ "page": page, "parent": page }))
        .expect_err("the parent is refused");
    assert!(refused.to_string().contains("own parent"), "the refusal says why: {refused}");
}

/// A **loop** of parents is refused.
#[test]
fn a_loop_of_parents_is_refused() {
    let mut session = session();
    let (_document, spread, page) = a_page(&mut session);
    let parent = another_page(&mut session, spread);
    session
        .execute("layout.page.parent", &json!({ "page": parent, "parent": page }))
        .expect("the parent follows the page");
    let refused = session
        .execute("layout.page.parent", &json!({ "page": page, "parent": parent }))
        .expect_err("the loop is refused");
    assert!(refused.to_string().contains("loop"), "the refusal says why: {refused}");
}

/// A parent that is **not a page** is refused.
#[test]
fn a_parent_that_is_not_a_page_is_refused() {
    let mut session = session();
    let (document, _spread, page) = a_page(&mut session);
    let refused = session
        .execute("layout.page.parent", &json!({ "page": page, "parent": document }))
        .expect_err("the parent is refused");
    assert!(refused.to_string().contains("not a page"), "the refusal says why: {refused}");
}

/// A page that follows **no page** has no parent to take away.
#[test]
fn a_page_that_follows_no_page_has_no_parent_to_take_away() {
    let mut session = session();
    let (_document, _spread, page) = a_page(&mut session);
    let refused = session
        .execute("layout.page.parent.clear", &json!({ "page": page }))
        .expect_err("there is no parent");
    assert!(refused.to_string().contains("follows no page"), "the refusal says why: {refused}");
}
