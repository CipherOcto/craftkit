//! The **move** of an element.
//!
//! An element may be moved on the timeline, and it may be moved to
//! another track. A move that crosses a track moves the **element** —
//! the element keeps its id, its name, its media and its length — and
//! the tree's containment is the truth about which track it is on.

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

/// A project with a scene, two video tracks and one element on the
/// first of them.
fn a_project() -> Session {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    session.execute("art.project.add", &json!({ "name": "the film" })).expect("the project is added");
    session
        .execute("art.scene.add", &json!({ "project": 1, "name": "act one", "main": true }))
        .expect("the scene is added");
    session.execute("art.track.video.add", &json!({ "scene": 2, "name": "V1" })).expect("the track is added");
    session.execute("art.track.video.add", &json!({ "scene": 2, "name": "V2" })).expect("the track is added");
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": 3, "name": "the shot", "start": 48, "duration": 100, "media": "m_1" }),
        )
        .expect("the element is added");
    session
}

/// An element is **moved on the timeline**: its start changes and
/// nothing else does.
#[test]
fn an_element_is_moved_on_the_timeline() {
    let mut session = a_project();
    session.execute("art.element.move", &json!({ "element": 5, "to": 120 })).expect("the element is moved");
    let element = session.document().node(5).expect("the element is there");
    assert_eq!(element.data["start"], json!(120), "the element starts later");
    assert_eq!(element.name, "the shot", "the element is the same element");
    assert_eq!(element.data["media"], json!("m_1"), "the element plays the same media");
}

/// An element is **moved to another track**, and it keeps everything it
/// was.
#[test]
fn an_element_is_moved_to_another_track() {
    let mut session = a_project();
    session
        .execute("art.element.move", &json!({ "element": 5, "to": 10, "track": 4 }))
        .expect("the element is moved");
    let element = session.document().node(5).expect("the element is there");
    assert_eq!(element.id, 5, "the element is the same element");
    assert_eq!(element.data["start"], json!(10));
    // The containment is the truth: the element is on the second track.
    let track = session.document().node(4).expect("the second track is there");
    assert_eq!(track.kind, "art.track.video");
    let children: Vec<u64> = track.children.iter().map(|child| child.id).collect();
    assert_eq!(children, vec![5], "the element is on the track it was moved to");
    // And it is not on the first one any more.
    let first = session.document().node(3).expect("the first track is there");
    assert!(first.children.iter().all(|child| child.id != 5), "the element left the track it was on");
}

/// The **length** of an element is derived, so a move does not change
/// it.
#[test]
fn a_move_does_not_change_the_length_of_an_element() {
    let mut session = a_project();
    session.execute("art.element.move", &json!({ "element": 5, "to": 500 })).expect("the element is moved");
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(600), "500 + 100 is the end of the track");
}

/// A move of something that is not an element is **refused**.
#[test]
fn a_move_of_something_that_is_not_an_element_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.element.move", &json!({ "element": 3, "to": 10 }))
        .expect_err("a track is not an element");
    assert!(error.to_string().contains("not an element"), "the refusal says what is wrong: {error}");
}

/// A move to a track that is not there is **refused**.
#[test]
fn a_move_to_a_track_that_is_not_there_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.element.move", &json!({ "element": 5, "to": 10, "track": 999 }))
        .expect_err("there is no track 999");
    assert!(error.to_string().contains("there is no track"), "the refusal says what is wrong: {error}");
}
