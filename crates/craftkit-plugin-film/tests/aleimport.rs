//! The **ALE** read back: an Avid log exchange made into a sequence.
//!
//! The media of the log are items, and the rows are clips on a track.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Node, Session};

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

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let path = craftkit_scratch::file(&format!("craftkit-v2-ale-{name}-{}", std::process::id()))
        .expect("the scratch is made");
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **log** with two rows.
fn a_log() -> String {
    let mut text = String::new();
    text.push_str("Heading\r\n");
    text.push_str("FIELD_DELIM\tTABS\r\n");
    text.push_str("VIDEO_FORMAT\t1080\r\n");
    text.push_str("AUDIO_FORMAT\t48khz\r\n");
    text.push_str("FPS\t25\r\n");
    text.push_str("\r\nColumn\r\n");
    text.push_str("Name\tTracks\tStart\tEnd\tTape\tSource File\r\n");
    text.push_str("\r\nData\r\n");
    text.push_str("the beach\tV\t00:00:00:00\t00:00:05:00\tBEACH\t/media/beach.mov\r\n");
    text.push_str("the pier\tV\t00:00:10:00\t00:00:15:00\tPIER\t/media/pier.mov\r\n");
    text
}

/// A log is read into a **sequence**: the media are items and the
/// rows are clips.
#[test]
fn a_log_is_read_into_a_sequence() {
    let mut session = session();
    let path = a_file("sequence", &a_log());
    let outcome = session
        .execute("film.import.ale", &json!({ "path": path, "name": "the film" }))
        .expect("the log is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = outcome.data["track"].as_u64().expect("the track has an id");
    let clips = outcome.data["clips"].as_array().expect("the clips are a list");
    assert_eq!(clips.len(), 2, "the two rows are clips: {outcome:?}");
    assert_eq!(clips[0]["start"], json!(0), "the first clip begins at the start");
    assert_eq!(clips[0]["duration"], json!(125), "and lasts as long as the row");
    assert_eq!(clips[1]["start"], json!(125), "and the second begins where the first ends");
    assert_eq!(clips[1]["duration"], json!(125), "and lasts as long as its row");
    // The **rate** of the sequence is the heading's.
    let rate = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    assert_eq!(rate.data["rate"], json!({ "num": 25, "den": 1 }), "the rate is the heading's: {rate:?}");
    let _ = track;
}

/// The **source** of a clip is the row's **start**, and the **media**
/// is an item with the row's name and its file.
#[test]
fn the_source_of_a_clip_is_the_rows_start() {
    let mut session = session();
    let path = a_file("source", &a_log());
    let outcome = session
        .execute("film.import.ale", &json!({ "path": path, "name": "the film" }))
        .expect("the log is read");
    let track = outcome.data["track"].as_u64().expect("the track has an id");
    let node = session.document().node(track).expect("the track is there");
    let clips: Vec<&std::sync::Arc<Node>> =
        node.children.iter().filter(|child| child.kind == "film.clip").collect();
    assert_eq!(clips.len(), 2, "there are two clips");
    assert_eq!(
        clips[1].data["source_in"],
        json!(250),
        "the second begins at ten seconds: {:?}",
        clips[1].data
    );
    assert_eq!(clips[1].data["duration"], json!(125), "and lasts five seconds");
    // The **media** is an item of the bin, with the row's name.
    let item = clips[1].data.get("item").and_then(|item| item.as_u64());
    assert!(item.is_none(), "a clip names its media with a relation, not in its payload");
    let node = session.document().node(track).expect("the track is there");
    let names: Vec<String> = node
        .children
        .iter()
        .filter(|child| child.kind == "film.clip")
        .map(|child| child.name.clone())
        .collect();
    assert_eq!(names, vec!["the beach".to_string(), "the pier".to_string()], "the rows name the clips");
}

/// A file that is **not an ALE** is refused, and so is a file that
/// cannot be read.
#[test]
fn a_file_that_is_not_an_ale_is_refused() {
    let mut session = session();
    let path = a_file("notale", "this is not a log\r\n");
    let error =
        session.execute("film.import.ale", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not an ALE file"), "the refusal says why: {error}");
    let error = session
        .execute("film.import.ale", &json!({ "path": "/there/is/no/such/file.ale" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("cannot be read"), "the refusal says why: {error}");
    let error = session.execute("film.import.ale", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}

/// A row whose **start** or **end** cannot be read is left out.
#[test]
fn a_row_that_cannot_be_read_is_left_out() {
    let mut session = session();
    let mut text = a_log();
    text.push_str("the broken\tV\tnot a timecode\t00:00:20:00\tBROKEN\t/media/broken.mov\r\n");
    let path = a_file("broken", &text);
    let outcome = session.execute("film.import.ale", &json!({ "path": path })).expect("the log is read");
    let clips = outcome.data["clips"].as_array().expect("the clips are a list");
    assert_eq!(clips.len(), 2, "the broken row is left out: {outcome:?}");
}

/// The **rate** of a heading that is not one of the tree's common
/// rates is a fraction of a thousand, and a log with no `FPS` has the
/// default rate.
#[test]
fn the_rate_is_worked_out() {
    let mut session = session();
    let text = a_log().replace("FPS\t25", "FPS\t23.976");
    let path = a_file("ntsc", &text);
    let outcome = session.execute("film.import.ale", &json!({ "path": path })).expect("the log is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let rate = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    assert_eq!(
        rate.data["rate"],
        json!({ "num": 24000, "den": 1001 }),
        "the rate is the heading's: {rate:?}"
    );
}
