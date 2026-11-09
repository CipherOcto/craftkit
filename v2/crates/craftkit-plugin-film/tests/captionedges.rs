//! The **edges** of a caption: trimmed, moved, split and merged.
//!
//! The rules are the tree's own.

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

/// A cue is **trimmed** at its in edge and at its out edge.
#[test]
fn a_cue_is_trimmed() {
    let mut session = session();
    let (track, first, _second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.trim", &json!({ "caption": first, "edge": "in", "delta": 5 }))
        .expect("the cue is trimmed");
    assert_eq!(outcome.data["start"], json!(15), "the in edge moves: {outcome:?}");
    assert_eq!(outcome.data["end"], json!(20), "and the out edge stays: {outcome:?}");
    let outcome = session
        .execute("film.caption.trim", &json!({ "caption": first, "edge": "out", "delta": -3 }))
        .expect("the cue is trimmed");
    assert_eq!(outcome.data["end"], json!(17), "the out edge moves: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues[0]["start"], json!(15), "and the cue holds it: {cues:?}");
}

/// A trim that does not fit the room the cue has is **refused**, and
/// not clamped.
#[test]
fn a_trim_that_does_not_fit_is_refused() {
    let mut session = session();
    let (_track, first, _second) = a_track(&mut session);
    let error = session
        .execute("film.caption.trim", &json!({ "caption": first, "edge": "in", "delta": 100 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("the in edge of the cue may move"), "it says why: {error}");
    let error = session
        .execute("film.caption.trim", &json!({ "caption": first, "edge": "middle", "delta": 1 }))
        .expect_err("the edge is refused");
    assert!(error.to_string().contains("the edge is `in` or `out`"), "it says why: {error}");
}

/// A cue is **moved**, and it does not run into its neighbours.
#[test]
fn a_cue_is_moved() {
    let mut session = session();
    let (track, first, _second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.move", &json!({ "caption": first, "delta": -5 }))
        .expect("the cue is moved");
    assert_eq!(outcome.data["start"], json!(5), "the cue is moved: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues[0]["start"], json!(5), "and it holds it: {cues:?}");
    // It does not run into the cue after it.
    let error = session
        .execute("film.caption.move", &json!({ "caption": first, "delta": 25 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("the cue may move between"), "it says why: {error}");
}

/// A cue is **split** in two, and the two halves are the two cues.
#[test]
fn a_cue_is_split() {
    let mut session = session();
    let (track, first, _second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.split", &json!({ "caption": first, "at": 15 }))
        .expect("the cue is split");
    assert!(outcome.data["split"].as_u64().is_some(), "a cue is made: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 3, "there are three cues: {cues:?}");
    assert_eq!(cues[0]["end"], json!(15), "the first half ends there: {cues:?}");
    assert_eq!(cues[1]["start"], json!(15), "and the second begins: {cues:?}");
    // A split that leaves nothing on one side is refused.
    let error = session
        .execute("film.caption.split", &json!({ "caption": first, "at": 10 }))
        .expect_err("the split is refused");
    assert!(error.to_string().contains("leaves a frame on either side"), "it says why: {error}");
}

/// Two cues that are **next to each other** are merged into one, and
/// their texts are joined with line breaks.
#[test]
fn two_cues_are_merged() {
    let mut session = session();
    let (track, first, second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.merge", &json!({ "captions": [first, second] }))
        .expect("the cues are merged");
    assert_eq!(outcome.data["caption"], json!(first), "the first cue is kept: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["start"], json!(10), "it starts where the first did: {cues:?}");
    assert_eq!(cues[0]["end"], json!(40), "and ends where the last did: {cues:?}");
    assert_eq!(cues[0]["text"], json!("First\nSecond"), "and the texts are joined: {cues:?}");
}

/// Cues that are **not next to each other** may not be merged.
#[test]
fn cues_that_are_not_next_to_each_other_may_not_be_merged() {
    let mut session = session();
    let (track, first, _second) = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 50, "end": 60, "text": "Third" }))
        .expect("the cue is added");
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list").clone();
    let first_id = cues[0]["id"].as_u64().expect("the cue has an id");
    let last_id = cues[2]["id"].as_u64().expect("the cue has an id");
    let error = session
        .execute("film.caption.merge", &json!({ "captions": [first_id, last_id] }))
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("must be next to each other"), "it says why: {error}");
    let _ = first;
}
