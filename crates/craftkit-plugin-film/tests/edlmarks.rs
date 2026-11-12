//! The **markers** and the **dissolves** of an edit decision list.
//!
//! The file is an EDL, and the sequence it names has the markers and
//! the transitions the list says.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-edlmarks-{name}-{}", std::process::id()));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **list** with two cuts, a **dissolve** and two **markers**.
fn a_list() -> String {
    let mut text = String::new();
    text.push_str("TITLE: THE FILM\n");
    text.push_str("FCM: NON-DROP FRAME\n\n");
    text.push_str("001  BEACH    V     C        00:00:00:00 00:00:05:00 00:00:00:00 00:00:05:00\n");
    text.push_str("002  PIER     V     C        00:00:05:00 00:00:10:00 00:00:05:00 00:00:10:00\n");
    text.push_str("* LOC: 00:00:07:00 YELLOW the mark\n");
    text.push_str("* LOC: 00:00:09:00 the plain mark\n");
    text.push_str("003  HILLS    V     D    025 00:00:10:00 00:00:15:00 00:00:10:00 00:00:15:00\n");
    text
}

/// The **tracks** of the sequence that was made.
fn the_tracks(session: &Session, sequence: u64) -> Vec<std::sync::Arc<craftkit_sdk::Node>> {
    let node = session.document().node(sequence).expect("the sequence is there");
    node.children.iter().filter(|child| child.kind == "film.track").cloned().collect()
}

/// The **nodes** of a kind under a node.
fn children_of(session: &Session, node: u64, kind: &str) -> Vec<Json> {
    let node = session.document().node(node).expect("the node is there");
    node.children.iter().filter(|child| child.kind == kind).map(|child| child.data.clone()).collect()
}

/// A `* LOC:` comment is a **marker** of the sequence: where it
/// starts, what it is called and what colour it is.
#[test]
fn a_loc_comment_is_a_marker() {
    let mut session = session();
    let path = a_file("markers", &a_list());
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    assert_eq!(outcome.data["markers"], json!(2), "there are two markers: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&session, sequence, "film.marker");
    assert_eq!(markers.len(), 2, "there are two markers: {markers:?}");
    // Seven seconds at twenty-five frames a second.
    assert_eq!(markers[0]["start"], json!(175), "the marker is where it says: {markers:?}");
    assert_eq!(markers[0]["name"], json!("the mark"), "and it is called what it says: {markers:?}");
    // The tree's own colours: yellow is yellow.
    assert_eq!(markers[0]["color"], json!("Yellow"), "and it is the colour it names: {markers:?}");
    assert_eq!(markers[0]["duration"], json!(0), "and it is a point: {markers:?}");
    // A marker that names no colour is green.
    assert_eq!(markers[1]["color"], json!("Green"), "and one that names none is green: {markers:?}");
    assert_eq!(markers[1]["name"], json!("the plain mark"), "and it is called what it says: {markers:?}");
}

/// A **dissolve** is a transition over the cut before it.
#[test]
fn a_dissolve_is_a_transition() {
    let mut session = session();
    let path = a_file("dissolve", &a_list());
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    let video = tracks.iter().find(|track| track.data["kind"] == json!("video")).expect("there is a video");
    let transitions = children_of(&session, video.id, "film.transition");
    assert_eq!(transitions.len(), 1, "there is one transition: {transitions:?}");
    assert_eq!(transitions[0]["start"], json!(250), "it starts at the cut: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(25), "and it is as long as the line says: {transitions:?}");
    assert_eq!(transitions[0]["align"], json!("start"), "and it starts at the cut: {transitions:?}");
    assert_eq!(transitions[0]["effect"], json!("cross_dissolve"), "and it is a dissolve: {transitions:?}");
    // The **clips** it joins: the one that ends at the cut and the one
    // that begins there.
    let clips: Vec<u64> =
        video.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect();
    assert_eq!(transitions[0]["from"], json!(clips[1]), "it comes from the clip before: {transitions:?}");
    assert_eq!(transitions[0]["to"], json!(clips[2]), "and comes into the one after: {transitions:?}");
}

/// A transition of an **effect this tree does not have** is left out,
/// and the answer says so.
#[test]
fn a_transition_of_an_effect_this_tree_does_not_have_is_left_out() {
    let mut session = session();
    let mut text = a_list();
    text.push_str("* EFFECT NAME: Iris Dissolve\n");
    let path = a_file("effect", &text);
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert!(
        left_out.iter().any(|line| line.as_str().unwrap_or_default().contains("Iris Dissolve")),
        "it is said: {left_out:?}"
    );
}
