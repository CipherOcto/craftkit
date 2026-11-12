//! **Lifting** a range of a track.
//!
//! The clips that are in the range go, and the gap they leave is a
//! gap.

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

/// The clips that are in the range are **taken away**.
#[test]
fn the_clips_in_the_range_are_taken_away() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let first = a_clip(&mut session, track, item, 10, 10);
    let second = a_clip(&mut session, track, item, 30, 10);
    session
        .execute("film.clip.lift", &json!({ "track": track, "from": 10, "to": 40 }))
        .expect("the range is lifted");
    assert!(session.document().node(first).is_none(), "the first clip is gone");
    assert!(session.document().node(second).is_none(), "and so is the second");
}

/// The clips that are **outside** the range stay where they are, and
/// the gap is a gap.
#[test]
fn the_clips_outside_the_range_stay_where_they_are() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let before = a_clip(&mut session, track, item, 0, 5);
    let after = a_clip(&mut session, track, item, 100, 10);
    a_clip(&mut session, track, item, 10, 10);
    session
        .execute("film.clip.lift", &json!({ "track": track, "from": 10, "to": 20 }))
        .expect("the range is lifted");
    assert_eq!(start_of(&session, before), 0, "the clip before did not move");
    assert_eq!(start_of(&session, after), 100, "and the clip after did not move");
}

/// A clip that **spans** an edge of the range is refused: the tree
/// splits it, and the split is a command of its own.
#[test]
fn a_clip_that_spans_an_edge_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 100);
    let error = session
        .execute("film.clip.lift", &json!({ "track": track, "from": 10, "to": 20 }))
        .expect_err("the lift is refused");
    assert!(error.to_string().contains("spans an edge of the range"), "the error says why: {error}");
    assert!(session.document().node(clip).is_some(), "and the clip is still there");
}

/// A range that **ends before it starts** is refused.
#[test]
fn a_range_that_ends_before_it_starts_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 10, 10);
    let error = session
        .execute("film.clip.lift", &json!({ "track": track, "from": 40, "to": 10 }))
        .expect_err("the lift is refused");
    assert!(error.to_string().contains("a range ends after it starts"), "the error says why: {error}");
    assert!(session.document().node(clip).is_some(), "and the clip is still there");
}

/// A range that is **not whole frames** is refused.
#[test]
fn a_range_that_is_not_whole_frames_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error = session
        .execute("film.clip.lift", &json!({ "track": track, "from": 10.5, "to": 20 }))
        .expect_err("the lift is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let (item, _track) = a_track(&mut session);
    let error = session
        .execute("film.clip.lift", &json!({ "track": item, "from": 0, "to": 10 }))
        .expect_err("the lift is refused");
    assert!(error.to_string().contains("not a track"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("film.clip.lift", &json!({ "track": 1_000_000, "from": 0, "to": 10 }))
        .expect_err("the lift is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
