//! **Rippling** a clip away.
//!
//! The clip is gone and the gap closes: the clips that were after it
//! move up by the length of the clip that went.

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

/// A clip is taken away and the **gap closes**: the clips that were
/// after it move up by its length.
#[test]
fn the_gap_closes() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, track, item, 40, 10);
    session.execute("film.clip.ripple", &json!({ "clip": clip })).expect("the clip is rippled away");
    assert!(session.document().node(clip).is_none(), "the clip is gone");
    assert_eq!(start_of(&session, other), 20, "and the clip that was after it moved up");
}

/// The clip **before** the one that goes does not move.
#[test]
fn the_clip_before_does_not_move() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let before = a_clip(&mut session, track, item, 0, 10);
    let clip = a_clip(&mut session, track, item, 20, 10);
    session.execute("film.clip.ripple", &json!({ "clip": clip })).expect("the clip is rippled away");
    assert_eq!(start_of(&session, before), 0, "the clip before stayed where it was");
}

/// A ripple whose gap **cannot close** is refused: another clip covers
/// the time the clip took.
#[test]
fn a_gap_that_cannot_close_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, track, item, 10, 10);
    let error =
        session.execute("film.clip.ripple", &json!({ "clip": clip })).expect_err("the ripple is refused");
    assert!(error.to_string().contains("no gap can close"), "the error says why: {error}");
    assert!(session.document().node(clip).is_some(), "and the clip is still there");
    assert_eq!(start_of(&session, other), 10, "and so is the other one");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error =
        session.execute("film.clip.ripple", &json!({ "clip": track })).expect_err("the ripple is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("film.clip.ripple", &json!({ "clip": 1_000_000 }))
        .expect_err("the ripple is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
