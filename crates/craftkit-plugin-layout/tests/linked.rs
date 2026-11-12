//! The **links** of the stories of a layout document.
//!
//! A story may be linked to another, and the link goes out of date
//! when the parent changes. The rules are the tree's own.

use craftkit_sdk::serde_json::json;
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

/// A **document** with **two** stories in it, and their ids.
fn a_document(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let mut stories = Vec::new();
    for (name, text) in [("the parent", "Original"), ("the child", "")] {
        let outcome = session
            .execute("layout.story.add", &json!({ "document": document, "name": name, "text": text }))
            .expect("the story is added");
        stories.push(outcome.data["id"].as_u64().expect("the story has an id"));
    }
    (document, stories[0], stories[1])
}

/// The **text** of a story, as the document holds it.
fn text_of(session: &Session, story: u64) -> String {
    session
        .document()
        .node(story)
        .and_then(|node| node.data.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or_default()
        .to_string()
}

/// The **links** of a document, as the answer holds them.
fn links_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("layout.document.links", &json!({ "document": document }))
        .expect("the links are read");
    outcome.data["links"].as_array().expect("the links are a list").clone()
}

/// A story is **linked** to a parent and takes its content.
#[test]
fn a_story_is_linked_to_a_parent() {
    let mut session = session();
    let (document, parent, child) = a_document(&mut session);
    let outcome = session
        .execute("layout.story.link", &json!({ "story": child, "parent": parent }))
        .expect("the story is linked");
    assert_eq!(outcome.data["parent"], json!(parent), "the parent is named: {outcome:?}");
    assert_eq!(text_of(&session, child), "Original", "the story takes the content of its parent");
    let links = links_of(&mut session, document);
    assert_eq!(links.len(), 1, "there is one link: {links:?}");
    assert_eq!(links[0]["story"], json!(child), "the story is the child: {links:?}");
    assert_eq!(links[0]["out_of_date"], json!(false), "and it is up to date: {links:?}");
}

/// The link goes **out of date** when the parent changes, and it is
/// **updated** when the parent says so.
#[test]
fn a_link_goes_out_of_date_and_is_updated() {
    let mut session = session();
    let (document, parent, child) = a_document(&mut session);
    session
        .execute("layout.story.link", &json!({ "story": child, "parent": parent }))
        .expect("the story is linked");
    // The parent changes.
    session
        .execute("layout.story.set", &json!({ "story": parent, "text": "Revised" }))
        .expect("the text is set");
    let links = links_of(&mut session, document);
    assert_eq!(links[0]["out_of_date"], json!(true), "the link is out of date: {links:?}");
    // The link is updated.
    let outcome = session.execute("layout.story.update", &json!({})).expect("the links are updated");
    assert_eq!(outcome.data["updated"], json!(1), "one link was updated: {outcome:?}");
    assert_eq!(text_of(&session, child), "Revised", "the story takes the content of its parent");
    let links = links_of(&mut session, document);
    assert_eq!(links[0]["out_of_date"], json!(false), "and it is up to date: {links:?}");
}

/// A link that is **not** out of date is not updated, and a link that
/// is named is updated whatever its state.
#[test]
fn a_link_that_is_not_out_of_date_is_not_updated() {
    let mut session = session();
    let (_document, parent, child) = a_document(&mut session);
    session
        .execute("layout.story.link", &json!({ "story": child, "parent": parent }))
        .expect("the story is linked");
    let outcome = session.execute("layout.story.update", &json!({})).expect("the links are updated");
    assert_eq!(outcome.data["updated"], json!(0), "nothing was updated: {outcome:?}");
    let outcome =
        session.execute("layout.story.update", &json!({ "story": child })).expect("the link is updated");
    assert_eq!(outcome.data["updated"], json!(1), "the named link is updated: {outcome:?}");
}

/// A story is **unlinked** and its content stays as it is.
#[test]
fn a_story_is_unlinked_and_keeps_its_content() {
    let mut session = session();
    let (document, parent, child) = a_document(&mut session);
    session
        .execute("layout.story.link", &json!({ "story": child, "parent": parent }))
        .expect("the story is linked");
    session.execute("layout.story.unlink", &json!({ "story": child })).expect("the story is unlinked");
    assert!(links_of(&mut session, document).is_empty(), "the link is gone");
    // The parent changes, and the story keeps what it holds.
    session
        .execute("layout.story.set", &json!({ "story": parent, "text": "Revised" }))
        .expect("the text is set");
    let outcome = session.execute("layout.story.update", &json!({})).expect("the links are updated");
    assert_eq!(outcome.data["updated"], json!(0), "nothing was updated: {outcome:?}");
}

/// A story that is **not there** is refused, and a story may not be
/// linked to itself.
#[test]
fn a_story_that_is_not_there_is_refused() {
    let mut session = session();
    let (_document, _parent, child) = a_document(&mut session);
    let error = session
        .execute("layout.story.link", &json!({ "story": child, "parent": 9999 }))
        .expect_err("the parent is refused");
    assert!(error.to_string().contains("there is no story"), "it says why: {error}");
    let error = session
        .execute("layout.story.link", &json!({ "story": child, "parent": child }))
        .expect_err("the story is refused");
    assert!(error.to_string().contains("may not be linked to itself"), "it says why: {error}");
}
