//! The **text** and the **times** of a caption.
//!
//! A cue may be edited: its text, its speaker and the two times it is
//! on the screen. The rules are the tree's own.

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

/// The **text** of a cue is changed, and so is its **speaker**.
#[test]
fn the_text_of_a_cue_is_changed() {
    let mut session = session();
    let (track, first, _second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.set", &json!({ "caption": first, "text": "Changed", "speaker": "Ann" }))
        .expect("the cue is set");
    assert_eq!(outcome.data["text"], json!("Changed"), "the text is set: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues[0]["text"], json!("Changed"), "and the cue holds it: {cues:?}");
    assert_eq!(cues[0]["speaker"], json!("Ann"), "and the speaker: {cues:?}");
    // An **empty** speaker takes it away.
    session
        .execute("film.caption.set", &json!({ "caption": first, "speaker": "  " }))
        .expect("the cue is set");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues[0]["speaker"], json!(null), "the speaker is gone: {cues:?}");
}

/// A command that changes **neither** the text nor the speaker is
/// refused.
#[test]
fn a_command_that_would_do_nothing_is_refused() {
    let mut session = session();
    let (_track, first, _second) = a_track(&mut session);
    let error = session
        .execute("film.caption.set", &json!({ "caption": first }))
        .expect_err("the command is refused");
    assert!(error.to_string().contains("a cue has a text or a speaker"), "it says why: {error}");
}

/// The **times** of a cue are moved, and a cue that would overlap
/// another one is refused.
#[test]
fn the_times_of_a_cue_are_moved() {
    let mut session = session();
    let (track, first, second) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.times", &json!({ "caption": first, "start": 12, "end": 22 }))
        .expect("the times are set");
    assert_eq!(outcome.data["start"], json!(12), "the start is set: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues[0]["start"], json!(12), "and the cue holds it: {cues:?}");
    // The cue that is moved over another one is refused.
    let error = session
        .execute("film.caption.times", &json!({ "caption": second, "start": 15, "end": 25 }))
        .expect_err("the times are refused");
    assert!(error.to_string().contains("overlaps a cue of the track"), "it says why: {error}");
    // A cue that ends exactly when another one starts is not an
    // overlap.
    session
        .execute("film.caption.times", &json!({ "caption": first, "start": 0, "end": 12 }))
        .expect("the times are set");
    session
        .execute("film.caption.times", &json!({ "caption": second, "start": 12, "end": 22 }))
        .expect("the times are set");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues[1]["start"], json!(12), "the cue is moved: {cues:?}");
}

/// A time that is not a whole number of frames, and a cue that does
/// not last, are refused.
#[test]
fn a_time_that_is_not_one_is_refused() {
    let mut session = session();
    let (_track, first, _second) = a_track(&mut session);
    let error = session
        .execute("film.caption.times", &json!({ "caption": first, "start": 12.5 }))
        .expect_err("the time is refused");
    assert!(error.to_string().contains("a whole number of frames"), "it says why: {error}");
    let error = session
        .execute("film.caption.times", &json!({ "caption": first, "start": 30, "end": 20 }))
        .expect_err("the times are refused");
    assert!(error.to_string().contains("a cue ends after it starts"), "it says why: {error}");
}
