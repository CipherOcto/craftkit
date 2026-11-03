//! The **recording** of a track.
//!
//! A track may be **armed** for recording, and it may **monitor** its
//! input. The tree's transport asks the host to record, and here there
//! is no host: what is here is the arming and the monitoring — the
//! facts of a track that is ready to record.

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

/// A track may be **armed** for recording.
#[test]
fn a_track_may_be_armed() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.record", &json!({ "track": track, "armed": true }))
        .expect("the track is armed");
    assert_eq!(payload_of(&session, track)["record_arm"].as_bool(), Some(true), "the track is armed");
}

/// The arm may be taken **away** again.
#[test]
fn the_arm_may_be_taken_away() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.record", &json!({ "track": track, "armed": true }))
        .expect("the track is armed");
    session
        .execute("sound.track.record", &json!({ "track": track, "armed": false }))
        .expect("the track is not armed");
    assert_eq!(payload_of(&session, track)["record_arm"].as_bool(), Some(false), "and it is not armed");
}

/// A track may **hear its input**.
#[test]
fn a_track_may_hear_its_input() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.monitor", &json!({ "track": track, "monitoring": true }))
        .expect("the track monitors its input");
    assert_eq!(payload_of(&session, track)["input_monitor"].as_bool(), Some(true), "the track is listening");
}

/// A track that is **not armed** is not armed.
#[test]
fn a_track_that_is_not_armed_is_not_armed() {
    let mut session = session();
    let track = a_track(&mut session);
    assert_eq!(payload_of(&session, track)["record_arm"].as_bool(), Some(false), "a new track is not armed");
}

/// The arm is the track's **own**: the tree's group shares the mute
/// and the solo with the tracks that mix together, and the arm is not
/// one of them.
///
/// So a track that is armed in a group that mixes is armed **alone**:
/// the other member of the group is not armed with it.
#[test]
fn the_arm_is_the_tracks_own() {
    let mut session = session();
    let track = a_track(&mut session);
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the other track is added");
    let other = session.document().node_by_name("bass").expect("the other track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "the pair", "tracks": [track, other] }))
        .expect("the tracks are in a group");
    session
        .execute("sound.track.record", &json!({ "track": track, "armed": true }))
        .expect("the track is armed");
    assert_eq!(payload_of(&session, track)["record_arm"].as_bool(), Some(true), "the track is armed");
    assert_eq!(payload_of(&session, other)["record_arm"].as_bool(), Some(false), "and the other one is not");
}

/// The **recording modes** are settings of the session.
#[test]
fn the_recording_modes_are_settings_of_the_session() {
    let mut session = session();
    let outcome = session
        .execute("sound.session.set", &json!({ "destructive_record": true, "track_punch": true }))
        .expect("the modes are set");
    assert_eq!(outcome.data["destructive_record"].as_bool(), Some(true), "destructive is on");
    assert_eq!(outcome.data["track_punch"].as_bool(), Some(true), "and the punch is on");
}

/// A track that is **not there** is refused.
#[test]
fn a_track_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session.execute("sound.track.record", &json!({ "track": 999, "armed": true }));
    assert!(refused.is_err(), "there is no such track");
}
