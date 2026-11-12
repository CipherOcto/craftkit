//! The **annotation**: the object that is on a page.
//!
//! An annotation is an object of its own — a dictionary with a type, a
//! kind, a rectangle, and somewhere to go — and the page names it in
//! its `/Annots`, which is an edge.

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
fn a_page(session: &mut Session, object: u64) -> u64 {
    session
        .execute("pdf.page.add", &json!({ "object": object, "entries": json!([]) }))
        .expect("the page is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "pdf.object")
        .last()
        .map(|node| node.id)
        .expect("the page is there")
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

/// The annotation is an **object** on the page, and the page names it.
#[test]
fn the_annotation_is_an_object_on_the_page() {
    let mut session = session();
    let page = a_page(&mut session, 1);
    let made = session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 5,
                "subtype": "Text",
                "rect": [10, 20, 110, 60],
                "contents": "a note",
            }),
        )
        .expect("the annotation is added");
    let annot = made.data["id"].as_u64().expect("the annotation has an id");
    let entries = entries_of(&session, annot);
    assert_eq!(entries[0], ("Type".to_string(), json!("Annot")), "it says what it is");
    assert_eq!(entries[1], ("Subtype".to_string(), json!("Text")), "and what kind it is");
    assert_eq!(entries[2], ("Rect".to_string(), json!([10.0, 20.0, 110.0, 60.0])), "and where it is");
    assert_eq!(entries[3], ("Contents".to_string(), json!({ "string": "a note" })), "and what it says");
    // The page **names** the annotation.
    let page_entries = entries_of(&session, page);
    let annots = page_entries
        .iter()
        .find(|(key, _)| key == "Annots")
        .and_then(|(_, value)| value.as_array().cloned())
        .expect("the page names its annotations");
    assert_eq!(annots, vec![json!({ "ref": [5, 0] })], "the annotation is in the list");
    // And the naming is an **edge**.
    let edge = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "ref" && relation.from == page && relation.to == annot)
        .expect("the page names the annotation");
    assert_eq!(edge.payload["key"], json!("Annots"), "the edge says under which key");
}

/// A link goes to **another page**, and the page it goes to is named.
#[test]
fn a_link_goes_to_another_page() {
    let mut session = session();
    let page = a_page(&mut session, 1);
    let other = a_page(&mut session, 2);
    session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 5,
                "subtype": "Link",
                "rect": [0, 0, 100, 100],
                "destination": other,
            }),
        )
        .expect("the link is added");
    let annot = session
        .document()
        .walk()
        .find(|node| node.data["object"].as_u64() == Some(5))
        .map(|node| node.id)
        .expect("the link is there");
    let entries = entries_of(&session, annot);
    let dest = entries.iter().find(|(key, _)| key == "Dest").map(|(_, value)| value.clone());
    assert_eq!(dest, Some(json!([{ "ref": [2, 0] }, "Fit"])), "the link says where it goes");
    let edge = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "ref" && relation.from == annot && relation.to == other)
        .expect("the link names the page");
    assert_eq!(edge.payload["key"], json!("Dest"), "and the edge says so");
}

/// A link goes to a **web address**, and the action is an object of its
/// own.
#[test]
fn a_link_goes_to_a_web_address() {
    let mut session = session();
    let page = a_page(&mut session, 1);
    session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 5,
                "subtype": "Link",
                "rect": [0, 0, 100, 100],
                "uri": "https://example.org/",
            }),
        )
        .expect("the link is added");
    let annot = session
        .document()
        .walk()
        .find(|node| node.data["object"].as_u64() == Some(5))
        .map(|node| node.id)
        .expect("the link is there");
    let entries = entries_of(&session, annot);
    let action = entries.iter().find(|(key, _)| key == "A").map(|(_, value)| value.clone());
    let action = action.expect("the link has an action");
    // The action is an **object**, and not a dictionary inside the
    // annotation: a third party may make an action of its own.
    let number = action["ref"][0].as_u64().expect("the action is a reference");
    let action_node = session
        .document()
        .walk()
        .find(|node| node.data["object"].as_u64() == Some(number))
        .expect("the action is there");
    let action_entries = entries_of(&session, action_node.id);
    assert_eq!(action_entries[0], ("S".to_string(), json!("URI")), "the action says what it is");
    assert_eq!(
        action_entries[1],
        ("URI".to_string(), json!({ "string": "https://example.org/" })),
        "and where it goes"
    );
}

/// The annotations of a page are **read**, and each one says where it
/// goes.
#[test]
fn the_annotations_of_a_page_are_read() {
    let mut session = session();
    let page = a_page(&mut session, 1);
    let other = a_page(&mut session, 2);
    session
        .execute(
            "pdf.annot.add",
            &json!({ "page": page, "object": 5, "subtype": "Link", "rect": [0, 0, 100, 100], "destination": other }),
        )
        .expect("the link is added");
    session
        .execute(
            "pdf.annot.add",
            &json!({
                "page": page,
                "object": 6,
                "subtype": "Text",
                "rect": [0, 200, 100, 300],
                "contents": "a note",
                "uri": "https://example.org/",
            }),
        )
        .expect("the note is added");
    let read =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    let annotations = read.data["annotations"].as_array().expect("there are annotations");
    assert_eq!(annotations.len(), 2, "the page has two annotations");
    assert_eq!(annotations[0]["Subtype"], json!("Link"), "the first is a link");
    assert_eq!(annotations[0]["goes_to"], json!({ "page": 2 }), "and it goes to the other page");
    assert_eq!(annotations[1]["Subtype"], json!("Text"), "the second is a note");
    assert_eq!(annotations[1]["goes_to"], json!({ "uri": "https://example.org/" }), "and it goes to the web");
}

/// An annotation that cannot be made is **refused**, and it says why.
#[test]
fn an_annotation_that_cannot_be_made_is_refused() {
    let mut session = session();
    let page = a_page(&mut session, 1);
    // A rectangle that is too small to be picked.
    let refused = session.execute(
        "pdf.annot.add",
        &json!({ "page": page, "object": 5, "subtype": "Text", "rect": [0, 0, 1, 1] }),
    );
    let message = match refused {
        Ok(_) => panic!("an annotation that cannot be picked is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("too small"), "the refusal says why: {message}");
    // A kind that is not a name.
    let refused = session.execute(
        "pdf.annot.add",
        &json!({ "page": page, "object": 5, "subtype": "two words", "rect": [0, 0, 100, 100] }),
    );
    assert!(refused.is_err(), "a kind that is not a name is refused");
    // An object number that is already in the document.
    let refused = session.execute(
        "pdf.annot.add",
        &json!({ "page": page, "object": 1, "subtype": "Text", "rect": [0, 0, 100, 100] }),
    );
    assert!(refused.is_err(), "an object number that is taken is refused");
    // Two places to go to.
    let refused = session.execute(
        "pdf.annot.add",
        &json!({
            "page": page,
            "object": 5,
            "subtype": "Link",
            "rect": [0, 0, 100, 100],
            "destination": page,
            "uri": "https://example.org/",
        }),
    );
    assert!(refused.is_err(), "an annotation goes to one place");
    // And a page that is not a page.
    session
        .execute("pdf.object.add", &json!({ "object": 9, "entries": [["Type", "XObject"]] }))
        .expect("the object is added");
    let refused = session.execute(
        "pdf.annot.add",
        &json!({ "page": 999, "object": 5, "subtype": "Text", "rect": [0, 0, 100, 100] }),
    );
    assert!(refused.is_err(), "there is no page to put it on");
}
