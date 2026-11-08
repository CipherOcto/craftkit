//! The **caption track** of a sequence.
//!
//! The cues of a track never overlap, and a cue ends after it starts.

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

/// A **sequence** with a **caption track** on it, and their ids.
fn a_track(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (sequence, track)
}

/// The **cues** of a track, as the answer holds them.
fn cues_of(session: &mut Session, track: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// A caption track is added to a **sequence**, and its format is the
/// tree's.
#[test]
fn a_caption_track_is_added_to_a_sequence() {
    let mut session = session();
    let (sequence, track) = a_track(&mut session);
    let node = session.document().node(track).expect("the track is there");
    assert_eq!(node.kind, "film.caption.track", "the track is a caption track");
    let parent = session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == track))
        .expect("the track is in something");
    assert_eq!(parent.id, sequence, "and the sequence holds it");
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    assert_eq!(outcome.data["format"], json!("subtitle"), "the format is the tree's: {outcome:?}");
}

/// A format that is **not one of the tree's** is refused by name.
#[test]
fn a_format_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (sequence, _track) = a_track(&mut session);
    let outcome = session.execute(
        "film.caption.track.add",
        &json!({ "sequence": sequence, "name": "the others", "format": "srt" }),
    );
    assert!(outcome.is_err(), "the format is refused: {outcome:?}");
    let outcome = session
        .execute(
            "film.caption.track.add",
            &json!({ "sequence": sequence, "name": "the others", "format": "cea608" }),
        )
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    assert_eq!(outcome.data["format"], json!("cea608"), "the format is the one asked for: {outcome:?}");
}

/// A **cue** is added, and the cues of a track are in the order they
/// start in.
#[test]
fn a_cue_is_added() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute(
            "film.caption.add",
            &json!({ "track": track, "start": 100, "end": 250, "text": "First line" }),
        )
        .expect("the cue is added");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 10, "end": 50, "text": "Earlier" }))
        .expect("the cue is added");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 2, "there are two cues: {cues:?}");
    assert_eq!(cues[0]["text"], json!("Earlier"), "the earlier cue is first: {cues:?}");
    assert_eq!(cues[0]["start"], json!(10), "and its start: {cues:?}");
    assert_eq!(cues[1]["text"], json!("First line"), "the other is second: {cues:?}");
}

/// The cues of a track never **overlap**: a cue that overlaps another
/// is refused, and a cue that ends exactly when the next one starts is
/// not an overlap.
#[test]
fn the_cues_of_a_track_never_overlap() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 0, "end": 100, "text": "First" }))
        .expect("the cue is added");
    let outcome = session
        .execute("film.caption.add", &json!({ "track": track, "start": 50, "end": 150, "text": "Over" }))
        .expect_err("the overlapping cue is refused");
    assert!(outcome.to_string().contains("overlaps"), "the refusal says why: {outcome}");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 100, "end": 200, "text": "Next" }))
        .expect("a cue that starts when the other ends is not an overlap");
}

/// A cue that **ends before it starts** is refused, and so is a time
/// that is not a whole number of frames.
#[test]
fn a_cue_that_ends_before_it_starts_is_refused() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.add", &json!({ "track": track, "start": 100, "end": 50, "text": "x" }))
        .expect_err("the cue is refused");
    assert!(outcome.to_string().contains("ends after it starts"), "the refusal says why: {outcome}");
    let outcome = session
        .execute("film.caption.add", &json!({ "track": track, "start": 10.5, "end": 50, "text": "x" }))
        .expect_err("the time is refused");
    assert!(outcome.to_string().contains("whole number of frames"), "the refusal says why: {outcome}");
}

/// A cue is **taken away** from its track.
#[test]
fn a_cue_is_taken_away() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    let outcome = session
        .execute("film.caption.add", &json!({ "track": track, "start": 0, "end": 100, "text": "First" }))
        .expect("the cue is added");
    let caption = outcome.data["id"].as_u64().expect("the cue has an id");
    session.execute("film.caption.remove", &json!({ "caption": caption })).expect("the cue is taken away");
    let cues = cues_of(&mut session, track);
    assert!(cues.is_empty(), "the track holds no cue: {cues:?}");
}
