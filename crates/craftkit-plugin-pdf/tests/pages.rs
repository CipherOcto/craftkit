//! The **page tree**: the pages of a PDF and what they inherit.
//!
//! A page may leave out what a page does not have to say: the media
//! box, the resources, the crop box and the rotation come from the
//! nearest ancestor that has them. The tree is two edges — the page
//! names its parent and the parent names the page.

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

/// A **page**, with the id it was given.
///
/// The page's own dictionary says what the page says for itself; the
/// rest it inherits.
fn a_page(
    session: &mut Session,
    object: u64,
    parent: Option<u64>,
    entries: craftkit_sdk::serde_json::Value,
) -> u64 {
    let mut params = json!({ "object": object, "entries": entries });
    if let Some(parent) = parent {
        params["parent"] = json!(parent);
    }
    session.execute("pdf.page.add", &params).expect("the page is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object")
        .last()
        .map(|node| node.id)
        .expect("the page is there")
}

/// The attributes of a page, as the command says them.
fn attributes_of(
    session: &mut Session,
    page: u64,
) -> (craftkit_sdk::serde_json::Value, craftkit_sdk::serde_json::Value) {
    let outcome =
        session.execute("pdf.page.attributes", &json!({ "page": page })).expect("the attributes are read");
    let attributes = outcome.data["attributes"].clone();
    let inherited = outcome.data["inherited"].clone();
    (attributes, inherited)
}

/// A page may be put in the document, and it says what it is.
#[test]
fn a_page_may_be_put_in_the_document() {
    let mut session = session();
    let id = a_page(&mut session, 1, None, json!([]));
    let node = session.document().node(id).expect("the page is there");
    let entries = node.data["entries"].as_array().expect("the page has a dictionary");
    let kind = entries
        .iter()
        .filter_map(|entry| entry.as_array())
        .find(|pair| pair.first().and_then(|key| key.as_str()) == Some("Type"))
        .and_then(|pair| pair.get(1).and_then(|value| value.as_str().map(|value| value.to_string())));
    assert_eq!(kind.as_deref(), Some("Page"), "the page says it is a page");
}

/// A page inherits what it does not say, and the **nearest** ancestor
/// that has it is the one it takes it from.
#[test]
fn a_page_inherits_what_it_does_not_say() {
    let mut session = session();
    let root = a_page(&mut session, 1, None, json!([]));
    session
        .execute(
            "pdf.object.add",
            &json!({ "object": 2, "entries": [["Type", "Pages"], ["MediaBox", [0, 0, 595, 842]]] }),
        )
        .expect("the parent is added");
    let parent = session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object")
        .last()
        .map(|node| node.id)
        .expect("the parent is there");
    let _ = root;
    // A page under the parent, saying nothing about its media box.
    let page = a_page(&mut session, 3, Some(parent), json!([]));
    let (attributes, inherited) = attributes_of(&mut session, page);
    assert_eq!(attributes["MediaBox"], json!([0, 0, 595, 842]), "the page inherits the media box");
    assert_eq!(inherited["MediaBox"], json!(parent), "the page says where it came from");
}

/// A page that says what it wants **keeps** it: the inheritance is for
/// what a page does not say.
#[test]
fn a_page_that_says_keeps_what_it_says() {
    let mut session = session();
    session
        .execute(
            "pdf.object.add",
            &json!({ "object": 2, "entries": [["Type", "Pages"], ["MediaBox", [0, 0, 595, 842]]] }),
        )
        .expect("the parent is added");
    let parent = session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object")
        .last()
        .map(|node| node.id)
        .expect("the parent is there");
    let page = a_page(&mut session, 3, Some(parent), json!([["MediaBox", [0, 0, 200, 200]]]));
    let (attributes, inherited) = attributes_of(&mut session, page);
    assert_eq!(attributes["MediaBox"], json!([0, 0, 200, 200]), "the page keeps its own media box");
    assert!(inherited.get("MediaBox").is_none(), "the page inherits nothing here");
}

/// A page with no parent inherits nothing, and a page whose type is
/// not a page is **refused**.
#[test]
fn a_page_with_no_parent_inherits_nothing() {
    let mut session = session();
    let page = a_page(&mut session, 1, None, json!([]));
    let (attributes, inherited) = attributes_of(&mut session, page);
    assert!(attributes.get("MediaBox").is_none(), "there is no media box to inherit");
    assert!(inherited.as_object().map(|map| map.is_empty()).unwrap_or(true), "nothing is inherited");
    let refused = session.execute("pdf.page.add", &json!({ "object": 2, "entries": [["Type", "XObject"]] }));
    assert!(refused.is_err(), "a page whose type is not a page is refused");
}
