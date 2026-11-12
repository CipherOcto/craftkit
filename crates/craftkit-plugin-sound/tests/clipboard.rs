//! The **clipboard**: the material that was copied, waiting to be
//! pasted.
//!
//! The tree's editor copies the material of a range and pastes it
//! somewhere else, and this tree's battery had no clipboard at all.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-clipboard-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **mono** sound of a second.
fn a_sound(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.wav");
    let mut samples = Vec::with_capacity(48_000);
    for frame in 0..48_000_u32 {
        samples.push(frame as f32 / 48_000.0);
    }
    std::fs::write(&path, craftkit_wav::write(1, 48_000, &samples)).expect("the file is on the disk");
    path
}

/// A session with a track and a clip of a second on it.
fn a_track(session: &mut Session, name: &str) -> (u64, u64) {
    let path = a_sound(name);
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 1, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// The clips of the document, as `(start, length)`.
fn clips_of(session: &Session) -> Vec<(i64, i64)> {
    let mut clips: Vec<(i64, i64)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| {
            let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            let length = node.data.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
            (start, length)
        })
        .collect();
    clips.sort();
    clips
}

/// A range of a track is **copied** into the clipboard, and the
/// clipboard says how long the range was.
#[test]
fn a_range_is_copied_into_the_clipboard() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "copied");
    let outcome = session
        .execute("sound.track.copy", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is copied");
    assert_eq!(outcome.data["copied"], 1);
    assert_eq!(outcome.data["length"], 24_000);
    let read = session.execute("sound.track.clipboard.read", &json!({})).expect("the clipboard is read");
    assert_eq!(read.data["length"], 24_000);
    assert_eq!(read.data["clips"][0]["length"], 24_000, "the clip is trimmed to the range");
}

/// The copy does not change the track: the material is still there.
#[test]
fn a_copy_changes_nothing() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "unchanged");
    session
        .execute("sound.track.copy", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is copied");
    assert_eq!(clips_of(&session), vec![(0, 48_000)], "the clip is still there");
}

/// The clipboard is **pasted** onto a track: the material is laid at
/// the place the caller gives.
#[test]
fn the_clipboard_is_pasted_onto_a_track() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "pasted");
    session
        .execute("sound.track.copy", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is copied");
    let outcome = session
        .execute("sound.track.paste", &json!({ "track": track, "at": 96_000 }))
        .expect("the clipboard is pasted");
    assert_eq!(outcome.data["clips"].as_array().expect("the clips are there").len(), 1);
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 2, "the clip and the pasted one: {clips:?}");
    assert_eq!(clips[1], (96_000, 24_000), "and it is where it was pasted");
}

/// A **cut** is a copy and a removal: the clipboard holds the material
/// and the track does not.
#[test]
fn a_cut_copies_and_removes() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "cut");
    let outcome = session
        .execute("sound.track.cut", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is cut");
    assert_eq!(outcome.data["cut"], 1);
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 1, "the clip is still there");
    // The part of the clip that was **inside** the range is gone, and
    // the part outside it is kept.
    assert_eq!(clips[0], (24_000, 24_000), "and it is the part that was not cut");
    let read = session.execute("sound.track.clipboard.read", &json!({})).expect("the clipboard is read");
    assert_eq!(read.data["length"], 24_000, "the clipboard holds the range");
}

/// An **empty** clipboard is refused: there is nothing to paste.
#[test]
fn an_empty_clipboard_is_refused() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "empty");
    let error = session
        .execute("sound.track.paste", &json!({ "track": track, "at": 0 }))
        .expect_err("the paste is refused");
    assert!(error.to_string().contains("clipboard is empty"), "the error says why: {error}");
}

/// A range with **no length** is refused: there is nothing to copy.
#[test]
fn an_empty_range_is_refused() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "norange");
    let error = session
        .execute("sound.track.copy", &json!({ "track": track, "start": 1000, "end": 1000 }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("starts at"), "the error says why: {error}");
}

/// The **automation** of a range is copied with it: the breakpoints
/// that were inside the range are in the clipboard.
#[test]
fn the_automation_of_a_range_is_copied() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "automation");
    session
        .execute(
            "sound.lane.add",
            &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [12_000, 1.0]] }),
        )
        .expect("the lane is added");
    session
        .execute("sound.track.copy", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is copied");
    let read = session.execute("sound.track.clipboard.read", &json!({})).expect("the clipboard is read");
    assert_eq!(read.data["lanes"], 1, "the lane is in the clipboard");
    assert_eq!(read.data["automation"][0]["param"], "volume");
    assert_eq!(read.data["automation"][0]["points"].as_array().expect("the points are there").len(), 2);
}

/// The **paste** writes the automation on the track: the breakpoints
/// are put where the material was put.
#[test]
fn the_paste_writes_the_automation() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "written");
    session
        .execute(
            "sound.lane.add",
            &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [12_000, 1.0]] }),
        )
        .expect("the lane is added");
    session
        .execute("sound.track.copy", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is copied");
    session
        .execute("sound.track.paste", &json!({ "track": track, "at": 96_000 }))
        .expect("the clipboard is pasted");
    let points = lane_points(&session, track, "volume");
    assert!(points.contains(&(96_000, 0.0)), "the breakpoint is where the material was put: {points:?}");
    assert!(points.contains(&(108_000, 1.0)), "and the other one with it: {points:?}");
}

/// A **cut** takes the automation of the range away with it.
#[test]
fn a_cut_takes_the_automation_away() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "cut-automation");
    session
        .execute(
            "sound.lane.add",
            &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [12_000, 1.0], [48_000, 0.0]] }),
        )
        .expect("the lane is added");
    session
        .execute("sound.track.cut", &json!({ "track": track, "start": 0, "end": 24_000 }))
        .expect("the range is cut");
    let points = lane_points(&session, track, "volume");
    assert!(!points.contains(&(0, 0.0)), "the breakpoint in the range is gone: {points:?}");
    assert!(!points.contains(&(12_000, 1.0)), "and the other one: {points:?}");
    assert!(points.contains(&(48_000, 0.0)), "and the one outside it is still there: {points:?}");
}

/// The **breakpoints** of a lane, read back.
///
/// A breakpoint is a pair: where, and what value.
fn lane_points(session: &Session, _track: u64, param: &str) -> Vec<(i64, f64)> {
    let document = session.document();
    for lane in document.walk().filter(|node| node.kind == "sound.lane") {
        if lane.data.get("param").and_then(|param| param.as_str()) != Some(param) {
            continue;
        }
        let Some(points) = lane.data.get("points").and_then(|points| points.as_array()) else { continue };
        let mut found = Vec::new();
        for point in points {
            let at = point.get(0).and_then(|at| at.as_i64()).unwrap_or(0);
            let value = point.get(1).and_then(|value| value.as_f64()).unwrap_or(0.0);
            found.push((at, value));
        }
        return found;
    }
    Vec::new()
}
