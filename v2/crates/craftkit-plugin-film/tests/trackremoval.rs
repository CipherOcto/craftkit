//! **Taking away** a track.

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
fn a_timeline(session: &mut Session, media: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": media }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": 0, "duration": 10, "source_in": 0 }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// A track is **taken away**.
#[test]
fn a_track_is_taken_away() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session, 100);
    let outcome =
        session.execute("film.track.remove", &json!({ "track": track })).expect("the track is taken away");
    assert_eq!(outcome.data["name"].as_str(), Some("the track"), "the name is said");
    assert!(session.document().node(track).is_none(), "and the track is not there");
}

/// The clips on the track go **with it**.
#[test]
fn the_clips_on_the_track_go_with_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    let outcome =
        session.execute("film.track.remove", &json!({ "track": track })).expect("the track is taken away");
    let clips = outcome.data["clips"].as_array().expect("the clips are said");
    assert_eq!(clips.len(), 1, "the clip is said");
    assert!(session.document().node(clip).is_none(), "and the clip is not there");
}

/// A **locked** track may not be taken away.
#[test]
fn a_locked_track_may_not_be_taken_away() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session, 100);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.track.remove", &json!({ "track": track }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("is locked"), "it says why: {message}");
    assert!(session.document().node(track).is_some(), "and the track is still there");
}

/// A track that is **unlocked** may be taken away.
#[test]
fn an_unlocked_track_may_be_taken_away() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session, 100);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": false }))
        .expect("the track is unlocked");
    session.execute("film.track.remove", &json!({ "track": track })).expect("the track is taken away");
    assert!(session.document().node(track).is_none(), "and the track is not there");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.track.remove", &json!({ "track": item }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("not a track"), "it says why: {message}");
    let error = session
        .execute("film.track.remove", &json!({ "track": 9999 }))
        .expect_err("the taking away is refused");
    let message = error.to_string();
    assert!(message.contains("there is no track"), "and there is nothing: {message}");
}
