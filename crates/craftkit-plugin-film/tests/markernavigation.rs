//! The **playhead** and the **markers** of a sequence.
//!
//! The tree's `markers.goNext` and `markers.goPrev`: the playhead
//! goes to the marker that is next, and to the one before.

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

/// A **bin**, an **item** in it, and a **track** in the item.
fn a_sequence(session: &mut Session) -> (u64, u64) {
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

/// A **marker** on the timeline of a sequence.
fn a_marker(session: &mut Session, sequence: u64, start: u64, name: &str) -> u64 {
    let outcome = session
        .execute("film.marker.add", &json!({ "sequence": sequence, "start": start, "name": name }))
        .expect("the marker is added");
    outcome.data["id"].as_u64().expect("the marker has an id")
}

/// The **playhead** of a sequence.
fn playhead_of(session: &Session, sequence: u64) -> u64 {
    session.document().node(sequence).expect("the sequence is there").data["playhead"].as_u64().unwrap_or(0)
}

/// The playhead goes to the **next** marker.
#[test]
fn the_playhead_goes_to_the_next_marker() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    let first = a_marker(&mut session, item, 20, "the first");
    let second = a_marker(&mut session, item, 50, "the second");
    session
        .execute("film.playhead.set", &json!({ "sequence": item, "frame": 0 }))
        .expect("the playhead is set");
    let outcome =
        session.execute("film.marker.next", &json!({ "sequence": item })).expect("the playhead moves");
    assert_eq!(outcome.data["marker"], json!(first), "it goes to the first marker");
    assert_eq!(outcome.data["frame"], json!(20), "at the frame of that marker");
    assert_eq!(playhead_of(&session, item), 20, "and the playhead is there");
    session.execute("film.marker.next", &json!({ "sequence": item })).expect("the playhead moves again");
    assert_eq!(playhead_of(&session, item), 50, "and it goes to the second marker");
    let _ = second;
}

/// The playhead goes to the **marker before** it.
#[test]
fn the_playhead_goes_to_the_marker_before_it() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    a_marker(&mut session, item, 20, "the first");
    let second = a_marker(&mut session, item, 50, "the second");
    session
        .execute("film.playhead.set", &json!({ "sequence": item, "frame": 80 }))
        .expect("the playhead is set");
    let outcome =
        session.execute("film.marker.previous", &json!({ "sequence": item })).expect("the playhead moves");
    assert_eq!(outcome.data["marker"], json!(second), "it goes to the last marker before it");
    assert_eq!(playhead_of(&session, item), 50, "and the playhead is there");
}

/// A sequence with **no marker** after the playhead is refused.
#[test]
fn a_sequence_with_no_marker_after_the_playhead_is_refused() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    a_marker(&mut session, item, 20, "the first");
    session
        .execute("film.playhead.set", &json!({ "sequence": item, "frame": 30 }))
        .expect("the playhead is set");
    let error =
        session.execute("film.marker.next", &json!({ "sequence": item })).expect_err("the move is refused");
    assert!(error.to_string().contains("no marker after"), "the refusal says why: {error}");
    assert_eq!(playhead_of(&session, item), 30, "and the playhead has not moved");
}

/// A sequence with **no marker** before the playhead is refused.
#[test]
fn a_sequence_with_no_marker_before_the_playhead_is_refused() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    a_marker(&mut session, item, 40, "the first");
    session
        .execute("film.playhead.set", &json!({ "sequence": item, "frame": 10 }))
        .expect("the playhead is set");
    let error = session
        .execute("film.marker.previous", &json!({ "sequence": item }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("no marker before"), "the refusal says why: {error}");
    assert_eq!(playhead_of(&session, item), 10, "and the playhead has not moved");
}

/// The markers are the **sequence's own**, and the playhead of one
/// sequence is not the playhead of another.
#[test]
fn the_markers_are_the_sequences_own() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    let (other, _other_track) = a_sequence(&mut session);
    a_marker(&mut session, item, 20, "the first");
    a_marker(&mut session, other, 60, "the other one");
    session
        .execute("film.playhead.set", &json!({ "sequence": item, "frame": 0 }))
        .expect("the playhead is set");
    session.execute("film.marker.next", &json!({ "sequence": item })).expect("the playhead moves");
    assert_eq!(playhead_of(&session, item), 20, "the playhead of the sequence is at its own marker");
    assert_eq!(playhead_of(&session, other), 0, "and the other sequence keeps its own playhead");
}
