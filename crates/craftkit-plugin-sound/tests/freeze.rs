//! The **frozen** track.
//!
//! The tree's freeze sets a flag on a track. The tree's engine reads
//! that flag nowhere but the track's header, so the tree's freeze
//! freezes nothing: it is a fact about the track and nothing more.
//! This is that fact, kept as one.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// A **track**, and its id.
fn a_track(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    session.document().node_by_name("drums").expect("the track is there").id
}

/// The **payload** of a track, as the document holds it.
fn payload_of(session: &Session, track: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(track).expect("the track is there");
    node.data.clone()
}

/// A track may be **frozen**.
#[test]
fn a_track_may_be_frozen() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.freeze", &json!({ "track": track, "frozen": true }))
        .expect("the track is frozen");
    assert_eq!(payload_of(&session, track)["frozen"].as_bool(), Some(true), "the track is frozen");
}

/// The freeze may be **taken away**.
#[test]
fn the_freeze_may_be_taken_away() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.freeze", &json!({ "track": track, "frozen": true }))
        .expect("the track is frozen");
    session
        .execute("sound.track.freeze", &json!({ "track": track, "frozen": false }))
        .expect("the track is not frozen");
    assert_eq!(payload_of(&session, track)["frozen"].as_bool(), Some(false), "and it is not frozen");
}

/// A track that was never frozen is **not frozen**.
#[test]
fn a_track_that_was_never_frozen_is_not_frozen() {
    let mut session = session();
    let track = a_track(&mut session);
    assert_eq!(payload_of(&session, track)["frozen"].as_bool(), Some(false), "a new track is not frozen");
}

/// The freeze is the track's **own**: freezing one track of a group
/// that mixes together freezes **only** that one.
#[test]
fn the_freeze_is_the_tracks_own() {
    let mut session = session();
    let track = a_track(&mut session);
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the other track is added");
    let other = session.document().node_by_name("bass").expect("the other track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "the pair", "tracks": [track, other] }))
        .expect("the tracks are in a group");
    session
        .execute("sound.track.freeze", &json!({ "track": track, "frozen": true }))
        .expect("the track is frozen");
    assert_eq!(payload_of(&session, track)["frozen"].as_bool(), Some(true), "the track is frozen");
    assert_eq!(payload_of(&session, other)["frozen"].as_bool(), Some(false), "and the other one is not");
}

/// The freeze is **saved**: a document that is written and read back
/// holds the frozen track.
#[test]
fn the_freeze_is_saved() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.freeze", &json!({ "track": track, "frozen": true }))
        .expect("the track is frozen");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(track).expect("the track is there");
    assert_eq!(node.data["frozen"].as_bool(), Some(true), "the frozen track is still frozen");
}

/// A node that is **not a track** cannot be frozen.
#[test]
fn a_node_that_is_not_a_track_cannot_be_frozen() {
    let mut session = session();
    let track = a_track(&mut session);
    let error = session
        .execute("sound.track.freeze", &json!({ "track": track + 1000, "frozen": true }))
        .expect_err("there is no such track");
    assert!(error.to_string().contains("no track"), "the error says there is no track: {error}");
}
