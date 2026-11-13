//! The cues of a caption track are **written** as an SCC file.
//!
//! An SCC file carries CEA-608 byte pairs with SMPTE timecodes, and the
//! captions of a track are loaded into it as pop-on captions: the load
//! is scheduled into the free frames before the cue, so that the end of
//! the caption lands on the cue's own frame. The rules are the tree's
//! own.

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

/// A **caption track** at the rate of the format, and its id.
fn a_track(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 1000 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "30000/1001" }))
        .expect("the rate is set");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str) -> String {
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-scc-write-{}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join(name);
    path.to_string_lossy().to_string()
}

/// The **text** of a file that was written.
fn text_of(path: &str) -> String {
    std::fs::read_to_string(path).expect("the file is there")
}

/// The cues of a track are written as an **SCC file**, and the file
/// says what it is.
#[test]
fn the_cues_of_a_track_are_written_as_an_scc_file() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-cues.scc");
    let outcome = session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "scc" }))
        .expect("the file is written");
    assert_eq!(outcome.data["count"], json!(1), "one cue was written: {outcome:?}");
    assert_eq!(outcome.data["format"], json!("scc"), "and it is an SCC file: {outcome:?}");
    let text = text_of(&path);
    assert!(text.starts_with("Scenarist_SCC V1.0\n\n"), "the file says what it is: {text}");
}

/// The file that is written is **read back**, and the cues of the
/// track are the cues of the file.
///
/// The tree's own three captions, with the letters the format has and
/// the ones it does not.
#[test]
fn the_file_that_is_written_is_read_back() {
    let mut session = session();
    let track = a_track(&mut session);
    for (start, end, text) in [
        (60i64, 120i64, "Hello, world!"),
        (120, 200, "Second caption\nwith two lines"),
        (260, 300, "Ñandú ♪ {x} * café “q”"),
    ] {
        session
            .execute("film.caption.add", &json!({ "track": track, "start": start, "end": end, "text": text }))
            .expect("the cue is added");
    }
    let path = a_file("round-trip.scc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "scc" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 3, "there are three cues: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Hello, world!"), "the first caption is the same: {outcome:?}");
    assert_eq!(cues[1]["text"], json!("Second caption\nwith two lines"), "and so is the second: {outcome:?}");
    assert_eq!(cues[2]["text"], json!("Ñandú ♪ {x} * café “q”"), "and so is the third: {outcome:?}");
    // The **frames** of the file are the frames of the captions: the
    // rate of the format is twenty-nine and ninety-seven a second, so
    // a frame is thirty-three and a third milliseconds.
    assert_eq!(cues[0]["start"], json!(2002), "the first caption begins at its frame: {outcome:?}");
    assert_eq!(cues[0]["end"], json!(4004), "and ends at its own: {outcome:?}");
    assert_eq!(cues[1]["start"], json!(4004), "the second begins where the first ended: {outcome:?}");
    assert_eq!(cues[2]["start"], json!(8675), "and the third is at its own frame: {outcome:?}");
}

/// A caption that begins at **zero** is not lost: it is delayed, and it
/// is shown when its load has been sent.
#[test]
fn a_caption_at_zero_is_delayed_and_not_lost() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 0, "end": 60, "text": "Start" }))
        .expect("the cue is added");
    let path = a_file("at-zero.scc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "scc" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "the caption is there: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Start"), "with its text: {outcome:?}");
    assert_eq!(cues[0]["end"], json!(2002), "and it ends at its own frame: {outcome:?}");
}

/// A **long line** is wrapped over the rows of the format, and it is
/// read back as the rows it was written over.
#[test]
fn a_long_line_is_wrapped_over_the_rows_of_the_format() {
    let mut session = session();
    let track = a_track(&mut session);
    let text = "the quick brown fox jumps over the lazy dog and keeps running far away";
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": text }))
        .expect("the cue is added");
    let path = a_file("wrapped.scc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "scc" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "there is one caption: {outcome:?}");
    let read = cues[0]["text"].as_str().expect("the caption is text");
    let rows: Vec<&str> = read.lines().collect();
    assert!(rows.len() > 1, "the caption is over more than one row: {read}");
    assert!(rows.iter().all(|row| row.chars().count() <= 32), "and no row is wider than the format: {read}");
    assert_eq!(rows.join(" "), text, "and the words are the same: {read}");
}

/// The **timecode** of the file carries the drop-frame mark when the
/// file is written for it, and it does not when it is not.
#[test]
fn the_timecode_carries_the_drop_frame_mark_only_when_it_is_asked_for() {
    let mut session = session();
    let cues = json!([{ "start": 2002, "end": 4004, "text": "Hello" }]);
    let path = a_file("drop-frame.scc");
    session
        .execute(
            "film.captions.write",
            &json!({ "path": path, "format": "scc", "cues": cues, "drop_frame": true }),
        )
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.contains(';'), "the timecode carries the drop-frame mark: {text}");
    let path = a_file("no-drop-frame.scc");
    session
        .execute("film.captions.write", &json!({ "path": path, "format": "scc", "cues": cues }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(!text.contains(';'), "and it does not when it is not asked for: {text}");
}

/// A format that is **not one** of the formats is refused by name.
#[test]
fn a_format_that_is_not_one_is_refused_by_name() {
    let mut session = session();
    let path = a_file("not-a-format.scc");
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "format": "not-a-format", "cues": [] }))
        .expect_err("the format is refused");
    let message = error.to_string();
    assert!(message.contains("not-a-format"), "the format is named: {message}");
    assert!(message.contains("scc"), "and the formats are named: {message}");
}
