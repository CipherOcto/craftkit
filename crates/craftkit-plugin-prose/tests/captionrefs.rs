//! A cross-reference points at a **caption** and shows its number.
//!
//! The number of a caption is derived, and so is the text a reference
//! shows.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// A **document** with a caption in it, and the block of the caption.
fn a_document(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the bridge" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.caption", &json!({ "block": block, "label": "Figure", "text": "the bridge" }))
        .expect("the caption is said");
    (document, block)
}

/// The **text** a reference shows.
fn text_of(session: &mut Session, block: u64) -> String {
    let outcome = session
        .execute("prose.block.reference.read", &json!({ "block": block }))
        .expect("the reference is read");
    outcome.data["text"].as_str().unwrap_or_default().to_string()
}

/// A reference points at a **caption** and shows its **number**.
#[test]
fn a_reference_points_at_a_caption_and_shows_its_number() {
    let mut session = session();
    let (document, _first) = a_document(&mut session);
    let outcome = session
        .execute(
            "prose.block.reference.add",
            &json!({ "document": document, "target": "1", "to": "figure", "show": "number" }),
        )
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    assert_eq!(text_of(&mut session, block), "Figure 1", "the reference shows the number");
}

/// A reference that shows the **text** of a caption shows the text of
/// the caption.
#[test]
fn a_reference_shows_the_text_of_a_caption() {
    let mut session = session();
    let (document, _first) = a_document(&mut session);
    let outcome = session
        .execute(
            "prose.block.reference.add",
            &json!({ "document": document, "target": "the bridge", "to": "figure" }),
        )
        .expect("the reference is added");
    let block = outcome.data["block"].as_u64().expect("the reference has an id");
    assert_eq!(text_of(&mut session, block), "the bridge", "the reference shows the text");
}

/// The number of a caption is **derived**: a caption that is taken
/// away renumbers the ones after it, and the reference follows.
#[test]
fn the_number_of_a_caption_is_derived() {
    let mut session = session();
    let (document, first) = a_document(&mut session);
    // A second caption, so that the first is number one and the second
    // is number two.
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the rock" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.caption", &json!({ "block": block, "label": "Figure", "text": "the rock" }))
        .expect("the caption is said");
    let outcome = session
        .execute(
            "prose.block.reference.add",
            &json!({ "document": document, "target": "2", "to": "figure", "show": "number" }),
        )
        .expect("the reference is added");
    let reference = outcome.data["block"].as_u64().expect("the reference has an id");
    assert_eq!(text_of(&mut session, reference), "Figure 2", "the second caption is two");
    // The first caption is taken away, and the second is one.
    session.execute("prose.block.remove", &json!({ "block": first })).expect("the caption is taken away");
    assert_eq!(text_of(&mut session, reference), "Figure 1", "and the reference follows: the second is one");
}

/// A kind of target that is **not one of the tree's** is refused by
/// name, and so is a way of showing a reference that is not one of the
/// tree's two.
#[test]
fn a_kind_of_target_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (document, _first) = a_document(&mut session);
    let error = session
        .execute(
            "prose.block.reference.add",
            &json!({ "document": document, "target": "1", "to": "paragraph" }),
        )
        .expect_err("the kind is refused");
    assert!(error.to_string().contains("paragraph"), "the refusal names the kind: {error}");
    let error = session
        .execute(
            "prose.block.reference.add",
            &json!({ "document": document, "target": "1", "to": "figure", "show": "page" }),
        )
        .expect_err("the way is refused");
    assert!(error.to_string().contains("page"), "the refusal names the way: {error}");
}

/// A **bookmark** is still what a reference points at, and a bookmark
/// has no number.
#[test]
fn a_bookmark_is_still_what_a_reference_points_at() {
    let mut session = session();
    let (document, _first) = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a word" }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("prose.block.bookmark", &json!({ "block": block, "name": "here" }))
        .expect("the bookmark is said");
    let outcome = session
        .execute("prose.block.reference.add", &json!({ "document": document, "target": "here" }))
        .expect("the reference is added");
    let reference = outcome.data["block"].as_u64().expect("the reference has an id");
    assert_eq!(text_of(&mut session, reference), "a word", "the reference shows the text");
    let error = session
        .execute(
            "prose.block.reference.add",
            &json!({ "document": document, "target": "here", "show": "number" }),
        )
        .expect_err("the number is refused");
    assert!(error.to_string().contains("number"), "the refusal says why: {error}");
}
