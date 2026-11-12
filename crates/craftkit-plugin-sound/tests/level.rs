//! The **level** of a track: the trim and the fader.
//!
//! The tree's mixer has two levels on a track and they are not the
//! same thing: the **trim** is the gain of the input, before the
//! inserts, and the **fader** is the level of the output, after them.
//! This tree had the trim and no fader, so a track's level could only
//! be automated and never set.
//!
//! The fader is the **default** of the volume lane, as it is in the
//! tree: a lane with breakpoints overrides it, and a lane without one
//! is the fader.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-level-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **tone** of that frequency, a second long.
fn a_tone(path: &str, frequency: f32) {
    let frames = 48_000;
    let samples: Vec<f32> = (0..frames)
        .map(|frame| {
            let t = frame as f64 / 48_000.0;
            (2.0 * std::f64::consts::PI * f64::from(frequency) * t).sin() as f32
        })
        .collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and a tone on it.
fn a_clip(session: &mut Session, source_path: &str) -> (u64, u64) {
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The loudest sample of the sound.
fn loudest(sound: &craftkit_wav::Sound) -> f32 {
    sound.samples.iter().fold(0.0_f32, |most, sample| most.max(sample.abs()))
}

/// The fader a track carries.
fn fader_of(session: &Session, track: u64) -> Option<f64> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("fader_db").and_then(|fader| fader.as_f64())
}

/// A track carries a **fader**, and it is at zero.
#[test]
fn a_track_carries_a_fader() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    assert_eq!(fader_of(&session, track), Some(0.0), "the fader is at zero");
}

/// A fader is **set** on a track.
#[test]
fn a_fader_is_set_on_the_track() {
    let mut session = session();
    let directory = directory("set");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    session.execute("sound.track.fader", &json!({ "track": track, "db": -6.0 })).expect("the fader is set");
    assert_eq!(fader_of(&session, track), Some(-6.0), "the fader is set");
}

/// A fader that is **too low** is refused: the tree's fader runs from
/// minus one hundred and forty-four decibels to twelve, and a fader
/// outside that is refused and not pulled to the edge.
#[test]
fn a_fader_that_is_too_low_is_refused() {
    let mut session = session();
    let directory = directory("too-low");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.fader", &json!({ "track": track, "db": -200.0 }));
    assert!(refused.is_err(), "a fader is bounded");
    assert_eq!(fader_of(&session, track), Some(0.0), "and the fader did not move");
}

/// A fader that is **too high** is refused, for the same reason and at
/// the other end.
#[test]
fn a_fader_that_is_too_high_is_refused() {
    let mut session = session();
    let directory = directory("too-high");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.fader", &json!({ "track": track, "db": 20.0 }));
    assert!(refused.is_err(), "a fader is bounded");
    assert_eq!(fader_of(&session, track), Some(0.0), "and the fader did not move");
}

/// A fader that is **not a number** is refused.
#[test]
fn a_fader_that_is_not_a_number_is_refused() {
    let mut session = session();
    let directory = directory("not-a-number");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (track, _) = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.fader", &json!({ "track": track, "db": "quiet" }));
    assert!(refused.is_err(), "a fader is a number");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    let (_, clip) = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.fader", &json!({ "track": clip, "db": -6.0 }));
    assert!(refused.is_err(), "a clip is not a track");
}

/// The fader is **heard**: a track with a fader below zero is quieter
/// than the same track without one.
#[test]
fn the_fader_is_heard() {
    let directory = directory("heard");
    let source_path = format!("{directory}/tone.wav");
    let plain_path = format!("{directory}/plain.wav");
    let quiet_path = format!("{directory}/quiet.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let (track, _) = a_clip(&mut session, &source_path);
    session.execute("sound.render", &json!({ "path": plain_path })).expect("the sound is rendered");
    session.execute("sound.track.fader", &json!({ "track": track, "db": -6.0 })).expect("the fader is set");
    session.execute("sound.render", &json!({ "path": quiet_path })).expect("the sound is rendered");

    let plain = rendered(&plain_path);
    let quiet = rendered(&quiet_path);
    let before = loudest(&plain);
    let after = loudest(&quiet);
    // Six decibels is about half the loudness.
    assert!(after < before * 0.55 && after > before * 0.45, "quieter: {before} and {after}");
}

/// A **volume lane** overrides the fader: the tree's mixer runs the
/// lane when there is one, and the fader when there is not.
#[test]
fn a_volume_lane_overrides_the_fader() {
    let directory = directory("lane");
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    a_tone(&source_path, 440.0);

    let mut session = session();
    let (track, _) = a_clip(&mut session, &source_path);
    session.execute("sound.track.fader", &json!({ "track": track, "db": -12.0 })).expect("the fader is set");
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "volume", "points": [[0, 0.0]] }))
        .expect("the lane is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    let loud = loudest(&sound);
    assert!(loud > 0.9, "the lane says zero decibels and the fader is not heard: {loud}");
}

/// The **trim** is not linked by the group: the tree's group links the
/// volume, and the volume is the fader.
#[test]
fn the_trim_is_not_linked_by_the_group() {
    let mut session = session();
    let directory = directory("trim");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    session.execute("sound.track.add", &json!({ "name": "kick" })).expect("the track is added");
    session.execute("sound.track.add", &json!({ "name": "snare" })).expect("the track is added");
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    let snare = session.document().node_by_name("snare").expect("the track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": [kick, snare] }))
        .expect("the tracks are grouped");
    session.execute("sound.track.set", &json!({ "track": kick, "trim_db": -6.0 })).expect("the trim is set");
    let other = session.document().node(snare).expect("the track is there").data.clone();
    assert_eq!(other.get("trim_db").and_then(|trim| trim.as_f64()), Some(0.0), "the trim is the track's own");
}

/// The fader is **linked** by the group: the tree's group links the
/// volume, and the volume is the fader.
#[test]
fn the_fader_is_linked_by_the_group() {
    let mut session = session();
    let directory = directory("linked");
    let source_path = format!("{directory}/tone.wav");
    a_tone(&source_path, 440.0);
    session.execute("sound.track.add", &json!({ "name": "kick" })).expect("the track is added");
    session.execute("sound.track.add", &json!({ "name": "snare" })).expect("the track is added");
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    let snare = session.document().node_by_name("snare").expect("the track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": [kick, snare] }))
        .expect("the tracks are grouped");
    session.execute("sound.track.fader", &json!({ "track": kick, "db": -6.0 })).expect("the fader is set");
    assert_eq!(fader_of(&session, snare), Some(-6.0), "the group is set with it");
}
