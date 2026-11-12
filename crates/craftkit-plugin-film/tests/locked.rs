//! The **locked** track.
//!
//! A locked track cannot be edited: the tree's locked track, and the
//! tree's refusal.

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

/// A **clip** on a track.
fn a_clip(session: &mut Session, track: u64, item: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": 10,
                "duration": 20,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// Lock a track.
fn lock(session: &mut Session, track: u64, locked: bool) {
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": locked }))
        .expect("the track is locked");
}

/// Whether a track is locked.
fn is_locked(session: &Session, track: u64) -> bool {
    let node = session.document().node(track).expect("the track is there");
    node.data["locked"].as_bool().unwrap_or(false)
}

/// A track may be **locked** and **unlocked**.
#[test]
fn a_track_may_be_locked_and_unlocked() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session, 100);
    assert!(!is_locked(&session, track), "the track is not locked to begin with");
    lock(&mut session, track, true);
    assert!(is_locked(&session, track), "and it is locked now");
    lock(&mut session, track, false);
    assert!(!is_locked(&session, track), "and unlocked again");
}

/// A locked track refuses a **trim**.
#[test]
fn a_locked_track_refuses_a_trim() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 5 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// A locked track refuses a **move**.
#[test]
fn a_locked_track_refuses_a_move() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    let error = session
        .execute("film.clip.move", &json!({ "clip": clip, "track": track, "start": 40 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// A locked track refuses a **split**.
#[test]
fn a_locked_track_refuses_a_split() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    let error = session
        .execute("film.clip.split", &json!({ "clip": clip, "at": 15 }))
        .expect_err("the split is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// A locked track refuses a **removal**.
#[test]
fn a_locked_track_refuses_a_removal() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    let error =
        session.execute("film.clip.remove", &json!({ "clip": clip })).expect_err("the removal is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// A locked track refuses a **ripple**.
#[test]
fn a_locked_track_refuses_a_ripple() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    let error =
        session.execute("film.clip.ripple", &json!({ "clip": clip })).expect_err("the ripple is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// A locked track refuses a **lift** and an **extract**.
#[test]
fn a_locked_track_refuses_a_lift_and_an_extract() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let _clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    let error = session
        .execute("film.clip.lift", &json!({ "track": track, "from": 10, "to": 20 }))
        .expect_err("the lift is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
    let error = session
        .execute("film.clip.extract", &json!({ "track": track, "from": 10, "to": 20 }))
        .expect_err("the extract is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// A locked track refuses a **new clip**.
#[test]
fn a_locked_track_refuses_a_new_clip() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    lock(&mut session, track, true);
    let error = session
        .execute("film.clip.add", &json!({ "track": track, "item": item, "start": 40, "duration": 10 }))
        .expect_err("the clip is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// An **unlocked** track may be edited again.
#[test]
fn an_unlocked_track_may_be_edited_again() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item);
    lock(&mut session, track, true);
    lock(&mut session, track, false);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 5 }))
        .expect("the clip is trimmed");
}
