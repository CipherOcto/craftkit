//! The **ALE** of a sequence: an Avid log exchange.
//!
//! A tab-delimited clip log with three sections — the heading, the
//! columns and the data — and the lines end with CR LF.

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

/// A **bin**, an **item** in it, and a **track** in the item.
fn a_timeline(session: &mut Session, name: &str, path: &str, duration: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": name, "bin": bin, "item": 1, "duration": duration, "path": path }),
        )
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// The **ALE** of a sequence, as the command writes it.
fn ale_of(session: &mut Session, sequence: u64) -> String {
    let outcome =
        session.execute("film.export.ale", &json!({ "sequence": sequence })).expect("the log is written");
    outcome.data["ale"].as_str().expect("the log is a text").to_string()
}

/// The log has the three **sections** of the tree's own, and the
/// lines end with CR LF.
#[test]
fn the_log_has_the_three_sections() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the clip", "/media/the clip.mov", 100);
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": sequence,
                "start": 0,
                "duration": 50,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    assert!(outcome.data["id"].as_u64().is_some(), "the clip has an id");
    let ale = ale_of(&mut session, sequence);
    assert!(ale.starts_with("Heading\r\n"), "the log begins with the heading: {ale}");
    assert!(ale.contains("\r\n\r\nColumn\r\n"), "and the columns come after it");
    assert!(ale.contains("\r\n\r\nData\r\n"), "and the data after them");
    assert!(ale.contains("FIELD_DELIM\tTABS\r\n"), "and the field delimiter is tabs");
    assert!(ale.contains("FPS\t25\r\n"), "and the rate is the sequence's");
}

/// A **row** is a clip of the timeline: the name of the media it
/// plays, the tracks it is on, the timecodes of its **source**, the
/// reel and the file.
#[test]
fn a_row_is_a_clip_of_the_timeline() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the clip", "/media/the clip.mov", 100);
    session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": sequence,
                "start": 0,
                "duration": 50,
                "source_in": 10,
            }),
        )
        .expect("the clip is added");
    let ale = ale_of(&mut session, sequence);
    let row = ale.lines().find(|line| line.starts_with("the clip")).expect("there is a row");
    assert!(row.contains("\tV\t"), "the row is on the video channel: {row}");
    assert!(row.contains("00:00:00:10\t"), "and it begins at the source: {row}");
    assert!(row.contains("00:00:02:10\t"), "and ends at the source: {row}");
    assert!(row.contains("\tTHE_CLIP\t"), "and the reel is the name of the media: {row}");
    assert!(row.contains("/media/the clip.mov"), "and the file is said: {row}");
}

/// An **audio** track is numbered `A1`, `A2` and so on.
#[test]
fn an_audio_track_is_numbered() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the clip", "/media/the clip.mov", 100);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": sequence, "kind": "audio" }))
        .expect("the track is added");
    let sound = outcome.data["id"].as_u64().expect("the track has an id");
    session
        .execute(
            "film.clip.add",
            &json!({
                "track": sound,
                "item": sequence,
                "start": 0,
                "duration": 50,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    let _ = track;
    let ale = ale_of(&mut session, sequence);
    let row = ale.lines().find(|line| line.starts_with("the clip")).expect("there is a row");
    assert!(row.contains("\tA1\t"), "the row is on the first audio channel: {row}");
}

/// A value holds **no tab** and **no line break**: they are turned
/// into spaces.
#[test]
fn a_value_holds_no_tab_and_no_line_break() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the\tclip", "/media/the clip.mov", 100);
    session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": sequence,
                "start": 0,
                "duration": 50,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    let ale = ale_of(&mut session, sequence);
    let row = ale.lines().find(|line| line.starts_with("the clip")).expect("there is a row");
    assert!(row.starts_with("the clip\t"), "the tab in the name became a space: {row}");
}
