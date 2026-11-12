//! The **removal of a page**.
//!
//! A page is an object of the file, and taking a page away is not only
//! a matter of dropping a node: the tree of pages has to be right
//! afterwards, and everything that pointed at the page has to stop
//! pointing at it.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
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

/// An **object** of the file, with the id it was given.
fn an_object(session: &mut Session, object: u64, entries: craftkit_sdk::serde_json::Value) -> u64 {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
    outcome.data["id"].as_u64().expect("the object has an id")
}

/// A **page** of the file, with the id it was given.
fn a_page(session: &mut Session, object: u64, parent: u64) -> u64 {
    let outcome = session
        .execute("pdf.page.add", &json!({ "object": object, "parent": parent }))
        .expect("the page is added");
    outcome.data["id"].as_u64().expect("the page has an id")
}

/// The **entries** of a node, as the document holds them.
fn entries_of(session: &Session, id: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(id).expect("the node is there");
    node.data["entries"]
        .as_array()
        .expect("the node has a dictionary")
        .clone()
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            Some((key, pair.get(1)?.clone()))
        })
        .collect()
}

/// The **value** under a key of a node.
fn value_of(session: &Session, id: u64, key: &str) -> Option<craftkit_sdk::serde_json::Value> {
    entries_of(session, id).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// A file with a **catalog**, a **tree of pages** and two pages.
fn a_file(session: &mut Session) -> (u64, u64, u64) {
    let root = an_object(session, 1, json!([["Type", "Pages"]]));
    an_object(session, 2, json!([["Type", "Catalog"], ["Pages", { "ref": [1, 0] }]]));
    let first = a_page(session, 3, root);
    let second = a_page(session, 4, root);
    (root, first, second)
}

/// A page may be **taken away**.
#[test]
fn a_page_may_be_taken_away() {
    let mut session = session();
    let (_, first, _) = a_file(&mut session);
    let outcome = session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    assert_eq!(outcome.data["object"].as_u64(), Some(3), "the page was the object 3");
    assert!(session.document().node(first).is_none(), "and the node is gone");
}

/// The **last page** cannot go: a file with no pages is not a file.
#[test]
fn the_last_page_cannot_go() {
    let mut session = session();
    let root = an_object(&mut session, 1, json!([["Type", "Pages"]]));
    let only = a_page(&mut session, 3, root);
    let outcome = session.execute("pdf.page.remove", &json!({ "page": only }));
    assert!(outcome.is_err(), "the last page stays");
    assert!(session.document().node(only).is_some(), "and the page is there");
}

/// The **tree of pages** is right afterwards: the parent loses the page
/// and the count of the children is derived.
#[test]
fn the_tree_of_pages_is_right_afterwards() {
    let mut session = session();
    let (root, first, _) = a_file(&mut session);
    assert_eq!(
        value_of(&session, root, "Count").and_then(|count| count.as_u64()),
        Some(2),
        "the tree has two children"
    );
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    let kids = value_of(&session, root, "Kids")
        .and_then(|kids| kids.as_array().cloned())
        .expect("the tree has children");
    assert_eq!(kids.len(), 1, "the tree has one child left");
    assert_eq!(
        kids[0]
            .get("ref")
            .and_then(|pair| pair.as_array())
            .and_then(|pair| pair.first())
            .and_then(|number| number.as_u64()),
        Some(4),
        "and it is the page that is left"
    );
    assert_eq!(
        value_of(&session, root, "Count").and_then(|count| count.as_u64()),
        Some(1),
        "the count is derived"
    );
}

/// A **destination** that went to the page goes with it: a link that
/// went to a page that is not there has no destination.
#[test]
fn a_destination_that_went_to_the_page_goes_with_it() {
    let mut session = session();
    let (_, first, second) = a_file(&mut session);
    session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": second,
                "object": 5,
                "subtype": "Link",
                "rect": [0.0, 0.0, 10.0, 10.0],
                "destination": first,
            }),
        )
        .expect("the link is added");
    let link = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.object" && node.data["object"].as_u64() == Some(5))
        .map(|node| node.id)
        .expect("the link is there");
    assert!(value_of(&session, link, "Dest").is_some(), "the link goes to the page");
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    assert!(value_of(&session, link, "Dest").is_none(), "and the link goes nowhere");
}

/// An **action** that goes to the page goes with it: a `GoTo` with no
/// destination is not an action.
#[test]
fn an_action_that_goes_to_the_page_goes_with_it() {
    let mut session = session();
    let (_, first, _) = a_file(&mut session);
    // The **action** is an object of its own: a `GoTo` that goes to the
    // first page.
    let action = an_object(
        &mut session,
        6,
        json!([["Type", "Action"], ["S", "GoTo"], ["D", [{ "ref": [3, 0] }, "Fit"]]]),
    );
    // And a **link** that uses it.
    let link = an_object(
        &mut session,
        5,
        json!([
            ["Type", "Annot"],
            ["Subtype", "Link"],
            ["Rect", [0.0, 0.0, 10.0, 10.0]],
            ["A", { "ref": [6, 0] }],
        ]),
    );
    assert!(value_of(&session, link, "A").is_some(), "the link has the action");
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    assert!(value_of(&session, link, "A").is_none(), "and the link has no action");
    assert!(session.document().node(action).is_some(), "the action itself stays");
}

/// What the page **held** stays: an annotation of the page is an
/// object of its own, and it stays in the document.
#[test]
fn what_the_page_held_stays() {
    let mut session = session();
    let (_, first, _) = a_file(&mut session);
    session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": first,
                "object": 5,
                "subtype": "Text",
                "rect": [0.0, 0.0, 10.0, 10.0],
                "contents": "a note",
            }),
        )
        .expect("the note is added");
    session.execute("pdf.page.remove", &json!({ "page": first })).expect("the page is gone");
    let note = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.object" && node.data["object"].as_u64() == Some(5));
    assert!(note.is_some(), "the note is still an object of the file");
}

/// A node that is **not a page** is refused.
#[test]
fn a_node_that_is_not_a_page_is_refused() {
    let mut session = session();
    let (root, _, _) = a_file(&mut session);
    let outcome = session.execute("pdf.page.remove", &json!({ "page": root }));
    assert!(outcome.is_err(), "the tree of pages is not a page");
}

/// A page that is **not there** is refused.
#[test]
fn a_page_that_is_not_there_is_refused() {
    let mut session = session();
    a_file(&mut session);
    let outcome = session.execute("pdf.page.remove", &json!({ "page": 999 }));
    assert!(outcome.is_err(), "there is no such page");
}
