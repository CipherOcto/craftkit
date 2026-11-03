//! The **markers** of a composition.
//!
//! A marker is a moment with something said about it, and the markers
//! of a composition stand in the order of their time.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_fx::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the fx battery.
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

/// A **composition**, and its id.
fn a_comp(session: &mut Session) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    outcome.data["id"].as_u64().expect("the comp has an id")
}

/// The **markers** of a composition, read from the plan.
fn markers_of(session: &Session, comp: u64) -> Vec<(u64, String)> {
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == comp && item.kind == "fx.markers")
        .expect("the comp is in the plan");
    item.value["markers"]
        .as_array()
        .expect("the markers are there")
        .iter()
        .map(|marker| {
            (
                marker["frame"].as_u64().expect("the marker has a frame"),
                marker["comment"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

/// A marker may be put in a **composition**.
#[test]
fn a_marker_may_be_put_in_a_composition() {
    let mut session = session();
    let comp = a_comp(&mut session);
    session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 12, "comment": "the beat" }))
        .expect("the marker is added");
    let markers = markers_of(&session, comp);
    assert_eq!(markers.len(), 1, "the composition has the marker");
    assert_eq!(markers[0], (12, "the beat".to_string()), "at the frame, with the comment");
}

/// The markers stand in the order of their **time**, as the tree's
/// command sorts them.
#[test]
fn the_markers_stand_in_the_order_of_their_time() {
    let mut session = session();
    let comp = a_comp(&mut session);
    session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 30, "comment": "the third" }))
        .expect("the marker is added");
    session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 10, "comment": "the first" }))
        .expect("the marker is added");
    session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 20, "comment": "the second" }))
        .expect("the marker is added");
    let markers = markers_of(&session, comp);
    let comments: Vec<String> = markers.into_iter().map(|(_, comment)| comment).collect();
    assert_eq!(comments, vec!["the first", "the second", "the third"], "the markers are in time order");
}

/// A marker with **no comment** says nothing, and that is allowed.
#[test]
fn a_marker_with_no_comment_says_nothing() {
    let mut session = session();
    let comp = a_comp(&mut session);
    session.execute("fx.marker.add", &json!({ "comp": comp, "frame": 5 })).expect("the marker is added");
    let markers = markers_of(&session, comp);
    assert_eq!(markers[0].1, "", "the marker says nothing");
}

/// A marker may be **moved**.
#[test]
fn a_marker_may_be_moved() {
    let mut session = session();
    let comp = a_comp(&mut session);
    let outcome = session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 12, "comment": "the beat" }))
        .expect("the marker is added");
    let marker = outcome.data["id"].as_u64().expect("the marker has an id");
    session.execute("fx.marker.set", &json!({ "marker": marker, "frame": 40 })).expect("the marker moves");
    let markers = markers_of(&session, comp);
    assert_eq!(markers[0].0, 40, "the marker is at the new frame");
}

/// A marker's **comment** may be changed.
#[test]
fn a_markers_comment_may_be_changed() {
    let mut session = session();
    let comp = a_comp(&mut session);
    let outcome = session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 12, "comment": "the beat" }))
        .expect("the marker is added");
    let marker = outcome.data["id"].as_u64().expect("the marker has an id");
    session
        .execute("fx.marker.set", &json!({ "marker": marker, "comment": "the downbeat" }))
        .expect("the comment changes");
    let markers = markers_of(&session, comp);
    assert_eq!(markers[0].1, "the downbeat", "the marker says the new thing");
}

/// A frame that is **not a frame** is refused.
#[test]
fn a_frame_that_is_not_a_frame_is_refused() {
    let mut session = session();
    let comp = a_comp(&mut session);
    let error = session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 1.5 }))
        .expect_err("a frame is a whole number");
    assert!(error.to_string().contains("whole number"), "the error says so: {error}");
}

/// A composition that is **not there** is refused.
#[test]
fn a_composition_that_is_not_there_is_refused() {
    let mut session = session();
    let comp = a_comp(&mut session);
    let error = session
        .execute("fx.marker.add", &json!({ "comp": comp + 1000, "frame": 1 }))
        .expect_err("there is no such composition");
    assert!(error.to_string().contains("no composition"), "the error says so: {error}");
}

/// The marker is **saved** with the composition.
#[test]
fn the_marker_is_saved_with_the_composition() {
    let mut session = session();
    let comp = a_comp(&mut session);
    session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 12, "comment": "the beat" }))
        .expect("the marker is added");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(comp).expect("the comp is there");
    let marker = node.children.first().expect("the marker is in the comp");
    assert_eq!(marker.data["frame"].as_u64(), Some(12), "the marker is still at its frame");
    assert_eq!(marker.data["comment"].as_str(), Some("the beat"), "and says its comment");
}
