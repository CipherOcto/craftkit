//! The **tab order** of a page.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// The **node** of an object, by its number.
fn node_of(session: &Session, object: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| node.id)
        .expect("the object is there")
}

/// A **page** of the document.
fn add_page(session: &mut Session, object: u64) -> u64 {
    session
        .execute("pdf.page.add", &json!({ "object": object, "entries": [["Type", "Page"]] }))
        .expect("the page is added");
    node_of(session, object)
}

/// The **entries** of a page, as the document holds them.
fn entries_of(session: &Session, page: u64) -> Json {
    session.document().node(page).expect("the page is there").data["entries"].clone()
}

/// The **order** the page asks for, as the tab order reads it.
fn order_of(session: &mut Session) -> String {
    let outcome = session.execute("pdf.tab.order", &json!({})).expect("the order is read");
    outcome.data["pages"]
        .as_array()
        .expect("the pages are a list")
        .first()
        .and_then(|page| page["order"].as_str())
        .unwrap_or_default()
        .to_string()
}

/// A page may be said to be reached in **rows**.
#[test]
fn a_page_may_be_said_to_be_reached_in_rows() {
    let mut session = session();
    let page = add_page(&mut session, 5);
    session.execute("pdf.page.tabs", &json!({ "page": page, "tabs": "rows" })).expect("the order is said");
    assert_eq!(order_of(&mut session), "rows", "the page is reached in rows");
}

/// The page says the **name the file uses**.
#[test]
fn the_page_says_the_name_the_file_uses() {
    let mut session = session();
    let page = add_page(&mut session, 5);
    session.execute("pdf.page.tabs", &json!({ "page": page, "tabs": "columns" })).expect("the order is said");
    let entries = entries_of(&session, page);
    let tabs = entries
        .as_array()
        .expect("the entries are a list")
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? != "Tabs" {
                return None;
            }
            Some(pair.get(1)?.clone())
        })
        .expect("the page has a Tabs");
    assert_eq!(tabs, json!("C"), "the file says C");
}

/// The order **annotations** takes the fact away.
#[test]
fn the_order_annotations_takes_the_fact_away() {
    let mut session = session();
    let page = add_page(&mut session, 5);
    session.execute("pdf.page.tabs", &json!({ "page": page, "tabs": "rows" })).expect("the order is said");
    session
        .execute("pdf.page.tabs", &json!({ "page": page, "tabs": "annotations" }))
        .expect("the order is said");
    let entries = entries_of(&session, page);
    let has = entries.as_array().expect("the entries are a list").iter().any(|entry| {
        entry.as_array().and_then(|pair| pair.first()).and_then(|key| key.as_str()) == Some("Tabs")
    });
    assert!(!has, "the page has no Tabs");
    assert_eq!(order_of(&mut session), "annotations", "and the page is reached in the annotation order");
}

/// An order that is **not an order** is refused.
#[test]
fn an_order_that_is_not_an_order_is_refused() {
    let mut session = session();
    let page = add_page(&mut session, 5);
    let refused = session
        .execute("pdf.page.tabs", &json!({ "page": page, "tabs": "sideways" }))
        .expect_err("the order is refused");
    assert!(refused.to_string().contains("not an order"), "the refusal says why: {refused}");
}

/// An order that is **not said** leaves the page as it was.
#[test]
fn an_order_that_is_not_said_leaves_the_page_as_it_was() {
    let mut session = session();
    let page = add_page(&mut session, 5);
    session
        .execute("pdf.page.tabs", &json!({ "page": page, "tabs": "structure" }))
        .expect("the order is said");
    session.execute("pdf.page.tabs", &json!({ "page": page })).expect("nothing is said");
    assert_eq!(order_of(&mut session), "structure", "the page keeps the order");
}

/// A node that is **not a page** is refused.
#[test]
fn a_node_that_is_not_a_page_is_refused() {
    let mut session = session();
    session
        .execute("pdf.object.add", &json!({ "object": 5, "entries": [["Type", "XObject"]] }))
        .expect("the object is added");
    let node = node_of(&session, 5);
    let refused = session
        .execute("pdf.page.tabs", &json!({ "page": node, "tabs": "rows" }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("is not a page"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session
        .execute("pdf.page.tabs", &json!({ "page": 999, "tabs": "rows" }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}
