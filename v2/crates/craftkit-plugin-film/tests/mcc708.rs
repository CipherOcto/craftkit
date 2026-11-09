//! The **CEA-708 service data** of an MCC file.
//!
//! The file carries the service data beside the CEA-608 pairs of the
//! field one, and a file that carries only the service data is read
//! with the service decoder.

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
    let directory = std::env::temp_dir().join(format!("craftkit-v2-mcc708-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    directory.join(name).to_string_lossy().to_string()
}

/// A track with **two** captions on it.
fn a_track_with_captions(session: &mut Session) -> u64 {
    let track = a_track(session);
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 200, "end": 260, "text": "Goodbye" }))
        .expect("the cue is added");
    track
}

/// The file carries the **service data** beside the field one.
#[test]
fn the_file_carries_the_service_data_beside_the_field_one() {
    let mut session = session();
    let track = a_track_with_captions(&mut session);
    let path = a_file("both.mcc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "mcc" }))
        .expect("the file is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    assert!(
        text.contains("CEA-608 field 1 and CEA-708 service 1 data"),
        "the file says what it carries: {text}"
    );
    // The **lines** of the file: the service data is written in the
    // frames before the captions are shown.
    let lines: Vec<&str> = text.lines().filter(|line| line.contains('\t')).collect();
    assert!(lines.len() > 2, "there are lines of data: {lines:?}");
}

/// A file that carries **only** the service data is read back.
#[test]
fn a_file_that_carries_only_the_service_data_is_read_back() {
    let mut session = session();
    let track = a_track_with_captions(&mut session);
    let path = a_file("service.mcc");
    session
        .execute(
            "film.captions.export",
            &json!({ "track": track, "path": path, "format": "mcc", "cea_608": false }),
        )
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 2, "both captions are read: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "the first one is there: {outcome:?}");
    assert_eq!(cues[1]["text"], json!("Goodbye"), "and the second one too: {outcome:?}");
}

/// The **field one comes first**: a file that carries both is read as
/// the field one.
#[test]
fn the_field_one_comes_first() {
    let mut session = session();
    let track = a_track_with_captions(&mut session);
    let path = a_file("both-read.mcc");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "mcc" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 2, "both captions are read: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "the first one is there: {outcome:?}");
    assert_eq!(cues[1]["text"], json!("Goodbye"), "and the second one too: {outcome:?}");
}

/// A file that carries **neither** stream has no caption in it.
#[test]
fn a_file_that_carries_neither_stream_has_no_caption_in_it() {
    let mut session = session();
    let track = a_track_with_captions(&mut session);
    let path = a_file("empty.mcc");
    session
        .execute(
            "film.captions.export",
            &json!({
                "track": track,
                "path": path,
                "format": "mcc",
                "cea_608": false,
                "cea_708": false,
            }),
        )
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert!(cues.is_empty(), "and there is no caption in it: {outcome:?}");
}
