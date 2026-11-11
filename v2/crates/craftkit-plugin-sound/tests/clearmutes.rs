//! The **mutes** of every track cleared.
//!
//! The tree's rules: every track of the document is unmuted at once,
//! and a track that is mixed with others is unmuted with them.

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
fn a_track(session: &mut Session, name: &str) -> u64 {
    let outcome = session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// Whether the track is **muted**.
fn muted(session: &Session, track: u64) -> bool {
    let node = session.document().node(track).expect("the track is there");
    node.data["mute"].as_bool().expect("the track has a mute")
}

/// Every track is **unmuted** at once.
#[test]
fn every_track_is_unmuted_at_once() {
    let mut session = session();
    let one = a_track(&mut session, "one");
    let two = a_track(&mut session, "two");
    for track in [one, two] {
        session
            .execute("sound.track.mute", &json!({ "track": track, "muted": true }))
            .expect("the track is muted");
    }
    let outcome = session.execute("sound.track.mute.clear", &json!({})).expect("the mutes are cleared");
    assert_eq!(outcome.data["cleared"], json!(2), "both tracks are unmuted: {outcome:?}");
    assert!(!muted(&session, one), "the first track is not muted");
    assert!(!muted(&session, two), "the second track is not muted");
}

/// A track that is **not muted** is left as it is.
#[test]
fn a_track_that_is_not_muted_is_left_alone() {
    let mut session = session();
    let track = a_track(&mut session, "one");
    let outcome = session.execute("sound.track.mute.clear", &json!({})).expect("the mutes are cleared");
    assert!(!outcome.changed, "nothing was unmuted: {outcome:?}");
    assert!(!muted(&session, track), "the track is not muted");
}

/// The mutes of the tracks are cleared **by name** as well.
#[test]
fn a_track_is_muted_by_name() {
    let mut session = session();
    let track = a_track(&mut session, "one");
    session
        .execute("sound.track.mute", &json!({ "track": track, "muted": true }))
        .expect("the track is muted");
    assert!(muted(&session, track), "the track is muted");
    session
        .execute("sound.track.mute", &json!({ "track": track, "muted": false }))
        .expect("the track is unmuted");
    assert!(!muted(&session, track), "the track is not muted");
}
