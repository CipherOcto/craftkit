//! The **repeating** of a range: the material of a range, again.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-repeat-{name}")).expect("the directory is made");
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

/// The material of a range is laid again after it.
#[test]
fn the_material_of_a_range_is_laid_again() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "repeat");
    let outcome = session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 0, "end": 24_000, "count": 1 }))
        .expect("the range is repeated");
    assert_eq!(outcome.data["repeats"], 1);
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 2, "the clip and its copy: {clips:?}");
    assert_eq!(clips[0], (0, 48_000), "the original is where it was");
    assert_eq!(clips[1], (24_000, 24_000), "and the copy is after the range");
}

/// The range is repeated as many times as the caller asks for.
#[test]
fn the_range_is_repeated_as_many_times_as_asked() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "thrice");
    session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 0, "end": 12_000, "count": 3 }))
        .expect("the range is repeated");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 4, "the clip and three copies: {clips:?}");
    assert_eq!(clips[1], (12_000, 12_000));
    assert_eq!(clips[2], (24_000, 12_000));
    assert_eq!(clips[3], (36_000, 12_000));
}

/// A copy is **trimmed** to the range: it is as long as the range and
/// it plays the material from where the range began.
#[test]
fn a_copy_is_trimmed_to_the_range() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "trimmed");
    session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 12_000, "end": 24_000, "count": 1 }))
        .expect("the range is repeated");
    let copy = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .max_by_key(|node| node.data["start"].as_i64().unwrap_or(0))
        .expect("the copy is there");
    // The copy is laid **after** the range — the tree's repeat puts it
    // at the range's end — and it is as long as the range.
    assert_eq!(copy.data["start"], 24_000, "the copy is after the range");
    assert_eq!(copy.data["length"], 12_000, "and it is as long as the range");
    assert_eq!(copy.data["offset"], 12_000, "and it plays the material from where the range began");
}

/// The copy plays the **source** of the clip it was made from: a copy
/// that plays nothing is a copy of nothing.
#[test]
fn a_copy_plays_the_source_of_the_clip_it_was_made_from() {
    let mut session = session();
    let (track, clip) = a_track(&mut session, "source");
    let source = session
        .document()
        .node(clip)
        .and_then(|node| node.data["source"].as_u64())
        .expect("the clip has a source");
    session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 0, "end": 24_000, "count": 1 }))
        .expect("the range is repeated");
    let copy = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .max_by_key(|node| node.data["start"].as_i64().unwrap_or(0))
        .expect("the copy is there");
    assert_eq!(copy.data["source"].as_u64(), Some(source), "the copy plays the source");
}

/// A range with **no length** is refused: there is nothing to repeat.
#[test]
fn an_empty_range_is_refused() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "empty");
    let error = session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 1000, "end": 1000, "count": 1 }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("starts at"), "the error says why: {error}");
}
