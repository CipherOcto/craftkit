//! The cues of a track are written as a **DFXP file**.
//!
//! The DFXP profile is the older one: it has no rate, and the times of
//! the captions are clock times. The rules are the tree's own.

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
fn a_file(name: &str) -> String {
    let directory = std::env::temp_dir().join(format!("craftkit-v2-dfxpwrite-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(name);
    path.to_string_lossy().to_string()
}

/// The **text** of the file that was written.
fn text_of(path: &str) -> String {
    std::fs::read_to_string(path).expect("the file is there")
}

/// The file says what it is: the **DFXP presentation** profile.
#[test]
fn the_file_says_what_it_is() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-captions.dfxp");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "dfxp" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(
        text.contains("ttp:profile=\"http://www.w3.org/ns/ttml/profile/dfxp-presentation\""),
        "the file is the DFXP profile: {text}"
    );
}

/// The **DFXP profile has no rate**, and the times of the captions are
/// clock times.
#[test]
fn the_dfxp_profile_has_no_rate() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-clock.dfxp");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "dfxp" }))
        .expect("the file is written");
    let text = text_of(&path);
    assert!(!text.contains("ttp:frameRate"), "the file has no rate: {text}");
    assert!(!text.contains("ttp:frameRateMultiplier"), "and no multiplier: {text}");
    assert!(text.contains("begin=\"00:00:02.400\""), "the begin is a clock: {text}");
    assert!(text.contains("end=\"00:00:04.800\""), "and the end is one too: {text}");
}

/// The file that is written is **read back**: a DFXP file is a TTML
/// file, and it is read as one.
#[test]
fn the_file_that_is_written_is_read_back() {
    let mut session = session();
    let track = a_track(&mut session, "25");
    session
        .execute("film.caption.add", &json!({ "track": track, "start": 60, "end": 120, "text": "Hello" }))
        .expect("the cue is added");
    let path = a_file("the-round-trip.dfxp");
    session
        .execute("film.captions.export", &json!({ "track": track, "path": path, "format": "dfxp" }))
        .expect("the file is written");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "dfxp" }))
        .expect("the file is read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "there is one caption: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "and it says Hello: {outcome:?}");
    assert_eq!(cues[0]["start"], json!(2400), "it begins at its time: {outcome:?}");
}
