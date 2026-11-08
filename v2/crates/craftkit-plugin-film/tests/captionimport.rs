//! A caption file is **imported** as a caption track of a sequence.
//!
//! The cues of the file are snapped to the frames of the sequence, and
//! cues that overlap each other are an error of the project.

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

/// A **sequence** that runs at **25** frames a second.
fn a_sequence(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    sequence
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    // A **directory** of this test's own, so that the name of the file
    // is the name the test gives it: the track is named after the file.
    let directory = std::env::temp_dir().join(format!("craftkit-v2-captionimport-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(format!("{name}.srt"));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **caption tracks** of a sequence.
fn tracks_of(session: &Session, sequence: u64) -> Vec<u64> {
    let node = session.document().node(sequence).expect("the sequence is there");
    node.children.iter().filter(|child| child.kind == "film.caption.track").map(|child| child.id).collect()
}

/// The **cues** of a track, as the answer holds them.
fn cues_of(session: &mut Session, track: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// A caption file is imported as a **new track**, and the cues are
/// snapped to the frames of the sequence.
#[test]
fn a_caption_file_is_imported_as_a_new_track() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let path = a_file("basic", "1\n00:00:01,000 --> 00:00:02,000\nHello\n");
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path }))
        .expect("the file is imported");
    let track = outcome.data["track"].as_u64().expect("the track has an id");
    assert_eq!(tracks_of(&session, sequence), vec![track], "the sequence has the new track");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 1, "the cue is there: {cues:?}");
    assert_eq!(cues[0]["start"], json!(25), "and it is snapped to the frames: {cues:?}");
    assert_eq!(cues[0]["end"], json!(50), "and its end too: {cues:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "and the text: {cues:?}");
}

/// The **name** of the track is the name of the file, unless a name is
/// given: the tree's own rule.
#[test]
fn the_track_is_named_after_the_file() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let path = a_file("named", "1\n00:00:01,000 --> 00:00:02,000\nHello\n");
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path }))
        .expect("the file is imported");
    assert_eq!(outcome.data["name"], json!("named"), "the track is named after the file: {outcome:?}");
    let path = a_file("other", "1\n00:00:01,000 --> 00:00:02,000\nHello\n");
    let outcome = session
        .execute(
            "film.caption.import",
            &json!({ "sequence": sequence, "path": path, "name": "the subtitles" }),
        )
        .expect("the file is imported");
    assert_eq!(outcome.data["name"], json!("the subtitles"), "and the name that is given: {outcome:?}");
}

/// The cues of a file that **overlap each other** are an error of the
/// project, and the whole import is refused.
#[test]
fn a_file_whose_cues_overlap_is_refused() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let path = a_file(
        "overlap",
        "1\n00:00:01,000 --> 00:00:04,000\nFirst\n\n2\n00:00:02,000 --> 00:00:03,000\nSecond\n",
    );
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path }))
        .expect_err("the import is refused");
    assert!(outcome.to_string().contains("overlap"), "the refusal says why: {outcome}");
    assert!(tracks_of(&session, sequence).is_empty(), "and the sequence is as it was");
}

/// A sequence with **no rate** has no frames, and the import is
/// refused.
#[test]
fn a_sequence_with_no_rate_is_refused() {
    let mut session = session();
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    let path = a_file("norate", "1\n00:00:01,000 --> 00:00:02,000\nHello\n");
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path }))
        .expect_err("the import is refused");
    assert!(outcome.to_string().contains("no rate"), "the refusal says why: {outcome}");
}
