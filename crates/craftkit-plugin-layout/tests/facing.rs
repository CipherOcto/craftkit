//! The **facing page** of a page.

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

/// A **spread** with the given number of pages, and the ids of the
/// pages.
fn a_spread(session: &mut Session, pages: usize) -> (u64, Vec<u64>) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the book" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut ids: Vec<u64> = Vec::new();
    for index in 0..pages {
        let outcome = session
            .execute(
                "layout.page.add",
                &json!({ "spread": spread, "name": format!("the page {index}"), "width": 100.0, "height": 200.0 }),
            )
            .expect("the page is added");
        ids.push(outcome.data["id"].as_u64().expect("the page has an id"));
    }
    (spread, ids)
}

/// The **facts** of a node, as the plan reports them.
fn facts_of(session: &Session, node: u64) -> Json {
    let plan = session.evaluate();
    plan.items.iter().find(|entry| entry.node == node).map(|entry| entry.value.clone()).unwrap_or(Json::Null)
}

/// The **facing page**, as the command reads it.
fn facing_of(session: &mut Session, page: u64) -> Json {
    let outcome =
        session.execute("layout.page.facing", &json!({ "page": page })).expect("the facing page is read");
    outcome.data["facing"].clone()
}

/// A page that faces nothing has **no** facing page.
#[test]
fn a_page_that_faces_nothing_has_no_facing_page() {
    let mut session = session();
    let (_, pages) = a_spread(&mut session, 1);
    let page = pages[0];
    assert_eq!(facing_of(&mut session, page), Json::Null, "the page faces nothing");
}

/// A page in a spread of **two** faces the other one.
#[test]
fn a_page_in_a_spread_of_two_faces_the_other_one() {
    let mut session = session();
    let (_, pages) = a_spread(&mut session, 2);
    let (first, second) = (pages[0], pages[1]);
    session.execute("layout.page.side", &json!({ "page": first, "side": "left" })).expect("the side is said");
    session
        .execute("layout.page.side", &json!({ "page": second, "side": "right" }))
        .expect("the side is said");
    assert_eq!(facing_of(&mut session, first), json!(second), "the first faces the second");
    assert_eq!(facing_of(&mut session, second), json!(first), "and the second faces the first");
}

/// A page that says it is **single** faces nothing.
#[test]
fn a_page_that_says_it_is_single_faces_nothing() {
    let mut session = session();
    let (_, pages) = a_spread(&mut session, 2);
    let page = pages[0];
    assert_eq!(facing_of(&mut session, page), Json::Null, "the page says it is single");
}

/// A spread of **three** pages has no facing page in it.
#[test]
fn a_spread_of_three_pages_has_no_facing_page_in_it() {
    let mut session = session();
    let (_, pages) = a_spread(&mut session, 3);
    let page = pages[0];
    session.execute("layout.page.side", &json!({ "page": page, "side": "left" })).expect("the side is said");
    assert_eq!(facing_of(&mut session, page), Json::Null, "there is nothing to face");
}

/// The page says which page it **faces** and which **spread** it is
/// in.
#[test]
fn the_page_says_which_page_it_faces_and_which_spread_it_is_in() {
    let mut session = session();
    let (spread, pages) = a_spread(&mut session, 2);
    let (first, second) = (pages[0], pages[1]);
    session.execute("layout.page.side", &json!({ "page": first, "side": "left" })).expect("the side is said");
    session
        .execute("layout.page.side", &json!({ "page": second, "side": "right" }))
        .expect("the side is said");
    let facts = facts_of(&session, first);
    assert_eq!(facts["facing"], json!(second), "the page faces the second");
    assert_eq!(facts["spread"], json!(spread), "and is in the spread");
}

/// The facing page is **derived**: it follows the spread.
#[test]
fn the_facing_page_is_derived_and_it_follows_the_spread() {
    let mut session = session();
    let (_, pages) = a_spread(&mut session, 2);
    let (first, second) = (pages[0], pages[1]);
    session.execute("layout.page.side", &json!({ "page": first, "side": "left" })).expect("the side is said");
    session
        .execute("layout.page.side", &json!({ "page": second, "side": "right" }))
        .expect("the side is said");
    assert_eq!(facing_of(&mut session, first), json!(second), "the first faces the second");
    // The first page says it is single again: it faces nothing.
    session
        .execute("layout.page.side", &json!({ "page": first, "side": "single" }))
        .expect("the side is said");
    assert_eq!(facing_of(&mut session, first), Json::Null, "and now it faces nothing");
}

/// A node that is **not a page** is refused.
#[test]
fn a_node_that_is_not_a_page_is_refused() {
    let mut session = session();
    let (spread, _) = a_spread(&mut session, 1);
    let refused = session
        .execute("layout.page.facing", &json!({ "page": spread }))
        .expect_err("the facing page is refused");
    assert!(refused.to_string().contains("not a page"), "the refusal says why: {refused}");
}
