//! **Inserting** a clip.
//!
//! The insert opens a gap and pushes the clips along: nothing is cut.

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

/// The clips of a track, in the order they start, with their starts.
fn timeline_of(session: &mut Session, track: u64) -> Vec<u64> {
    let outcome = session.execute("film.timeline", &json!({})).expect("the timeline is read");
    let tracks = outcome.data["tracks"].as_array().expect("the tracks are a list");
    let entry =
        tracks.iter().find(|entry| entry["track"].as_u64() == Some(track)).expect("the track is there");
    entry["clips"]
        .as_array()
        .expect("the clips are a list")
        .iter()
        .map(|clip| clip["start"].as_u64().expect("a clip has a start"))
        .collect()
}

/// A clip is **inserted**, and the clips that are there are **pushed**
/// along by its length: nothing is cut.
#[test]
fn an_insert_pushes_the_clips_along() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, track, item, 50, 10);
    session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": track, "start": 30 }))
        .expect("the clip is inserted");
    assert_eq!(start_of(&session, clip), 30, "the clip is where it was put");
    assert_eq!(start_of(&session, other), 70, "and the clip that was there was pushed along");
    assert_eq!(timeline_of(&mut session, track), vec![30, 70], "the track holds both clips");
}

/// The gap opens at the place, and the clip that is **after** it is
/// pushed by the length of the inserted clip.
#[test]
fn the_gap_opens_at_the_place() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, track, item, 30, 10);
    session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": track, "start": 30 }))
        .expect("the clip is inserted");
    assert_eq!(start_of(&session, other), 50, "the clip that was at the place was pushed by the length");
}

/// A clip that **spans** the place is refused: the tree splits it, and
/// splitting is a command of its own.
#[test]
fn a_clip_that_spans_the_place_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, track, item, 20, 30);
    let error = session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": track, "start": 30 }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("it spans the place"), "the error says why: {error}");
    assert_eq!(start_of(&session, clip), 0, "and the clip is as it was");
    assert_eq!(start_of(&session, other), 20, "and so is the other one");
}

/// A clip is inserted on **another track**, and the gap opens there.
#[test]
fn a_clip_is_inserted_on_another_track() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the other track", "sequence": item, "kind": "audio" }))
        .expect("the other track is added");
    let other_track = outcome.data["id"].as_u64().expect("the track has an id");
    let clip = a_clip(&mut session, track, item, 0, 20);
    let other = a_clip(&mut session, other_track, item, 40, 10);
    session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": other_track, "start": 40 }))
        .expect("the clip is inserted");
    assert_eq!(start_of(&session, clip), 40, "the clip is on the other track");
    assert_eq!(start_of(&session, other), 60, "and the clip that was there was pushed along");
    assert_eq!(timeline_of(&mut session, track), Vec::<u64>::new(), "and the old track is empty");
}

/// The **end** of the track is derived, and it grows by the length of
/// the clip that was inserted.
#[test]
fn the_end_of_the_track_grows() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    a_clip(&mut session, track, item, 40, 10);
    session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": track, "start": 40 }))
        .expect("the clip is inserted");
    let outcome = session.execute("film.timeline", &json!({})).expect("the timeline is read");
    let tracks = outcome.data["tracks"].as_array().expect("the tracks are a list");
    let entry =
        tracks.iter().find(|entry| entry["track"].as_u64() == Some(track)).expect("the track is there");
    assert_eq!(entry["end"].as_u64(), Some(70), "the track ends where the last clip ends");
}

/// A start that is **before the timeline** is refused.
#[test]
fn a_start_before_the_timeline_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let error = session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": track, "start": -1 }))
        .expect_err("the insert is refused");
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
        .execute("film.clip.insert", &json!({ "clip": clip, "track": track, "start": 1.5 }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error = session
        .execute("film.clip.insert", &json!({ "clip": track, "track": track, "start": 10 }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let (item, track) = a_track(&mut session);
    let clip = a_clip(&mut session, track, item, 0, 20);
    let error = session
        .execute("film.clip.insert", &json!({ "clip": clip, "track": item, "start": 10 }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("not a track"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let (_item, track) = a_track(&mut session);
    let error = session
        .execute("film.clip.insert", &json!({ "clip": 1_000_000, "track": track, "start": 10 }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
