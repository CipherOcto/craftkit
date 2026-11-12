//! The **markers** of a sequence.
//!
//! A marker is a named place on the timeline of a sequence.

use craftkit_sdk::serde_json::json;
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

/// A **bin**, an **item** in it, a **track** in the item.
fn a_timeline(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **marker** on a sequence, and its id.
fn a_marker(session: &mut Session, sequence: u64, start: u64) -> u64 {
    let outcome = session
        .execute("film.marker.add", &json!({ "sequence": sequence, "start": start }))
        .expect("the marker is added");
    outcome.data["id"].as_u64().expect("the marker has an id")
}

/// The facts of a marker.
fn of(session: &Session, marker: u64) -> (u64, u64, String, String, Option<String>, String) {
    let node = session.document().node(marker).expect("the marker is there");
    let start = node.data["start"].as_u64().expect("the marker has a start");
    let duration = node.data["duration"].as_u64().expect("the marker has a duration");
    let name = node.data["name"].as_str().expect("the marker has a name").to_string();
    let comment = node.data["comment"].as_str().expect("the marker has a comment").to_string();
    let color = node.data.get("color").and_then(|color| color.as_str()).map(|color| color.to_string());
    let kind = node.data["kind"].as_str().expect("the marker has a kind").to_string();
    (start, duration, name, comment, color, kind)
}

/// A marker is **added** to a sequence, and it is a thing of its own.
#[test]
fn a_marker_is_added() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let marker = a_marker(&mut session, item, 30);
    let node = session.document().node(marker).expect("the marker is there");
    assert_eq!(node.kind, "film.marker", "and it is a marker");
    let (start, duration, name, comment, color, kind) = of(&session, marker);
    assert_eq!(start, 30, "the marker is where it was put");
    assert_eq!(duration, 0, "and it is a point");
    assert_eq!(name, "", "it has no name of its own");
    assert_eq!(comment, "", "and nothing to say");
    assert_eq!(color, None, "and no colour");
    assert_eq!(kind, "Comment", "and it is a comment");
}

/// The marker is **in** the sequence: the tree is containment and
/// relations.
#[test]
fn the_marker_is_in_the_sequence() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let marker = a_marker(&mut session, item, 30);
    let node = session.document().node(item).expect("the sequence is there");
    assert!(node.children.iter().any(|child| child.id == marker), "the sequence holds the marker");
}

/// A marker may be given a **name**, a **comment**, a **colour**, a
/// **kind** and a **length**.
#[test]
fn a_marker_may_carry_its_facts() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let outcome = session
        .execute(
            "film.marker.add",
            &json!({
                "sequence": item,
                "start": 10,
                "duration": 20,
                "name": "the opening",
                "comment": "where it begins",
                "color": "Violet",
                "kind": "Chapter",
            }),
        )
        .expect("the marker is added");
    let marker = outcome.data["id"].as_u64().expect("the marker has an id");
    let (start, duration, name, comment, color, kind) = of(&session, marker);
    assert_eq!(start, 10, "the marker is where it was put");
    assert_eq!(duration, 20, "and it is as long as it was told");
    assert_eq!(name, "the opening", "and it is called what it was called");
    assert_eq!(comment, "where it begins", "and it says what it was told");
    assert_eq!(color, Some("Violet".to_string()), "and it is the colour it was told");
    assert_eq!(kind, "Chapter", "and it is the kind it was told");
}

/// A colour that is **not one of the tree's own** is refused, and the
/// refusal says what the colours are.
#[test]
fn a_colour_that_is_not_a_label_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let error = session
        .execute("film.marker.add", &json!({ "sequence": item, "start": 10, "color": "Mauve" }))
        .expect_err("the marker is refused");
    assert!(error.to_string().contains("there is no label `Mauve`"), "the error says so: {error}");
    assert!(error.to_string().contains("Violet"), "and it says what the labels are");
}

/// A kind that is **not one of the tree's own** is refused.
#[test]
fn a_kind_that_is_not_a_kind_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let error = session
        .execute("film.marker.add", &json!({ "sequence": item, "start": 10, "kind": "Bookmark" }))
        .expect_err("the marker is refused");
    assert!(error.to_string().contains("there is no kind `Bookmark`"), "the error says so: {error}");
    assert!(error.to_string().contains("Chapter"), "and it says what the kinds are");
}

/// A marker that would start **before the timeline** is refused.
#[test]
fn a_marker_before_the_timeline_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let error = session
        .execute("film.marker.add", &json!({ "sequence": item, "start": -5 }))
        .expect_err("the marker is refused");
    assert!(error.to_string().contains("whole number"), "the error says so: {error}");
}

/// A marker that would start **half a frame** in is refused.
#[test]
fn a_marker_of_half_a_frame_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let error = session
        .execute("film.marker.add", &json!({ "sequence": item, "start": 1.5 }))
        .expect_err("the marker is refused");
    assert!(error.to_string().contains("whole number"), "the error says so: {error}");
}

/// A node that is **not a sequence** is refused: a marker belongs to a
/// sequence and not to a track.
#[test]
fn a_node_that_is_not_a_sequence_is_refused() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session);
    let error = session
        .execute("film.marker.add", &json!({ "sequence": track, "start": 10 }))
        .expect_err("the marker is refused");
    assert!(error.to_string().contains("not a sequence"), "the error says so: {error}");
}

/// A marker may be **changed**, and only what is named is changed.
#[test]
fn a_marker_may_be_changed() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let marker = a_marker(&mut session, item, 10);
    session
        .execute("film.marker.set", &json!({ "marker": marker, "name": "the opening" }))
        .expect("the marker is changed");
    let (_, _, name, comment, color, kind) = of(&session, marker);
    assert_eq!(name, "the opening", "the name changed");
    assert_eq!(comment, "", "the comment did not");
    assert_eq!(color, None, "the colour did not");
    assert_eq!(kind, "Comment", "the kind did not");
    session
        .execute(
            "film.marker.set",
            &json!({ "marker": marker, "color": "Green", "kind": "Segmentation", "duration": 40 }),
        )
        .expect("the marker is changed");
    let (_, duration, _, _, color, kind) = of(&session, marker);
    assert_eq!(duration, 40, "the length changed");
    assert_eq!(color, Some("Green".to_string()), "the colour changed");
    assert_eq!(kind, "Segmentation", "the kind changed");
}

/// A change to a colour that is **not a label** is refused, and the
/// marker is as it was.
#[test]
fn a_change_to_an_unknown_colour_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let marker = a_marker(&mut session, item, 10);
    session
        .execute("film.marker.set", &json!({ "marker": marker, "color": "Green" }))
        .expect("the marker is changed");
    let error = session
        .execute("film.marker.set", &json!({ "marker": marker, "color": "Mauve" }))
        .expect_err("the change is refused");
    assert!(error.to_string().contains("there is no label `Mauve`"), "the error says so: {error}");
    assert_eq!(of(&session, marker).4, Some("Green".to_string()), "and the marker is unchanged");
}

/// A node that is **not a marker** may not be changed.
#[test]
fn a_node_that_is_not_a_marker_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let error = session
        .execute("film.marker.set", &json!({ "marker": item, "name": "the opening" }))
        .expect_err("the change is refused");
    assert!(error.to_string().contains("not a marker"), "the error says so: {error}");
}

/// A marker may be **taken away**.
#[test]
fn a_marker_may_be_taken_away() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let marker = a_marker(&mut session, item, 10);
    session.execute("film.marker.remove", &json!({ "marker": marker })).expect("the marker is taken away");
    assert!(session.document().node(marker).is_none(), "and it is gone");
}

/// Every marker of a sequence may be **taken away at once**, and the
/// markers of another sequence are not touched.
#[test]
fn every_marker_of_a_sequence_may_be_taken_away() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let other = session
        .execute("film.item.add", &json!({ "name": "the other", "bin": 1, "item": 2, "duration": 100 }))
        .expect("the other sequence is added")
        .data["id"]
        .as_u64()
        .expect("the other sequence has an id");
    let first = a_marker(&mut session, item, 10);
    let second = a_marker(&mut session, item, 20);
    let elsewhere = a_marker(&mut session, other, 10);
    session.execute("film.marker.clear", &json!({ "sequence": item })).expect("the markers are taken away");
    assert!(session.document().node(first).is_none(), "the first marker is gone");
    assert!(session.document().node(second).is_none(), "the second marker is gone");
    assert!(session.document().node(elsewhere).is_some(), "and the other sequence's is not");
}

/// The marker is **saved**: it is in the document after it is done.
#[test]
fn the_marker_is_saved() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session);
    let marker = a_marker(&mut session, item, 10);
    session
        .execute("film.marker.set", &json!({ "marker": marker, "name": "the opening", "color": "Rose" }))
        .expect("the marker is changed");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(marker).expect("the marker is in the document");
    assert_eq!(node.kind, "film.marker", "and it is a marker");
    assert_eq!(node.data["name"].as_str(), Some("the opening"), "with its name");
    assert_eq!(node.data["color"].as_str(), Some("Rose"), "and its colour");
    assert_eq!(node.data["start"].as_u64(), Some(10), "and its place");
}
