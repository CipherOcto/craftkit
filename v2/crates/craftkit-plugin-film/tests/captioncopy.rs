//! The **copy** of a cue, and the **end** of a track.
//!
//! A cue may be duplicated and a caption track may be deleted. The
//! rules are the tree's own.

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

/// A **caption track** with **two** cues on it, and their ids.
fn a_track(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let mut cues = Vec::new();
    for (start, end, text) in [(10, 20, "First"), (30, 40, "Second")] {
        let outcome = session
            .execute("film.caption.add", &json!({ "track": track, "start": start, "end": end, "text": text }))
            .expect("the cue is added");
        cues.push(outcome.data["id"].as_u64().expect("the cue has an id"));
    }
    (track, cues[0], cues[1])
}

/// The **cues** of a track, as the answer holds them.
fn cues_of(session: &mut Session, track: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// A cue is **duplicated** and the copy stands later on the track.
#[test]
fn a_cue_is_duplicated() {
    let mut session = session();
    let (track, first, _second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.duplicate", &json!({ "caption": first, "delta": 50 }))
        .expect("the cue is duplicated");
    let copy = outcome.data["copy"].as_u64().expect("the copy has an id");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 3, "there are three cues: {cues:?}");
    assert_eq!(cues[2]["start"], json!(60), "the copy stands later: {cues:?}");
    assert_eq!(cues[2]["end"], json!(70), "and it lasts as long: {cues:?}");
    assert_eq!(cues[2]["text"], json!("First"), "and it says the same: {cues:?}");
    assert_eq!(cues[2]["id"], json!(copy), "and it is the copy: {cues:?}");
}

/// A copy that would stand **before the start** of the sequence is
/// refused, and not clamped.
#[test]
fn a_copy_before_the_start_is_refused() {
    let mut session = session();
    let (_track, first, _second) = a_track(&mut session);
    let error = session
        .execute("film.caption.duplicate", &json!({ "caption": first, "delta": -20 }))
        .expect_err("the copy is refused");
    assert!(error.to_string().contains("may not stand before the start"), "it says why: {error}");
}

/// A copy that would run **into** another cue is refused: the cues of
/// a track never overlap.
#[test]
fn a_copy_that_overlaps_a_cue_is_refused() {
    let mut session = session();
    let (_track, first, _second) = a_track(&mut session);
    let error = session
        .execute("film.caption.duplicate", &json!({ "caption": first, "delta": 15 }))
        .expect_err("the copy is refused");
    assert!(error.to_string().contains("overlaps a cue of the track"), "it says why: {error}");
}

/// A caption **track** is deleted, and its cues go with it.
#[test]
fn a_caption_track_is_deleted() {
    let mut session = session();
    let (track, _first, _second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.track.delete", &json!({ "track": track }))
        .expect("the track is deleted");
    assert_eq!(outcome.data["cues"], json!(2), "its cues went with it: {outcome:?}");
    let error = session
        .execute("film.caption.track.delete", &json!({ "track": track }))
        .expect_err("the track is not there");
    assert!(error.to_string().contains("there is no caption track"), "it says why: {error}");
}
