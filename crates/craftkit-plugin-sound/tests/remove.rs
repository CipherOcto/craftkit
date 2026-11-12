//! The **taking away** of a node: a clip, a marker and a track.
//!
//! The tree's editor can delete a clip, a marker and a track, and this
//! tree could do none of it.

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

/// A session with a track and a clip on it, and the ids of both.
fn a_track(session: &mut Session) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// A clip is taken away and is not there any more.
#[test]
fn a_clip_is_taken_away() {
    let mut session = session();
    let (_, clip) = a_track(&mut session);
    session.execute("sound.clip.remove", &json!({ "clip": clip })).expect("the clip is removed");
    assert!(session.document().walk().all(|node| node.kind != "sound.clip"), "the clip is not there");
}

/// A clip whose **content** is locked is **refused**: a lock says the
/// clip is finished.
#[test]
fn a_locked_clip_is_refused() {
    let mut session = session();
    let (_, clip) = a_track(&mut session);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clip, "locked": true }))
        .expect("the clip is locked");
    let error =
        session.execute("sound.clip.remove", &json!({ "clip": clip })).expect_err("the clip is refused");
    assert!(error.to_string().contains("locked"), "the error says why: {error}");
    assert!(session.document().walk().any(|node| node.kind == "sound.clip"), "and the clip is still there");
}

/// A marker is taken away and is not there any more.
#[test]
fn a_marker_is_taken_away() {
    let mut session = session();
    session
        .execute("sound.marker.add", &json!({ "name": "the chorus", "start": 96000 }))
        .expect("the marker is added");
    let marker =
        session.document().walk().find(|node| node.kind == "sound.marker").expect("the marker is there").id;
    session.execute("sound.marker.remove", &json!({ "marker": marker })).expect("the marker is removed");
    assert!(session.document().walk().all(|node| node.kind != "sound.marker"), "the marker is not there");
}

/// A track is taken away with its **clips**, and the **route** that
/// named it goes with it: a route to a track that is not there is a
/// route to nothing.
#[test]
fn a_track_is_taken_away_with_its_clips() {
    let mut session = session();
    let (track, _) = a_track(&mut session);
    session.execute("sound.bus.add", &json!({ "name": "the bus" })).expect("the bus is added");
    let bus = session.document().walk().find(|node| node.kind == "sound.bus").expect("the bus is there").id;
    session.execute("sound.route.add", &json!({ "track": track, "bus": bus })).expect("the route is made");
    session.execute("sound.track.remove", &json!({ "track": track })).expect("the track is removed");
    assert!(session.document().walk().all(|node| node.kind != "sound.track"), "the track is not there");
    assert!(
        session.document().walk().all(|node| node.kind != "sound.clip"),
        "and its clip is not there either"
    );
    assert!(
        session.document().relations_to(bus, Some("route")).is_empty(),
        "and the route that named it is not there"
    );
}

/// A **master** is not taken away: the master holds the fader of the
/// whole mix.
#[test]
fn a_master_is_refused() {
    let mut session = session();
    session
        .execute("sound.track.add", &json!({ "name": "the master", "track_kind": "master" }))
        .expect("the master is added");
    let master = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.track" && node.name == "the master")
        .expect("the master is there")
        .id;
    let error = session
        .execute("sound.track.remove", &json!({ "track": master }))
        .expect_err("the master is refused");
    assert!(error.to_string().contains("master"), "the error says why: {error}");
}

/// A node that is **not** a clip is refused: the command says what it
/// takes away.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (track, _) = a_track(&mut session);
    let error =
        session.execute("sound.clip.remove", &json!({ "clip": track })).expect_err("the node is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
}
