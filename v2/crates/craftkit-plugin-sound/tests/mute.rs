//! The **mute** of a track: the level of the whole track.
//!
//! The tree's mixer keeps a mute of its own beside the mute lane, and
//! here the lane is the mute's default: a lane with breakpoints
//! overrides it, and a lane without one is the mute. The mute is also
//! one of the things the tree's group shares.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-mute-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of that level, one frame long.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and a sound on it.
fn a_clip(session: &mut Session, source_path: &str) -> u64 {
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
        .expect("the clip is added");
    track
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The mute a track carries.
fn mute_of(session: &Session, track: u64) -> Option<bool> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("mute").and_then(|mute| mute.as_bool())
}

/// A track carries a **mute**, and it is off.
#[test]
fn a_track_carries_its_mute() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 0.5);
    let track = a_clip(&mut session, &source_path);
    assert_eq!(mute_of(&session, track), Some(false), "the track is not muted");
}

/// A mute is **set** on a track, and taken off again.
#[test]
fn a_mute_is_set_on_the_track() {
    let mut session = session();
    let directory = directory("set");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 0.5);
    let track = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.mute", &json!({ "track": track, "muted": true }))
        .expect("the track is muted");
    assert_eq!(mute_of(&session, track), Some(true), "the track is muted");
    session
        .execute("sound.track.mute", &json!({ "track": track, "muted": false }))
        .expect("the track is unmuted");
    assert_eq!(mute_of(&session, track), Some(false), "the track is not muted");
}

/// A muted track is **not heard**.
#[test]
fn a_muted_track_is_not_heard() {
    let directory = directory("heard");
    let source_path = format!("{directory}/sound.wav");
    let loud_path = format!("{directory}/loud.wav");
    let quiet_path = format!("{directory}/quiet.wav");
    a_sound(&source_path, 0.5);

    let mut session = session();
    let track = a_clip(&mut session, &source_path);
    session.execute("sound.render", &json!({ "path": loud_path })).expect("the sound is rendered");
    session
        .execute("sound.track.mute", &json!({ "track": track, "muted": true }))
        .expect("the track is muted");
    session.execute("sound.render", &json!({ "path": quiet_path })).expect("the sound is rendered");

    let loud = rendered(&loud_path);
    let quiet = rendered(&quiet_path);
    assert!((loud.samples[0] - 0.5).abs() < 1e-3, "the track was heard: {}", loud.samples[0]);
    assert!(quiet.samples[0].abs() < 1e-6, "and now it is not: {}", quiet.samples[0]);
}

/// A **mute lane** overrides the mute of the track: the lane is the
/// mute's default, and a lane with breakpoints says else.
#[test]
fn a_mute_lane_overrides_the_mute_of_the_track() {
    let directory = directory("lane");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    a_sound(&source_path, 0.5);

    let mut session = session();
    let track = a_clip(&mut session, &source_path);
    session
        .execute("sound.track.mute", &json!({ "track": track, "muted": true }))
        .expect("the track is muted");
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "mute", "points": [[0, 0.0]] }))
        .expect("the lane is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-3, "the lane says not muted: {}", sound.samples[0]);
}

/// The mute is **linked** by the group: the tree's group shares the
/// volume, the mute, the solo and the record.
#[test]
fn the_mute_is_linked_by_the_group() {
    let mut session = session();
    let directory = directory("group");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 0.5);
    session.execute("sound.track.add", &json!({ "name": "kick" })).expect("the track is added");
    session.execute("sound.track.add", &json!({ "name": "snare" })).expect("the track is added");
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    let snare = session.document().node_by_name("snare").expect("the track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": [kick, snare] }))
        .expect("the tracks are grouped");
    session
        .execute("sound.track.mute", &json!({ "track": kick, "muted": true }))
        .expect("the track is muted");
    assert_eq!(mute_of(&session, snare), Some(true), "the group is muted with it");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 0.5);
    let track = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.mute", &json!({ "track": track + 1_000_000, "muted": true }));
    assert!(refused.is_err(), "there is no such track");
}
