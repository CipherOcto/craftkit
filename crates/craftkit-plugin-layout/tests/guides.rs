//! The **guides** of a page.
//!
//! A guide is a line that is drawn on a page to help with the placing
//! of things, and the rules are the tree's own.

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

/// A **document** with a **spread** and **two** pages of a hundred
/// millimetres in it, and their ids.
fn a_document(session: &mut Session) -> (u64, u64, u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut pages = Vec::new();
    for name in ["first", "second"] {
        let outcome = session
            .execute(
                "layout.page.add",
                &json!({ "spread": spread, "name": name, "width": 100, "height": 200 }),
            )
            .expect("the page is added");
        pages.push(outcome.data["id"].as_u64().expect("the page has an id"));
    }
    (document, spread, pages[0], pages[1])
}

/// The **guides** of a page, as the answer holds them.
fn guides_of(session: &mut Session, page: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("layout.page.guides", &json!({ "page": page })).expect("the guides are read");
    outcome.data["guides"].as_array().expect("the guides are a list").clone()
}

/// A guide is put on a page and stands where it is told.
#[test]
fn a_guide_is_put_on_a_page() {
    let mut session = session();
    let (_document, _spread, page, _other) = a_document(&mut session);
    let outcome = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "vertical", "position": 40 }))
        .expect("the guide is added");
    let guide = outcome.data["guides"][0]["id"].as_u64().expect("the guide has an id");
    let guides = guides_of(&mut session, page);
    assert_eq!(guides.len(), 1, "there is one guide: {guides:?}");
    assert_eq!(guides[0]["orientation"], json!("vertical"), "it is vertical: {guides:?}");
    assert_eq!(guides[0]["position"], json!(40.0), "and it stands where it was told: {guides:?}");
    assert_eq!(guides[0]["index"], json!(0), "and it is the first: {guides:?}");
    assert_eq!(outcome.data["guides"][0]["id"], json!(guide), "the answer names it: {outcome:?}");
}

/// A guide in **both** directions is two guides.
#[test]
fn a_guide_in_both_directions_is_two_guides() {
    let mut session = session();
    let (_document, _spread, page, _other) = a_document(&mut session);
    let outcome = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "both", "x": 30, "y": 20 }))
        .expect("the guides are added");
    assert_eq!(outcome.data["count"], json!(2), "there are two: {outcome:?}");
    let guides = guides_of(&mut session, page);
    assert_eq!(guides.len(), 2, "and the page holds two: {guides:?}");
    assert_eq!(guides[0]["orientation"], json!("horizontal"), "the first is across: {guides:?}");
    assert_eq!(guides[0]["position"], json!(20.0), "at the y: {guides:?}");
    assert_eq!(guides[1]["orientation"], json!("vertical"), "the second is down: {guides:?}");
    assert_eq!(guides[1]["position"], json!(30.0), "at the x: {guides:?}");
}

/// A guide that is not one is **refused**: an orientation that is not
/// known, a position that is not given, and a position that is out of
/// its bounds.
#[test]
fn a_guide_that_is_not_one_is_refused() {
    let mut session = session();
    let (_document, _spread, page, _other) = a_document(&mut session);
    let error = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "diagonal", "position": 1 }))
        .expect_err("the orientation is refused");
    assert!(error.to_string().contains("orientation"), "it says why: {error}");
    let error = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "vertical" }))
        .expect_err("the missing position is refused");
    assert!(error.to_string().contains("needs a position"), "it says why: {error}");
    let error = session
        .execute(
            "layout.guide.add",
            &json!({ "page": page, "orientation": "vertical", "position": 1_000_001 }),
        )
        .expect_err("the position is refused");
    assert!(error.to_string().contains("between -1000000 and 1000000"), "it says why: {error}");
}

/// A guide that is given no page is put on the page that stands
/// **under** its coordinate, and a spread guide is put on the **first**
/// page of its spread.
#[test]
fn the_page_under_the_coordinate_takes_the_guide() {
    let mut session = session();
    let (_document, spread, first, second) = a_document(&mut session);
    session
        .execute("layout.guide.add", &json!({ "spread": spread, "orientation": "vertical", "position": 50 }))
        .expect("the guide is added");
    assert_eq!(guides_of(&mut session, first).len(), 1, "the first page holds the guide");
    session
        .execute("layout.guide.add", &json!({ "spread": spread, "orientation": "vertical", "position": 150 }))
        .expect("the guide is added");
    assert_eq!(guides_of(&mut session, second).len(), 1, "the second page holds the guide");
    // A **spread guide** crosses the pasteboard, and it stands on the
    // first page.
    let outcome = session
        .execute(
            "layout.guide.add",
            &json!({ "spread": spread, "orientation": "horizontal", "position": 20, "spread_guide": true }),
        )
        .expect("the guide is added");
    assert_eq!(outcome.data["guides"][0]["page"], json!(first), "it stands on the first page: {outcome:?}");
    assert_eq!(outcome.data["guides"][0]["spread"], json!(true), "and it is a spread guide: {outcome:?}");
}

/// A **liquid** guide is never a spread guide, as in the tree.
#[test]
fn a_liquid_guide_is_never_a_spread_guide() {
    let mut session = session();
    let (_document, spread, _first, _second) = a_document(&mut session);
    let outcome = session
        .execute(
            "layout.guide.add",
            &json!({
                "spread": spread,
                "orientation": "vertical",
                "position": 20,
                "spread_guide": true,
                "liquid": true
            }),
        )
        .expect("the guide is added");
    assert_eq!(outcome.data["guides"][0]["spread"], json!(true), "it crosses the pasteboard: {outcome:?}");
    assert_eq!(outcome.data["guides"][0]["liquid"], json!(false), "and it is not liquid: {outcome:?}");
}

/// A guide is **moved** and **deleted**.
#[test]
fn a_guide_is_moved_and_deleted() {
    let mut session = session();
    let (_document, _spread, page, _other) = a_document(&mut session);
    let outcome = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "vertical", "position": 40 }))
        .expect("the guide is added");
    let guide = outcome.data["guides"][0]["id"].as_u64().expect("the guide has an id");
    session
        .execute("layout.guide.move", &json!({ "guide": guide, "position": 60 }))
        .expect("the guide is moved");
    let guides = guides_of(&mut session, page);
    assert_eq!(guides[0]["position"], json!(60.0), "it stands where it was moved: {guides:?}");
    session.execute("layout.guide.delete", &json!({ "guide": guide })).expect("the guide is deleted");
    assert!(guides_of(&mut session, page).is_empty(), "and it is gone");
}

/// Every guide of a **spread** is deleted at once.
#[test]
fn the_guides_of_a_spread_are_deleted_at_once() {
    let mut session = session();
    let (_document, spread, first, second) = a_document(&mut session);
    for page in [first, second] {
        session
            .execute("layout.guide.add", &json!({ "page": page, "orientation": "vertical", "position": 20 }))
            .expect("the guide is added");
    }
    let outcome = session
        .execute("layout.guide.delete.all", &json!({ "spread": spread }))
        .expect("the guides are deleted");
    assert_eq!(outcome.data["deleted"], json!(2), "both are gone: {outcome:?}");
    assert!(guides_of(&mut session, first).is_empty(), "the first page holds none");
    assert!(guides_of(&mut session, second).is_empty(), "and the second holds none");
}

/// A guide on a **locked** layer may not be moved, as in the tree.
#[test]
fn a_guide_on_a_locked_layer_may_not_be_moved() {
    let mut session = session();
    let (document, _spread, page, _other) = a_document(&mut session);
    let outcome = session
        .execute("layout.guide.add", &json!({ "page": page, "orientation": "vertical", "position": 40 }))
        .expect("the guide is added");
    let guide = outcome.data["guides"][0]["id"].as_u64().expect("the guide has an id");
    let outcome = session
        .execute("layout.layer.add", &json!({ "document": document, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    session
        .execute("core.layer.on", &json!({ "node": guide, "layer": layer }))
        .expect("the guide is on the layer");
    session.execute("layout.layer.lock", &json!({ "layer": layer })).expect("the layer is locked");
    let error = session
        .execute("layout.guide.move", &json!({ "guide": guide, "position": 60 }))
        .expect_err("the guide may not be moved");
    assert!(error.to_string().contains("locked or hidden"), "it says why: {error}");
}
