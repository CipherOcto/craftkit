//! The **evaluators** of the layout battery: what the whole document
//! means.
//!
//! An evaluator is a gesture about the **whole document**: it may look at
//! every node and say something about all of them. It is a rule the kit
//! publishes as much as a command or a kind is, and an evaluator that is
//! only proved in another battery's tests is a rule that does not travel
//! with the thing it is about.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// The **pages** and the **frames** evaluators are there, and the layout
/// layer owns them.
#[test]
fn the_layout_evaluators_are_there() {
    let ecosystem = ecosystem();
    let pages = ecosystem.evaluators.get("layout.pages").expect("the pages evaluator is there");
    assert_eq!(pages.label, "Pages", "the evaluator is called Pages");
    assert_eq!(pages.layer, "layout", "and the layout layer owns it");
    let frames = ecosystem.evaluators.get("layout.frames").expect("the frames evaluator is there");
    assert_eq!(frames.label, "Frames", "the evaluator is called Frames");
    assert_eq!(frames.layer, "layout", "and the layout layer owns it");
}

/// The pages evaluator says how many **pages** a spread holds.
#[test]
fn the_pages_evaluator_says_the_pages_of_a_spread() {
    let mut session = session();
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    session
        .execute("layout.page.add", &json!({ "spread": spread, "name": "the page" }))
        .expect("the page is added");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "layout.pages").expect("the plan has the pages");
    assert_eq!(item.node, spread, "the item is about the spread");
    let pages = item.value["pages"].as_array().expect("the pages are a list");
    assert_eq!(pages.len(), 1, "the spread holds one page: {item:?}");
}

/// The frames evaluator says which **items** a story's text is set in.
#[test]
fn the_frames_evaluator_says_the_frames_of_a_story() {
    let mut session = session();
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let outcome = session
        .execute("layout.item.add", &json!({ "spread": spread, "name": "the item" }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute(
            "layout.story.add",
            &json!({ "document": document, "name": "the story", "text": "a line of text" }),
        )
        .expect("the story is added");
    let story = outcome.data["id"].as_u64().expect("the story has an id");
    session
        .execute("layout.item.story", &json!({ "item": item, "story": story }))
        .expect("the story is set in the item");
    let plan = session.evaluate();
    let item_of_the_plan =
        plan.items.iter().find(|item| item.kind == "layout.frames").expect("the plan has the frames");
    assert_eq!(item_of_the_plan.node, story, "the item is about the story");
    let frames = item_of_the_plan.value["frames"].as_array().expect("the frames are a list");
    assert_eq!(frames.len(), 1, "the story is set in one frame: {item_of_the_plan:?}");
    assert_eq!(frames[0]["item"], json!(item), "and it is that item");
}
