//! **Taking away** a clip.
//!
//! The clip is gone, the edge that says which item it played goes with
//! it, and the gap it leaves is a gap.

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

/// The **end** of a track, as the timeline says it.
fn end_of(session: &mut Session, track: u64) -> u64 {
    let outcome = session.execute("film.timeline", &json!({})).expect("the timeline is read");
    let tracks = outcome.data["tracks"].as_array().expect("the tracks are a list");
    let entry =
        tracks.iter().find(|entry| entry["track"].as_u64() == Some(track)).expect("the track is there");
    entry["end"].as_u64().expect("the track has an end")
}

/// A clip is **gone** when it is taken away.
#[test]
fn a_clip_is_taken_away() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    session.execute("film.clip.remove", &json!({ "clip": clip })).expect("the clip is taken away");
    assert!(session.document().node(clip).is_none(), "the clip is gone");
}

/// The edge that says which item the clip played goes **with it**.
#[test]
fn the_edge_goes_with_the_clip() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    session.execute("film.clip.remove", &json!({ "clip": clip })).expect("the clip is taken away");
    let edges = session
        .document()
        .relations
        .iter()
        .filter(|relation| relation.kind == "clip" && relation.from == clip)
        .count();
    assert_eq!(edges, 0, "the edge is gone: there is nothing to point at");
}

/// The gap the clip leaves is a **gap**: the clips that were after it
/// stay where they are, and the track is shorter.
#[test]
fn the_gap_is_left() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, track, item, 40, 10);
    session.execute("film.clip.remove", &json!({ "clip": clip })).expect("the clip is taken away");
    let node = session.document().node(other).expect("the other clip is there");
    assert_eq!(node.data["start"].as_u64(), Some(40), "the other clip did not move");
    assert_eq!(end_of(&mut session, track), 50, "and the track ends where the last clip ends");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error =
        session.execute("film.clip.remove", &json!({ "clip": track })).expect_err("the removal is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
    assert!(session.document().node(track).is_some(), "and the node is still there");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("film.clip.remove", &json!({ "clip": 1_000_000 }))
        .expect_err("the removal is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
