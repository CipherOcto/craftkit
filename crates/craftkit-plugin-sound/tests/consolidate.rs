//! The **consolidate** of a track: the clips of a track become one
//! clip of a file that holds their sound.
//!
//! The tree's consolidate renders the sound of a track's clips over a
//! range and puts one clip of that file where the clips were.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-consolidate-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A WAV file of **one second** of silence, at the session's rate.
fn a_sound(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.wav");
    let frames = 48_000_usize;
    let samples = vec![0.0_f32; frames * 2];
    let bytes = craftkit_wav::write(2, 48_000, &samples);
    std::fs::write(&path, bytes).expect("the file is on the disk");
    path
}

/// A session with a track, a source and **two** clips of that source.
fn a_track(session: &mut Session, name: &str) -> (u64, u64) {
    let path = a_sound(name);
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 2, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 24_000, "source": source }))
        .expect("the first clip is added");
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 24_000, "length": 24_000, "source": source }),
        )
        .expect("the second clip is added");
    (track, source)
}

/// The clips of the track, in the order they are in.
fn clips_on(session: &Session, track: u64) -> Vec<(i64, i64)> {
    let document = session.document();
    let mut clips: Vec<(i64, i64)> = Vec::new();
    for clip in document.walk() {
        if clip.kind != "sound.clip" {
            continue;
        }
        // The clip is under a playlist, and the playlist is under the
        // track: the track is the parent or the grandparent.
        let Some(parent) = document.walk().find(|node| node.children.iter().any(|child| child.id == clip.id))
        else {
            continue;
        };
        let owner = if parent.kind == "sound.track" {
            Some(parent.id)
        } else {
            document
                .walk()
                .find(|node| {
                    node.kind == "sound.track" && node.children.iter().any(|child| child.id == parent.id)
                })
                .map(|node| node.id)
        };
        if owner != Some(track) {
            continue;
        }
        clips.push((clip.data["start"].as_i64().unwrap_or(0), clip.data["length"].as_i64().unwrap_or(0)));
    }
    clips.sort();
    clips
}

/// The clips of the track become **one** clip, and the file behind it
/// holds the sound of the range.
#[test]
fn the_clips_of_the_track_become_one_clip() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "consolidated");
    let directory = directory("consolidated-out");
    let outcome = session
        .execute(
            "sound.track.consolidate",
            &json!({ "track": track, "start": 0, "end": 48_000, "directory": directory }),
        )
        .expect("the track is consolidated");
    assert_eq!(outcome.data["removed"], 2);
    let path = outcome.data["path"].as_str().expect("the path is there").to_string();
    assert!(std::path::Path::new(&path).exists(), "the file is written: {path}");

    let clips = clips_on(&session, track);
    assert_eq!(clips.len(), 1, "there is one clip: {clips:?}");
    assert_eq!(clips[0], (0, 48_000));
}

/// The file holds the sound of the range: it is as long as the range
/// and it is as many channels as the session has.
#[test]
fn the_file_holds_the_sound_of_the_range() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "thesound");
    let directory = directory("thesound-out");
    let outcome = session
        .execute(
            "sound.track.consolidate",
            &json!({ "track": track, "start": 0, "end": 24_000, "directory": directory }),
        )
        .expect("the track is consolidated");
    let path = outcome.data["path"].as_str().expect("the path is there").to_string();
    let bytes = std::fs::read(&path).expect("the file is on the disk");
    let sound = craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the file is a sound");
    assert_eq!(sound.channels, 2);
    assert_eq!(sound.rate, 48_000);
    assert_eq!(sound.samples.len(), 24_000 * 2, "the file holds the range and not one sample more");
}

/// A range that covers **only part** of a clip leaves the part outside
/// it where it was.
#[test]
fn a_clip_that_crosses_the_range_keeps_the_part_outside_it() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "crossing");
    let directory = directory("crossing-out");
    session
        .execute(
            "sound.track.consolidate",
            &json!({ "track": track, "start": 12_000, "end": 36_000, "directory": directory }),
        )
        .expect("the track is consolidated");
    let clips = clips_on(&session, track);
    assert_eq!(clips.len(), 3, "the two halves and the new clip: {clips:?}");
    // The first clip keeps its head, the second keeps its tail, and
    // the new clip is in the middle.
    assert!(clips.contains(&(0, 12_000)), "the head is kept: {clips:?}");
    assert!(clips.contains(&(36_000, 12_000)), "the tail is kept: {clips:?}");
    assert!(clips.contains(&(12_000, 24_000)), "the range is the new clip: {clips:?}");
}

/// A range with **no length** is refused: there is nothing to
/// consolidate.
#[test]
fn an_empty_range_is_refused() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "empty");
    let directory = directory("empty-out");
    let error = session
        .execute(
            "sound.track.consolidate",
            &json!({ "track": track, "start": 1000, "end": 1000, "directory": directory }),
        )
        .expect_err("the range is refused");
    assert!(error.to_string().contains("starts at"), "the error says why: {error}");
}

/// A track that is **not** a track is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("notatrack-out");
    let error = session
        .execute(
            "sound.track.consolidate",
            &json!({ "track": 999_999, "start": 0, "end": 100, "directory": directory }),
        )
        .expect_err("the node is refused");
    assert!(error.to_string().contains("no node"), "the error says why: {error}");
}
