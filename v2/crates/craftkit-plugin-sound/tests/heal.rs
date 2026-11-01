//! The **healing** of a track: the clips that were separated are put
//! back together.
//!
//! The tree's editor heals a separation: it joins the clips of a track
//! that are the same sound continued. Here the rule is the same, and
//! the cut of §212 is the cut that is undone.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-heal-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A sound of that many frames, all of them full.
fn a_sound(path: &str, frames: usize) {
    let bytes = craftkit_wav::write(1, 48_000, &vec![1.0_f32; frames]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track, a source and a clip of that many frames.
fn a_clip(session: &mut Session, path: &str, length: i64) -> (u64, u64) {
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": length, "source": source }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
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

/// A clip that was **separated** is healed back into one clip.
#[test]
fn a_separated_clip_is_healed() {
    let mut session = session();
    let directory = directory("separate");
    let path = format!("{directory}/sound.wav");
    a_sound(&path, 48_000);
    let (track, clip) = a_clip(&mut session, &path, 48_000);
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 24_000 }))
        .expect("the clip is separated");
    assert_eq!(clips_of(&session), [(0, 24_000), (24_000, 24_000)], "the clip is in two");
    let healed = session.execute("sound.clip.heal", &json!({ "track": track })).expect("the track is healed");
    assert_eq!(healed.data["healed"], 1, "one separation was healed");
    assert_eq!(clips_of(&session), [(0, 48_000)], "and the clip is one again");
}

/// Two clips of **different sources** are not the same sound
/// continued, and they stay apart — even when they touch.
#[test]
fn clips_of_different_sources_stay_apart() {
    let mut session = session();
    let directory = directory("apart");
    let first_path = format!("{directory}/one.wav");
    let second_path = format!("{directory}/two.wav");
    a_sound(&first_path, 48_000);
    a_sound(&second_path, 48_000);
    session.execute("sound.source.add", &json!({ "path": first_path })).expect("the source is added");
    session.execute("sound.source.add", &json!({ "path": second_path })).expect("the source is added");
    let sources: Vec<u64> =
        session.document().walk().filter(|node| node.kind == "sound.source").map(|node| node.id).collect();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 0, "length": 24_000, "source": sources[0] }),
        )
        .expect("the first clip is added");
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 24_000, "length": 24_000, "source": sources[1] }),
        )
        .expect("the second clip is added");
    let healed = session.execute("sound.clip.heal", &json!({ "track": track })).expect("the track is healed");
    assert_eq!(healed.data["healed"], 0, "nothing was healed");
    assert_eq!(clips_of(&session).len(), 2, "and the clips are still two");
}

/// Two clips that **overlap** are not a separation, and they stay
/// apart.
#[test]
fn clips_that_overlap_stay_apart() {
    let mut session = session();
    let directory = directory("overlap");
    let path = format!("{directory}/sound.wav");
    a_sound(&path, 48_000);
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 24_000, "source": source }))
        .expect("the first clip is added");
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 12_000, "length": 24_000, "source": source }),
        )
        .expect("the second clip is added");
    let healed = session.execute("sound.clip.heal", &json!({ "track": track })).expect("the track is healed");
    assert_eq!(healed.data["healed"], 0, "the clips overlap");
    assert_eq!(clips_of(&session).len(), 2, "and they stay apart");
}

/// Two clips that are played **differently** are not the same sound
/// continued, and they stay apart.
#[test]
fn clips_played_differently_stay_apart() {
    for command in ["sound.clip.mute", "sound.clip.gain"] {
        let mut session = session();
        let directory = directory(&format!("different-{command}"));
        let path = format!("{directory}/sound.wav");
        a_sound(&path, 48_000);
        let (track, clip) = a_clip(&mut session, &path, 48_000);
        session
            .execute("sound.clip.separate", &json!({ "clip": clip, "at": 24_000 }))
            .expect("the clip is separated");
        let second = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.clip")
            .max_by_key(|node| node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0))
            .map(|node| node.id)
            .expect("the second piece is there");
        let params = if command == "sound.clip.mute" {
            json!({ "clip": second, "muted": true })
        } else {
            json!({ "clips": [second], "db": -6.0 })
        };
        session.execute(command, &params).expect("the piece is changed");
        let healed =
            session.execute("sound.clip.heal", &json!({ "track": track })).expect("the track is healed");
        assert_eq!(healed.data["healed"], 0, "{command} makes the clips different");
        assert_eq!(clips_of(&session).len(), 2, "and they stay apart");
    }
}

/// The **lanes** of the second piece move into the first, and their
/// breakpoints move with them.
#[test]
fn the_lanes_of_the_second_piece_move_into_the_first() {
    let mut session = session();
    let directory = directory("lanes");
    let path = format!("{directory}/sound.wav");
    a_sound(&path, 48_000);
    let (track, clip) = a_clip(&mut session, &path, 48_000);
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 24_000 }))
        .expect("the clip is separated");
    let second = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .max_by_key(|node| node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0))
        .map(|node| node.id)
        .expect("the second piece is there");
    session
        .execute(
            "sound.lane.add",
            &json!({ "clip": second, "param": "volume", "points": [[0, 0.0], [12_000, -6.0]] }),
        )
        .expect("the lane is added");
    session.execute("sound.clip.heal", &json!({ "track": track })).expect("the track is healed");
    assert_eq!(clips_of(&session), [(0, 48_000)], "the clip is one again");
    let lane = session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there");
    let points = lane.data["points"].as_array().expect("the points are there");
    assert_eq!(points[0][0], json!(24_000), "the lane moved with the sound it is over");
    assert_eq!(points[1][0], json!(36_000));
}

/// The **fade out** of the second piece is the fade out of the joined
/// clip, and the head of the first piece is kept.
#[test]
fn the_fade_out_of_the_second_piece_is_kept() {
    let mut session = session();
    let directory = directory("fade");
    let path = format!("{directory}/sound.wav");
    a_sound(&path, 48_000);
    let (track, clip) = a_clip(&mut session, &path, 48_000);
    session.execute("sound.clip.fade", &json!({ "clip": clip, "fade_out": 4_800 })).expect("the fade is set");
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 24_000 }))
        .expect("the clip is separated");
    session.execute("sound.clip.heal", &json!({ "track": track })).expect("the track is healed");
    let clip = session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
    assert_eq!(clip.data["fade_out"]["len"], json!(4_800), "the tail is still the tail");
    assert_eq!(clip.data["fade_out"]["shape"], json!("equal_power"), "and the shape is kept");
}
