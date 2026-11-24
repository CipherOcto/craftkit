//! The **limits of the layout battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

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

/// A document with a spread, a page and an item in it: the ids of the
/// document and of the item.
fn a_document_with_an_item(session: &mut Session) -> (u64, u64) {
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
    let outcome = session
        .execute("layout.item.add", &json!({ "spread": spread, "name": "the item" }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (document, item)
}

/// A group of **more items** than `MAX_ITEMS` allows is refused.
///
/// The items come from outside, and a list of a thousand and one ids is
/// refused rather than half-grouped.
#[test]
fn a_group_with_too_many_items_is_refused() {
    let mut session = session();
    let items: Vec<u64> = (0..=1000).collect();
    let error = session
        .execute("layout.item.group", &json!({ "items": items }))
        .expect_err("there are too many items");
    assert!(error.to_string().contains("the most there may be"), "{error}");
}

/// A stroke **weight** over `MAX_WEIGHT` is refused.
///
/// A weight is a number from zero to the limit, and a number outside it
/// is not a weight.
#[test]
fn a_stroke_weight_over_the_limit_is_refused() {
    let mut session = session();
    let (_, item) = a_document_with_an_item(&mut session);
    let error = session
        .execute("layout.item.stroke", &json!({ "item": item, "weight": 1001.0 }))
        .expect_err("that weight is too heavy");
    assert!(error.to_string().contains("is not a weight"), "{error}");
}

/// A bookmark **name** over `MAX_NAME` is refused.
#[test]
fn a_bookmark_name_over_the_limit_is_refused() {
    let mut session = session();
    let (document, _) = a_document_with_an_item(&mut session);
    let name = "n".repeat(257);
    let error = session
        .execute("layout.bookmark.add", &json!({ "document": document, "name": name }))
        .expect_err("that name is too long");
    assert!(error.to_string().contains("longer than"), "{error}");
}

/// A link **target** over `MAX_TARGET` is refused.
#[test]
fn a_link_target_over_the_limit_is_refused() {
    let mut session = session();
    let (_, item) = a_document_with_an_item(&mut session);
    let target = format!("https://example.com/{}", "a".repeat(2049));
    let error = session
        .execute("layout.item.link", &json!({ "item": item, "target": target }))
        .expect_err("that target is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}
