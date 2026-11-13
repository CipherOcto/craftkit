//! The **edge is in the dictionary**: a reference is two things at once.
//!
//! A PDF file is written from the dictionaries of its objects. A
//! reference that is only an edge in the document is a reference the file
//! does not have — and a page tree that is only in the document is a
//! tree the file cannot show. So the reference is put in the dictionary
//! as well, and the count of the children is **derived** from the edges.

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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-refs-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
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

/// The id of the object with an object **number**.
fn the_object(session: &Session, number: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|value| value.as_u64()) == Some(number))
        .map(|node| node.id)
        .expect("the object is there")
}

/// A reference is in the **dictionary**, and not only in the document.
#[test]
fn a_reference_is_in_the_dictionary() {
    let mut session = session();
    for number in [1, 2] {
        session
            .execute("pdf.object.add", &json!({ "object": number, "entries": [] }))
            .expect("the object is added");
    }
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Resources", "to": 2 }))
        .expect("the reference is added");
    let first = the_object(&session, 1);
    let second = the_object(&session, 2);
    // The edge is in the document.
    assert_eq!(session.document().relations.len(), 1, "the edge is there");
    // And the **value** is in the dictionary, so that the file has it.
    assert_eq!(
        under(&session, first, "Resources"),
        Some(json!({ "ref": [2, 0] })),
        "the dictionary holds the reference"
    );
    // The object that is named does not hold the reference: it is the
    // one that is named that holds it.
    assert_eq!(under(&session, second, "Resources"), None, "the other side does not");
}

/// A key that holds a **list** of references grows, and one that holds a
/// single reference is set.
#[test]
fn a_list_key_grows_and_a_single_key_is_set() {
    let mut session = session();
    for number in [1, 2, 3] {
        session
            .execute("pdf.object.add", &json!({ "object": number, "entries": [] }))
            .expect("the object is added");
    }
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Kids", "to": 2, "list": true }))
        .expect("the first reference is added");
    session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Kids", "to": 3, "list": true }))
        .expect("the second reference is added");
    let first = the_object(&session, 1);
    assert_eq!(
        under(&session, first, "Kids"),
        Some(json!([{ "ref": [2, 0] }, { "ref": [3, 0] }])),
        "the list grew"
    );
    // A key that holds one reference is **set**, and not grown.
    session
        .execute("pdf.ref.add", &json!({ "from": 2, "key": "Parent", "to": 3 }))
        .expect("the reference is added");
    let second = the_object(&session, 2);
    assert_eq!(under(&session, second, "Parent"), Some(json!({ "ref": [3, 0] })), "the key is set");
}

/// A **page tree** says itself in the dictionary: the page names its
/// parent, the parent names its children, and the count of the children
/// is how many there are.
#[test]
fn a_page_tree_says_itself_in_the_dictionary() {
    let mut session = session();
    session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Pages"]] }))
        .expect("the root is added");
    let root = the_object(&session, 1);
    for number in [2, 3] {
        session
            .execute("pdf.page.add", &json!({ "object": number, "entries": [], "parent": root }))
            .expect("the page is added");
    }
    let first = the_object(&session, 2);
    let second = the_object(&session, 3);
    // The page names its parent.
    assert_eq!(under(&session, first, "Parent"), Some(json!({ "ref": [1, 0] })), "the page names it");
    // The parent names its children, in order.
    assert_eq!(
        under(&session, root, "Kids"),
        Some(json!([{ "ref": [2, 0] }, { "ref": [3, 0] }])),
        "the parent names them"
    );
    // And the count of the children is **derived**: it is how many of
    // them there are.
    assert_eq!(under(&session, root, "Count"), Some(json!(2)), "the count is the count");
    // The second page does not change the count of the first.
    assert_eq!(under(&session, second, "Count"), None, "a page has no count of its own");
}

/// The page tree is in the **file** that is written out.
#[test]
fn the_page_tree_is_in_the_file() {
    let directory = directory("file");
    let mut session = session();
    session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Catalog"]] }))
        .expect("the catalog is added");
    session
        .execute("pdf.object.add", &json!({ "object": 2, "entries": [["Type", "Pages"]] }))
        .expect("the root is added");
    let root = the_object(&session, 2);
    session
        .execute("pdf.page.add", &json!({ "object": 3, "entries": [], "parent": root }))
        .expect("the page is added");
    let path = format!("{directory}/tree.pdf");
    session
        .execute("pdf.file.write", &json!({ "path": path.clone(), "root": 1 }))
        .expect("the file is written");
    let text = String::from_utf8_lossy(&std::fs::read(&path).expect("the file is there")).to_string();
    assert!(text.contains("/Parent 2 0 R"), "the page names its parent: {text}");
    assert!(text.contains("/Kids [3 0 R]"), "the parent names its child: {text}");
    assert!(text.contains("/Count 1"), "the count is in the file: {text}");
}

/// A reference to an object that is not in the document is still
/// refused: the edge and the value are made together, or not at all.
#[test]
fn a_reference_to_nothing_is_refused() {
    let mut session = session();
    session.execute("pdf.object.add", &json!({ "object": 1, "entries": [] })).expect("the object is added");
    let error = session
        .execute("pdf.ref.add", &json!({ "from": 1, "key": "Kids", "to": 9 }))
        .expect_err("there is no object 9");
    assert!(error.to_string().contains("no object"), "{error}");
}
