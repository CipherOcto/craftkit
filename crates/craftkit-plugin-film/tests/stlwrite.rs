//! The cues of a caption track are **written** as an EBU STL file.
//!
//! The file is a general block and then a run of text and timing
//! blocks, and the text of a cue is ISO/IEC 6937.

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

/// A **sequence** with a **caption track** on it, at the rate, and
/// their ids.
fn a_track(session: &mut Session, rate: &str) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": rate }))
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
    let directory = std::env::temp_dir().join(format!("craftkit-v2-stl-write-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(name);
    path.to_string_lossy().to_string()
}

/// The **bytes** of a file that was written.
fn bytes_of(path: &str) -> Vec<u8> {
    std::fs::read(path).expect("the file is there")
}

/// The cues of a track are written as an **EBU STL file**: a general
/// block of a thousand and twenty-four bytes and a block of a hundred
/// and twenty-eight for every cue.
#[test]
fn the_cues_of_a_track_are_written_as_an_stl_file() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 25, "end": 50, "text": "First line" }))
        .expect("the cue is added");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 75, "end": 100, "text": "Bye" }))
        .expect("the cue is added");
    let path = a_file("the-cues.stl");
    let outcome = session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "stl" }))
        .expect("the file is written");
    assert_eq!(outcome.data["count"], json!(2), "two cues were written: {outcome:?}");
    let bytes = bytes_of(&path);
    assert_eq!(bytes.len(), 1024 + 128 * 2, "the file is a general block and two blocks");
    assert_eq!(&bytes[0..3], b"850", "the file says what it is");
    assert_eq!(&bytes[3..11], b"STL25.01", "and the rate it is written at");
}

/// The file that is written is **read back**, and the cues of the
/// track are the cues of the file.
#[test]
fn the_file_that_is_written_is_read_back() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 25, "end": 50, "text": "First line" }))
        .expect("the cue is added");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 75, "end": 100, "text": "Bye" }))
        .expect("the cue is added");
    let path = a_file("round-trip.stl");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "stl" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "stl" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 2, "there are two cues: {outcome:?}");
    // The programme of a file starts at zero, and the timecodes of the
    // cues are the frames of the sequence.
    assert_eq!(cues[0]["start"], json!(1000), "the first cue is a second on: {outcome:?}");
    assert_eq!(cues[0]["end"], json!(2000), "and it lasts a second: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("First line"), "with its text: {outcome:?}");
    assert_eq!(cues[1]["start"], json!(3000), "the second cue is three seconds on: {outcome:?}");
    assert_eq!(cues[1]["text"], json!("Bye"), "with its text: {outcome:?}");
}

/// A sequence at **twenty-nine and ninety-seven** is written at that
/// rate, as the tree's rule says.
#[test]
fn a_sequence_at_twenty_nine_ninety_seven_is_written_at_that_rate() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session, "30000/1001");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 30, "end": 60, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("ntsc.stl");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "stl" }))
        .expect("the file is written");
    let bytes = bytes_of(&path);
    assert_eq!(&bytes[3..11], b"STL30.01", "the file says the rate it is written at");
}

/// The **letters** of ISO/IEC 6937 are written as the set holds them.
#[test]
fn the_letters_of_iso_sixty_nine_thirty_seven_are_written() {
    let mut session = session();
    let (_sequence, track) = a_track(&mut session, "25");
    session
        .execute(
            "film.caption.add",
            &json!({ "track": track, "start": 25, "end": 50, "text": "Ñandú café £" }),
        )
        .expect("the cue is added");
    let path = a_file("letters.stl");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "stl" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "stl" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "there is one cue: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Ñandú café £"), "and the letters are the same: {outcome:?}");
}
