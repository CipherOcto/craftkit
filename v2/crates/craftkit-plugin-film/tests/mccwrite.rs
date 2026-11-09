//! The cues of a track are written as an **MCC file**.
//!
//! The file carries the CEA-608 pairs of the field one in a caption
//! distribution packet a frame. The rules are the tree's own.

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

/// A **sequence** at the rate of the format, and the **track** of its
/// captions.
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
    let directory = std::env::temp_dir().join(format!("craftkit-v2-mccwrite-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(name);
    path.to_string_lossy().to_string()
}

/// The **text** of the file that was written.
fn text_of(path: &str) -> String {
    std::fs::read_to_string(path).expect("the file is there")
}

/// The file says what it is: a **MacCaption MCC file**, and the rate
/// of its timecode.
#[test]
fn the_file_says_what_it_is() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-captions.mcc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "mcc" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.starts_with("File Format=MacCaption_MCC V1.0\n"), "the file is an MCC file: {text}");
    assert!(text.contains("Time Code Rate=30\n"), "and the rate of it is thirty: {text}");
    assert!(text.contains("Creation Program=craftkit\n"), "and it says who wrote it: {text}");
}

/// The **data** of a line is written with the abbreviations of the
/// format.
#[test]
fn the_data_of_a_line_uses_the_abbreviations() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-abbreviations.mcc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "mcc" }))
        .expect("the file is written");
    let text = text_of(&path);
    // The header of the packet is `61 01`, which the format writes `T`.
    let line = text.lines().find(|line| line.contains('\t')).expect("there is a line of data");
    let data = line.split('\t').nth(1).expect("the line has data");
    assert!(data.starts_with('T'), "the packet is abbreviated: {data}");
    assert!(data.contains('S'), "and so is the header of it: {data}");
}

/// The file that is written is **read back**, and the caption of the
/// track is the caption of the file.
#[test]
fn the_file_that_is_written_is_read_back() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-round-trip.mcc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "mcc" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "there is one caption: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "and it says Hello: {outcome:?}");
    // The caption is shown at the frame of the caption itself — the
    // load of it is a frame a pair, and it ends on the frame of the
    // caption — and it is cleared at the frame of its end.
    assert_eq!(cues[0]["start"], json!(2002), "it begins at its frame: {outcome:?}");
    assert_eq!(cues[0]["end"], json!(4004), "and it ends at its own: {outcome:?}");
}

/// The **drop-frame** timecode carries the mark when the file is
/// written for it.
#[test]
fn the_drop_frame_timecode_carries_the_mark() {
    let mut session = session();
    let cues = json!([{ "start": 2002, "end": 4004, "text": "Hello" }]);
    let path = a_file("the-drop-frame.mcc");
    session
        .execute(
            "film.captions.write",
            &json!({ "path": path, "format": "mcc", "cues": cues, "drop_frame": true }),
        )
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.contains("Time Code Rate=30DF"), "the rate of the file says so: {text}");
    // The **timecode** of the lines of data: the mark is in them.
    let times: Vec<&str> = text.lines().filter(|line| line.contains('\t')).collect();
    assert!(times.iter().all(|line| line.contains(';')), "the timecode carries the mark: {text}");
    let path = a_file("the-no-drop-frame.mcc");
    session
        .execute("film.captions.write", &json!({ "path": path, "format": "mcc", "cues": cues }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(text.contains("Time Code Rate=30\n"), "the rate of the file says so: {text}");
    let times: Vec<&str> = text.lines().filter(|line| line.contains('\t')).collect();
    assert!(times.iter().all(|line| !line.contains(';')), "and the timecode does not carry the mark: {text}");
}
