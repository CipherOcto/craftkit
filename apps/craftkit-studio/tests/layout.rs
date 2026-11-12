//! The layout battery: a page layout document is a **set of spreads**,
//! each with **pages** and a flat list of **items**, beside **layers**,
//! **stories** and an **asset table**.
//!
//! What is proved here: the hand-written id fields are **edges**, a
//! relation's **inverse is derived**, the kinds of item are **open** and
//! the caller names them, an asset's bytes are a **stream**, and the
//! pages of a spread are derived.

use craftkit_sdk::Ecosystem;
#[cfg(feature = "layout")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A document with a spread, a layer and a story.
#[cfg(feature = "layout")]
fn a_document() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session.execute("layout.document.add", &json!({ "name": "the issue" })).expect("the document is added");
    session
        .execute("layout.spread.add", &json!({ "document": 1, "name": "pages 1-2" }))
        .expect("the spread is added");
    session
        .execute("layout.layer.add", &json!({ "document": 1, "name": "Background", "color": "grey" }))
        .expect("the layer is added");
    session
        .execute("layout.story.add", &json!({ "document": 1, "name": "the body", "text": "the quick fox" }))
        .expect("the story is added");
    session
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "layout")]
#[test]
fn the_layout_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("layout.document.add").is_some(), "the battery is in");
    assert!(ecosystem.kinds.get("layout.item.frame").is_some());
    assert!(ecosystem.evaluators.get("layout.pages").is_some());
    assert!(ecosystem.evaluators.get("layout.frames").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "layout"))]
#[test]
fn the_layout_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("layout.document.add").is_none(), "the battery is out");
}

/// The kinds of item are **open**, and the **caller** names them.
#[cfg(feature = "layout")]
#[test]
fn the_item_kinds_are_open_and_the_caller_names_them() {
    let mut session = a_document();
    for (kind, label) in [("layout.item.frame", "the headline"), ("layout.item.graphic", "the picture")] {
        session
            .execute("layout.item.add", &json!({ "spread": 2, "name": label, "kind": kind }))
            .expect("the item is added");
    }
    let items: Vec<_> =
        session.document().walk().filter(|node| node.kind.starts_with("layout.item.")).collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].kind, "layout.item.frame");
    assert_eq!(items[1].kind, "layout.item.graphic");
}

/// And a kind that is not a well-formed id is refused.
#[cfg(feature = "layout")]
#[test]
fn a_kind_that_is_not_an_id_is_refused() {
    let mut session = a_document();
    let error = session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "a thing", "kind": "Not An Id" }))
        .expect_err("that is not an id");
    assert!(error.to_string().contains("contribution id"), "{error}");
}

/// An item's **layer** is an edge, and not a parent.
#[cfg(feature = "layout")]
#[test]
fn an_items_layer_is_an_edge() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    session.execute("layout.item.layer", &json!({ "item": 5, "layer": 3 })).expect("it is on the layer");
    let edge = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == craftkit_sdk::layer::ON_LAYER)
        .expect("there is an edge");
    assert_eq!(edge.from, 5, "the edge is from the item");
    assert_eq!(edge.to, 3, "the edge is to the layer");
    assert_eq!(edge.payload["layer"], json!("Background"), "and it says which layer");
    // The item is still in the spread: the layer did not take it over.
    let spread = session.document().node(2).expect("the spread is there");
    assert_eq!(spread.children.len(), 1, "the item is in the spread");
}

/// An item's **story** is an edge too.
#[cfg(feature = "layout")]
#[test]
fn an_items_story_is_an_edge() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    session.execute("layout.item.story", &json!({ "item": 5, "story": 4 })).expect("it shows the story");
    let edge = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "layout.text")
        .expect("there is an edge");
    assert_eq!(edge.from, 5);
    assert_eq!(edge.to, 4);
    assert_eq!(edge.payload["story"], json!("the body"));
}

/// The **inverse** of the edge is derived: the story knows its frames
/// because the edges say so, and not because a field says so.
#[cfg(feature = "layout")]
#[test]
fn the_storys_frames_are_derived() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    session.execute("layout.item.story", &json!({ "item": 5, "story": 4 })).expect("it shows the story");
    let plan = session.evaluate();
    let frames: Vec<_> = plan.items.iter().filter(|item| item.kind == "layout.frames").collect();
    assert_eq!(frames[0].node, 4, "it is about the story");
    let list = frames[0].value["frames"].as_array().expect("a list of frames");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["item"], json!(5));
    assert_eq!(list[0]["name"], json!("the headline"));
}

/// And it follows the edge when a second frame is added.
#[cfg(feature = "layout")]
#[test]
fn the_storys_frames_follow_the_edges() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    session.execute("layout.item.story", &json!({ "item": 5, "story": 4 })).expect("it shows the story");
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the caption" }))
        .expect("the item is added");
    session.execute("layout.item.story", &json!({ "item": 6, "story": 4 })).expect("it shows the story");
    let plan = session.evaluate();
    let frames: Vec<_> = plan.items.iter().filter(|item| item.kind == "layout.frames").collect();
    let list = frames[0].value["frames"].as_array().expect("a list of frames");
    assert_eq!(list.len(), 2, "both frames are there");
}

/// The **pages** of a spread are derived, in order.
#[cfg(feature = "layout")]
#[test]
fn the_pages_are_derived() {
    let mut session = a_document();
    session.execute("layout.page.add", &json!({ "spread": 2, "name": "left" })).expect("the page is added");
    session
        .execute("layout.page.add", &json!({ "spread": 2, "name": "right", "width": 200.0, "height": 300.0 }))
        .expect("the page is added");
    let plan = session.evaluate();
    let pages: Vec<_> = plan.items.iter().filter(|item| item.kind == "layout.pages").collect();
    assert_eq!(pages[0].node, 2, "it is about the spread");
    let list = pages[0].value["pages"].as_array().expect("a list of pages");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["name"], json!("left"));
    assert_eq!(list[1]["name"], json!("right"));
    assert_eq!(list[1]["width"], json!(200.0));
    assert_eq!(list[1]["height"], json!(300.0));
}

/// The **asset** is a stream: the file is the node's stream, and the
/// bytes travel beside the document in the package.
#[cfg(feature = "layout")]
#[test]
fn the_asset_is_a_stream() {
    let mut session = a_document();
    session
        .execute(
            "layout.asset.add",
            &json!({ "document": 1, "name": "the logo", "path": "assets/logo.eps", "kind": "eps" }),
        )
        .expect("the asset is added");
    let asset = session.document().node(5).expect("the asset is there");
    assert_eq!(asset.kind, "layout.asset");
    assert_eq!(asset.stream.as_deref(), Some("assets/logo.eps"));
    assert_eq!(asset.data["kind"], json!("eps"));
}

/// A **story** is a string and its runs are intervals: the text is
/// stored once.
#[cfg(feature = "layout")]
#[test]
fn the_story_is_the_text_stored_once() {
    let session = a_document();
    let story = session.document().node(4).expect("the story is there");
    assert_eq!(story.data["text"], json!("the quick fox"));
    let plan = session.evaluate();
    let stories: Vec<_> = plan.items.iter().filter(|item| item.kind == "layout.story").collect();
    assert_eq!(stories[0].value["characters"], json!(13));
}

/// An item put on a layer twice is refused: the edge would be said
/// twice.
#[cfg(feature = "layout")]
#[test]
fn an_item_on_a_layer_twice_is_refused() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    session.execute("layout.item.layer", &json!({ "item": 5, "layer": 3 })).expect("it is on the layer");
    let error = session
        .execute("layout.item.layer", &json!({ "item": 5, "layer": 3 }))
        .expect_err("it is already on the layer");
    assert!(error.to_string().contains("already on the layer"), "{error}");
}

/// A story of something that is not a story is refused.
#[cfg(feature = "layout")]
#[test]
fn a_story_of_something_that_is_not_a_story_is_refused() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    let error = session
        .execute("layout.item.story", &json!({ "item": 5, "story": 3 }))
        .expect_err("a layer is not a story");
    assert!(error.to_string().contains("not a story"), "{error}");
}

/// A size that is not a size is refused.
#[cfg(feature = "layout")]
#[test]
fn a_page_that_is_not_a_page_size_is_refused() {
    let mut session = a_document();
    let error = session
        .execute("layout.page.add", &json!({ "spread": 2, "name": "left", "width": 0.0 }))
        .expect_err("a width is not zero");
    assert!(error.to_string().contains("above zero"), "{error}");
}

/// The undo takes the edge with it.
#[cfg(feature = "layout")]
#[test]
fn the_undo_takes_the_edge_with_it() {
    let mut session = a_document();
    session
        .execute("layout.item.add", &json!({ "spread": 2, "name": "the headline" }))
        .expect("the item is added");
    session.execute("layout.item.layer", &json!({ "item": 5, "layer": 3 })).expect("it is on the layer");
    assert_eq!(session.document().relations.len(), 1);
    session.undo().expect("the undo works");
    assert_eq!(session.document().relations.len(), 0, "the edge is gone");
    session.undo().expect("the undo works again");
    assert_eq!(session.document().walk().count(), 4, "and so is the item");
}
