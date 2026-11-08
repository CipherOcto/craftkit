//! The cues of a caption track are **exported** as a caption file.
//!
//! The frames of the track are the milliseconds of the file.

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
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (sequence, track)
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str) -> String {
    let directory = std::env::temp_dir().join(format!("craftkit-v2-captions-export-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(name);
    path.to_string_lossy().to_string()
}

/// The **text** of a file that was written.
fn text_of(path: &str) -> String {
    let bytes = std::fs::read(path).expect("the file is there");
    String::from_utf8_lossy(&bytes).to_string()
}

/// The cues of a track are written as a **caption file**: the frames
/// of the track are the milliseconds of the file.
#[test]
fn the_cues_of_a_track_are_written_as_a_caption_file() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 25, "end": 50, "text": "First line" }))
        .expect("the cue is added");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 75, "end": 100, "text": "Bye" }))
        .expect("the cue is added");
    let path = a_file("subrip.srt");
    let outcome = session
        .execute("film.captions.export", &json!({ "track": track, "path": path }))
        .expect("the file is written");
    assert_eq!(outcome.data["count"], json!(2), "two cues were written: {outcome:?}");
    let written = text_of(&path);
    assert_eq!(
        written, "1\n00:00:01,000 --> 00:00:02,000\nFirst line\n\n2\n00:00:03,000 --> 00:00:04,000\nBye\n\n",
        "the file is as the tree writes it"
    );
}

/// What is written is read back as the cues that were written.
#[test]
fn what_is_written_is_read_back() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 25, "end": 50, "text": "First line" }))
        .expect("the cue is added");
    let path = a_file("round.srt");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path }))
        .expect("the file is written");
    let outcome = session.execute("film.captions.read", &json!({ "path": path })).expect("the cues are read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "there is one cue: {outcome:?}");
    assert_eq!(cues[0]["start"], json!(1000), "and it starts where it did: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("First line"), "and says what it said: {outcome:?}");
}

/// A format that is **not one of the tree's** is refused by name.
#[test]
fn a_format_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session);
    let path = a_file("bad.srt");
    let outcome = session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "substation" }));
    assert!(outcome.is_err(), "the format is refused: {outcome:?}");
}
