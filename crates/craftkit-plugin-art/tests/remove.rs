//! The **removal** of an element.
//!
//! An element that is added may be taken away: the element goes, and
//! its edges go with it, because an edge to an element that is not
//! there is an edge to nothing.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_art::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the art battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A project with a scene, one video track and one element on it.
fn a_project() -> Session {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session.execute("art.project.add", &json!({ "name": "the film" })).expect("the project is added");
    session
        .execute("art.scene.add", &json!({ "project": 1, "name": "act one", "main": true }))
        .expect("the scene is added");
    session.execute("art.track.video.add", &json!({ "scene": 2, "name": "V1" })).expect("the track is added");
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": 3, "name": "the shot", "start": 48, "duration": 100, "media": "m_1" }),
        )
        .expect("the element is added");
    session
}

/// An element is **taken away**, and the track is left with what it has.
#[test]
fn an_element_is_taken_away() {
    let mut session = a_project();
    session.execute("art.element.remove", &json!({ "element": 4 })).expect("the element is taken away");
    assert!(session.document().node(4).is_none(), "the element is not there");
    let track = session.document().node(3).expect("the track is there");
    assert!(track.children.is_empty(), "and the track has no element on it");
}

/// The **length of the track** is derived, so an element that is taken
/// away shortens it.
#[test]
fn an_element_that_is_taken_away_shortens_the_track() {
    let mut session = a_project();
    session.execute("art.element.remove", &json!({ "element": 4 })).expect("the element is taken away");
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(0), "the track is as long as what is on it");
}

/// A removal of something that is **not an element** is refused.
#[test]
fn a_removal_of_something_that_is_not_an_element_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.element.remove", &json!({ "element": 3 }))
        .expect_err("a track is not an element");
    assert!(error.to_string().contains("not an element"), "the refusal says what is wrong: {error}");
    // And the track is still there.
    assert!(session.document().node(3).is_some(), "the track is still there");
}
