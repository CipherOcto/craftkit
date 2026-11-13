//! The cues of a track are written as a **TTML file**.
//!
//! The file is an IMSC1 text profile, and the times of the captions
//! are frame offsets when they are on a frame boundary of the rate of
//! the sequence, and clock times otherwise. The rules are the tree's
//! own.

use std::sync::atomic::{AtomicUsize, Ordering};

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

/// A **sequence** at a rate, and the **track** of its captions.
fn a_track(session: &mut Session, rate: &str) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 1000 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": rate }))
        .expect("the rate is set");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
///
/// The directory is **unique per call**: the tests of one binary run in
/// parallel threads, and two tests that shared a directory would race —
/// one of them removing the folder the other is writing into.
fn a_file(name: &str) -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-ttmlwrite-{}-{unique}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join(name);
    path.to_string_lossy().to_string()
}

/// The **text** of the file that was written.
fn text_of(path: &str) -> String {
    std::fs::read_to_string(path).expect("the file is there")
}

/// The file says what it is: an **IMSC1 text profile**.
#[test]
fn the_file_says_what_it_is() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-captions.ttml");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "ttml" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"), "the file is XML: {text}");
    assert!(
        text.contains("ttp:profile=\"http://www.w3.org/ns/ttml/profile/imsc1/text\""),
        "and it is the IMSC1 profile: {text}"
    );
    assert!(text.contains("ttp:frameRate=\"25\""), "and it has the rate of the sequence: {text}");
}

/// The times of the captions are **frame offsets** when they are on a
/// frame boundary.
#[test]
fn the_times_of_the_captions_are_frame_offsets() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-frames.ttml");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "ttml" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.contains("begin=\"60f\""), "the begin is a frame: {text}");
    assert!(text.contains("end=\"120f\""), "and the end is one too: {text}");
}

/// A time that is **not on a frame boundary** is a clock.
#[test]
fn a_time_that_is_not_on_a_frame_boundary_is_a_clock() {
    let mut session = session();
    let track = a_track(&mut session, "30000/1001");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 1, "end": 2, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-clock.ttml");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "ttml" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.contains("begin=\"00:00:00.033\""), "the begin is a clock: {text}");
    assert!(!text.contains("1f\""), "and it is not a frame: {text}");
}

/// The **text** of a caption is escaped, and a line break in it is a
/// `<br/>`.
#[test]
fn the_text_of_a_caption_is_escaped() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute(
            "film.caption.add",
            &json!({ "track": track, "start": 60, "end": 120, "text": "a & b\nsecond line" }),
        )
        .expect("the cue is added");
    let path = a_file("the-text.ttml");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "ttml" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.contains("a &amp; b<br/>second line"), "the text is as it should be: {text}");
}

/// The file that is written is **read back**, and the caption of the
/// track is the caption of the file.
#[test]
fn the_file_that_is_written_is_read_back() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-round-trip.ttml");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "ttml" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "ttml" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "there is one caption: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "and it says Hello: {outcome:?}");
    // The frame sixty of twenty-five frames a second is two thousand
    // four hundred milliseconds.
    assert_eq!(cues[0]["start"], json!(2400), "it begins at its time: {outcome:?}");
    assert_eq!(cues[0]["end"], json!(4800), "and it ends at its own: {outcome:?}");
}
