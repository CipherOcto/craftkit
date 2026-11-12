//! **Moving** a clip.
//!
//! A clip moves to a place on a track, and a move that would land on
//! another clip is refused — the tree cuts the clip that is under it
//! and says nothing about the cut.

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
fn a_track(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 1000 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": start, "duration": duration }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// Where a clip **starts**.
fn start_of(session: &Session, clip: u64) -> u64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["start"].as_u64().expect("the clip has a start")
}

/// A clip moves **along** its track.
#[test]
fn a_clip_moves_along_its_track() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 100 }))
        .expect("the clip moves");
    assert_eq!(start_of(&session, clip), 100, "the clip is at the new place");
}

/// A clip moves to **another track**, and it keeps its id and the edge
/// that says which item it plays.
#[test]
fn a_clip_moves_to_another_track() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the other track", "sequence": item, "kind": "audio" }))
        .expect("the other track is added");
    let other = outcome.data["id"].as_u64().expect("the track has an id");
    let clip = a_clip(&mut session, track, item, 0, 20);
    session
        .execute("film.clip.move", &json!({ "clip": clip, "track": other, "start": 40 }))
        .expect("the clip moves");
    let node = session.document().node(clip).expect("the clip is still there");
    assert_eq!(node.id, clip, "the clip kept its id");
    assert_eq!(start_of(&session, clip), 40, "and it is at the new place");
    let relation = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .expect("the edge is still there");
    assert_eq!(relation.to, item, "and it still names the item it plays");
}

/// A move that would land **on another clip** is refused: the tree cuts
/// the clip that is under it and says nothing about the cut.
#[test]
fn a_move_that_would_land_on_another_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    a_clip(&mut session, track, item, 100, 20);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let error = session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 110 }))
        .expect_err("the move is refused");
    assert!(
        error.to_string().contains("the clip on the track runs from frame 100"),
        "the error says why: {error}"
    );
    assert_eq!(start_of(&session, clip), 0, "and the clip is as it was");
}

/// A move to a place that is **free** is allowed, even if it touches
/// the end of the other clip.
#[test]
fn a_move_that_touches_the_end_of_another_clip_is_allowed() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    a_clip(&mut session, track, item, 100, 20);
    let clip = a_clip(&mut session, track, item, 0, 20);
    session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 120 }))
        .expect("the clip moves");
    assert_eq!(start_of(&session, clip), 120, "the clip is at the new place");
}

/// A start that is **before the timeline** is refused.
#[test]
fn a_start_before_the_timeline_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let error = session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": -1 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("the timeline begins at frame 0"), "the error says why: {error}");
    assert_eq!(start_of(&session, clip), 0, "and the clip is as it was");
}

/// A start that is **not a whole number** is refused.
#[test]
fn a_start_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let error = session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 1.5 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error = session
        .execute("film.clip.move", &json!({ "clip": track, "track": track, "start": 10 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let error = session
        .execute("film.clip.move", &json!({ "clip": clip, "track": item, "start": 10 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("not a track"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error = session
        .execute("film.clip.move", &json!({ "clip": 1_000_000, "track": track, "start": 10 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
