//! The **outline**: the bookmarks of a PDF document.
//!
//! An outline is a tree of items: each one has a title, and each one
//! may have children. The file keeps the tree as a **list** of
//! siblings — each item names the next, and the last names none — and
//! the root of the outline names the first of the top-level items.
//!
//! The counts are **derived**: the count of an item is how many items
//! are visible below it, and the sign says whether it is open or
//! closed.

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

/// A **catalog**, the object a file's root is.
fn a_catalog(session: &mut Session) -> u64 {
    session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Catalog"]] }))
        .expect("the catalog is added");
    session.document().walk().last().map(|node| node.id).expect("the catalog is there")
}

/// A **page**, the thing a bookmark goes to.
fn a_page(session: &mut Session, object: u64) -> u64 {
    session.execute("pdf.page.add", &json!({ "object": object, "entries": [] })).expect("the page is added");
    session.document().walk().last().map(|node| node.id).expect("the page is there")
}

/// The **root** of the outline: the object that says it is one.
fn the_root(session: &Session) -> u64 {
    session
        .document()
        .walk()
        .find(|node| under(session, node.id, "Type") == Some(json!("Outlines")))
        .map(|node| node.id)
        .expect("the outline is there")
}

/// The **entries** of a node's dictionary, as the pairs they are.
fn entries_of(session: &Session, id: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(id).expect("the node is there");
    node.data["entries"]
        .as_array()
        .expect("the node has a dictionary")
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            Some((key, pair.get(1)?.clone()))
        })
        .collect()
}

/// The value under a key of a node's dictionary.
fn under(session: &Session, id: u64, key: &str) -> Option<craftkit_sdk::serde_json::Value> {
    entries_of(session, id).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// A bookmark may be put in the outline, and the outline is made when
/// it is not there.
#[test]
fn a_bookmark_may_be_put_in_the_outline() {
    let mut session = session();
    let catalog = a_catalog(&mut session);
    let page = a_page(&mut session, 2);
    let outcome = session
        .execute("pdf.outline.add", &json!({ "title": "Chapter one", "destination": page, "object": 10 }))
        .expect("the bookmark is added");
    let item = outcome.data["id"].as_u64().expect("the bookmark has an id");
    // The **catalog** names the outline, and the outline is an object
    // that says it is one.
    let root = under(&session, catalog, "Outlines")
        .and_then(|value| {
            value
                .get("ref")
                .and_then(|pair| pair.as_array())
                .and_then(|pair| pair.first())
                .and_then(|number| number.as_u64())
        })
        .expect("the catalog names the outline");
    let root_node = session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|value| value.as_u64()) == Some(root))
        .expect("the outline is there");
    assert_eq!(
        under(&session, root_node.id, "Type"),
        Some(json!("Outlines")),
        "the root says it is the outline"
    );
    // The bookmark has a **title** and a **destination**.
    assert_eq!(
        under(&session, item, "Title"),
        Some(json!({ "string": "Chapter one" })),
        "the bookmark has a title"
    );
    assert_eq!(
        under(&session, item, "Dest"),
        Some(json!([{ "ref": [2, 0] }, "Fit"])),
        "the bookmark goes to the page"
    );
    // And the outline names it as its first.
    assert_eq!(under(&session, root_node.id, "First"), Some(json!({ "ref": [10, 0] })));
}

/// The outline is read back in the order it is in.
#[test]
fn the_outline_is_read_in_order() {
    let mut session = session();
    a_catalog(&mut session);
    for (number, title) in [(10, "One"), (11, "Two"), (12, "Three")] {
        session
            .execute("pdf.outline.add", &json!({ "title": title, "object": number }))
            .expect("the bookmark is added");
    }
    let outcome = session.execute("pdf.outline.read", &json!({})).expect("the outline is read");
    let items = outcome.data["items"].as_array().expect("there are items");
    assert_eq!(items.len(), 3, "there are three bookmarks");
    let titles: Vec<&str> =
        items.iter().filter_map(|item| item.get("title").and_then(|title| title.as_str())).collect();
    assert_eq!(titles, vec!["One", "Two", "Three"], "they are in order");
    // The **count** of the root is how many items are visible at all.
    let root = outcome.data["root"].as_u64().expect("the outline has a root");
    assert_eq!(under(&session, root, "Count"), Some(json!(3)), "the count is the count");
    // The **list** is relinked: each item names the next.
    let first = outcome.data["items"][0]["id"].as_u64().expect("the first item has an id");
    assert_eq!(under(&session, first, "Next"), Some(json!({ "ref": [11, 0] })), "the list is a list");
}

/// A bookmark may go **under** another bookmark, and the counts are
/// worked out again.
#[test]
fn a_bookmark_may_go_under_another() {
    let mut session = session();
    a_catalog(&mut session);
    let first = session
        .execute("pdf.outline.add", &json!({ "title": "Parent", "object": 10 }))
        .expect("the bookmark is added")
        .data["id"]
        .as_u64()
        .expect("the bookmark has an id");
    let child = session
        .execute("pdf.outline.add", &json!({ "title": "Child", "parent": first, "object": 11 }))
        .expect("the bookmark is added")
        .data["id"]
        .as_u64()
        .expect("the bookmark has an id");
    // The child hangs under the first.
    assert_eq!(
        under(&session, child, "Parent"),
        Some(json!({ "ref": [10, 0] })),
        "the child names its parent"
    );
    // And the parent names it as its first.
    assert_eq!(under(&session, first, "First"), Some(json!({ "ref": [11, 0] })));
    // The **count** of the parent is one: it shows one item.
    assert_eq!(under(&session, first, "Count"), Some(json!(1)), "the parent shows one");
    // And the root shows two: the parent and the child.
    let root = the_root(&session);
    assert_eq!(under(&session, root, "Count"), Some(json!(2)), "the root shows two");
    // The outline is read as a **tree**.
    let outcome = session.execute("pdf.outline.read", &json!({})).expect("the outline is read");
    let items = outcome.data["items"].as_array().expect("there are items");
    assert_eq!(items.len(), 1, "there is one top-level bookmark");
    assert_eq!(items[0]["title"], json!("Parent"));
    assert_eq!(items[0]["children"][0]["title"], json!("Child"), "the child is under it");
}

/// The **list** of siblings is relinked: the last names the new one as
/// its next, and the new one names the old last as its previous.
#[test]
fn the_list_of_siblings_is_relinked() {
    let mut session = session();
    a_catalog(&mut session);
    let first = session
        .execute("pdf.outline.add", &json!({ "title": "One", "object": 10 }))
        .expect("the bookmark is added")
        .data["id"]
        .as_u64()
        .expect("the bookmark has an id");
    let second = session
        .execute("pdf.outline.add", &json!({ "title": "Two", "object": 11 }))
        .expect("the bookmark is added")
        .data["id"]
        .as_u64()
        .expect("the bookmark has an id");
    // The first names the second as its next.
    assert_eq!(under(&session, first, "Next"), Some(json!({ "ref": [11, 0] })));
    // And the second names the first as its previous.
    assert_eq!(under(&session, second, "Prev"), Some(json!({ "ref": [10, 0] })));
    // The root names the second as its last, and the first as its
    // first.
    let root = the_root(&session);
    assert_eq!(under(&session, root, "First"), Some(json!({ "ref": [10, 0] })));
    assert_eq!(under(&session, root, "Last"), Some(json!({ "ref": [11, 0] })));
}

/// The **edges** are there as well as the values: a reference is two
/// things at once.
#[test]
fn the_edges_are_there_as_well_as_the_values() {
    let mut session = session();
    a_catalog(&mut session);
    session.execute("pdf.outline.add", &json!({ "title": "One" })).expect("the bookmark is added");
    let edges: Vec<_> = session
        .document()
        .relations
        .iter()
        .filter(|relation| relation.kind == "ref")
        .filter_map(|relation| {
            relation
                .payload
                .get("key")
                .and_then(|key| key.as_str())
                .map(|key| (relation.from, relation.to, key.to_string()))
        })
        .collect();
    assert!(edges.iter().any(|(_, _, key)| key == "First"), "the outline names its first item: {edges:?}");
    assert!(edges.iter().any(|(_, _, key)| key == "Last"), "and its last: {edges:?}");
}

/// A bookmark with no title, or under something that is not in an
/// outline, is refused.
#[test]
fn a_bookmark_that_cannot_be_is_refused() {
    let mut session = session();
    a_catalog(&mut session);
    let empty = session.execute("pdf.outline.add", &json!({ "title": "  " }));
    assert!(empty.is_err(), "a bookmark with no title is refused");
    let page = a_page(&mut session, 2);
    let outside = session.execute("pdf.outline.add", &json!({ "title": "One", "parent": page }));
    assert!(outside.is_err(), "a bookmark under a page is refused");
}

/// The outline **survives the round trip**: the file that is written
/// out has the bookmarks in it, and the file that comes back has them
/// too.
#[test]
fn the_outline_survives_the_round_trip() {
    let directory = craftkit_scratch::dir("craftkit-v2-outline-round").expect("the directory is made");
    let path = format!("{}/bookmarks.pdf", directory.to_string_lossy());
    let mut session = session();
    a_catalog(&mut session);
    let page = a_page(&mut session, 2);
    session
        .execute("pdf.outline.add", &json!({ "title": "Chapter one", "destination": page, "object": 10 }))
        .expect("the bookmark is added");
    session
        .execute("pdf.file.write", &json!({ "path": path.clone(), "root": 1 }))
        .expect("the file is written");
    let text = String::from_utf8_lossy(&std::fs::read(&path).expect("the file is there")).to_string();
    assert!(text.contains("/Outlines"), "the file has an outline: {text}");
    assert!(text.contains("/Title (Chapter one)"), "and a title: {text}");
    // And the file that comes back is read, and the outline is in it.
    let mut other = ecosystem().session();
    let into = format!("{}/streams", directory.to_string_lossy());
    other.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let outcome = other.execute("pdf.outline.read", &json!({})).expect("the outline is read");
    let items = outcome.data["items"].as_array().expect("there are items");
    assert_eq!(items.len(), 1, "the bookmark is there");
    assert_eq!(items[0]["title"], json!("Chapter one"), "with its title");
}
