//! The **gain** of a clip: how loud the clip is.
//!
//! The v2 sound battery had a gain in the clip's payload and nothing
//! that set it. Here the gain is set, it is refused outside its range,
//! and it can be bypassed without being thrown away.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-gain-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A track with one clip on it, and the id of the clip.
fn one_clip(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48000 }))
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// The gain a clip carries.
fn gain_of(session: &Session, clip: u64) -> Option<f64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("gain_db").and_then(|gain| gain.as_f64())
}

/// Whether a clip's gain is bypassed.
fn bypassed_of(session: &Session, clip: u64) -> Option<bool> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("gain_bypassed").and_then(|bypassed| bypassed.as_bool())
}

/// The gain is **set**, and it is on the clip.
#[test]
fn the_gain_is_set() {
    let mut session = session();
    let clip = one_clip(&mut session);
    assert_eq!(gain_of(&session, clip), Some(0.0), "a new clip is at unity");
    session.execute("sound.clip.gain", &json!({ "clips": [clip], "db": -6.0 })).expect("the gain is set");
    assert_eq!(gain_of(&session, clip), Some(-6.0), "the clip is six decibels down");
}

/// The gain is changed by a **delta**, which is the tree's nudge.
#[test]
fn the_gain_is_changed_by_a_delta() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.gain", &json!({ "clips": [clip], "db": -6.0 })).expect("the gain is set");
    session
        .execute("sound.clip.gain", &json!({ "clips": [clip], "delta_db": 1.0 }))
        .expect("the gain is changed");
    assert_eq!(gain_of(&session, clip), Some(-5.0), "the clip is a decibel up");
}

/// A gain outside the range is **refused**, and the clip keeps the gain
/// it had: the tree's command clamps such a gain into the range.
#[test]
fn a_gain_outside_the_range_is_refused() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.gain", &json!({ "clips": [clip], "db": -6.0 })).expect("the gain is set");
    let refused = session
        .execute("sound.clip.gain", &json!({ "clips": [clip], "db": 100.0 }))
        .expect_err("a hundred decibels is not a gain");
    assert!(refused.to_string().contains("decibels"), "the error says why: {refused}");
    assert_eq!(gain_of(&session, clip), Some(-6.0), "the clip keeps its gain");
    let refused = session
        .execute("sound.clip.gain", &json!({ "clips": [clip], "db": -1000.0 }))
        .expect_err("a thousand decibels down is not a gain");
    assert!(refused.to_string().contains("decibels"), "the error says why: {refused}");
    assert_eq!(gain_of(&session, clip), Some(-6.0), "the clip keeps its gain");
}

/// A **change** that would take the clip out of the range is refused as
/// well: the change is what was asked for, and the result is not a
/// gain.
#[test]
fn a_change_outside_the_range_is_refused() {
    let mut session = session();
    let clip = one_clip(&mut session);
    let refused = session
        .execute("sound.clip.gain", &json!({ "clips": [clip], "delta_db": -200.0 }))
        .expect_err("two hundred decibels down is not a gain");
    assert!(refused.to_string().contains("decibels"), "the error says why: {refused}");
    assert_eq!(gain_of(&session, clip), Some(0.0), "the clip keeps its gain");
}

/// A gain with **neither** a level nor a change is refused: the
/// command does not know what the gain should be.
#[test]
fn a_gain_with_nothing_to_say_is_refused() {
    let mut session = session();
    let clip = one_clip(&mut session);
    let refused = session
        .execute("sound.clip.gain", &json!({ "clips": [clip] }))
        .expect_err("a gain needs a level or a change");
    assert!(refused.to_string().contains("decibels"), "the error says why: {refused}");
}

/// The gain is **bypassed**, and it is **kept**: the tree's bypass
/// throws the gain away, which is a reset and not a bypass.
#[test]
fn the_gain_is_bypassed_and_kept() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.gain", &json!({ "clips": [clip], "db": -6.0 })).expect("the gain is set");
    session.execute("sound.clip.gain.bypass", &json!({ "clips": [clip] })).expect("the gain is bypassed");
    assert_eq!(bypassed_of(&session, clip), Some(true), "the gain is bypassed");
    assert_eq!(gain_of(&session, clip), Some(-6.0), "and the gain is kept");
    session.execute("sound.clip.gain.bypass", &json!({ "clips": [clip] })).expect("the bypass is taken off");
    assert_eq!(bypassed_of(&session, clip), Some(false), "the gain is in the path again");
    assert_eq!(gain_of(&session, clip), Some(-6.0), "and the gain is still there");
}

/// A bypass that is not named is **toggled**.
#[test]
fn a_bypass_that_is_not_named_is_toggled() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session
        .execute("sound.clip.gain.bypass", &json!({ "clips": [clip], "bypassed": true }))
        .expect("the gain is bypassed");
    session.execute("sound.clip.gain.bypass", &json!({ "clips": [clip] })).expect("the bypass is taken off");
    assert_eq!(bypassed_of(&session, clip), Some(false), "the gain is in the path");
}

/// A tone at half the scale, a clip that plays it, and where to write
/// the rendered sound.
fn a_tone(session: &mut Session, name: &str) -> (u64, String) {
    let directory = directory(name);
    let source_path = format!("{directory}/tone.wav");
    let output_path = format!("{directory}/out.wav");
    let samples: Vec<f32> = (0..48_000)
        .map(|index| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * index as f32 / 48_000.0).sin())
        .collect();
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(&source_path, bytes).expect("the sound is written");
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
    (clip, output_path)
}

/// The loudest sample of a rendered sound.
fn peak_of(path: &str) -> f32 {
    let sound = craftkit_wav::read(
        &std::fs::read(path).expect("the sound is on the disk"),
        &craftkit_wav::Limits::default(),
    )
    .expect("the sound is read");
    sound.samples.iter().fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()))
}

/// The gain is **heard**: a clip six decibels down is quieter, and a
/// bypassed clip is not.
#[test]
fn the_gain_is_heard() {
    let mut session = session();
    let (clip, output_path) = a_tone(&mut session, "heard");
    session.execute("sound.clip.gain", &json!({ "clips": [clip], "db": -6.0 })).expect("the gain is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let quiet = peak_of(&output_path);
    assert!(quiet > 0.2 && quiet < 0.3, "the clip is six decibels down: {quiet}");
    // And with the bypass on, the clip is as loud as it is.
    session.execute("sound.clip.gain.bypass", &json!({ "clips": [clip] })).expect("the gain is bypassed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let loud = peak_of(&output_path);
    assert!(loud > 0.45, "the clip is as loud as it is: {loud}");
}

/// The bypass takes the clip's **curve** out of the path as well: the
/// tree's bypass clears the curve, and here the curve is kept and is
/// out of the path while the bypass is on.
#[test]
fn a_bypassed_gain_takes_the_curve_out_of_the_path_too() {
    let mut session = session();
    let (clip, output_path) = a_tone(&mut session, "curve");
    // A lane under the clip that automates its level, twelve decibels
    // down.
    session
        .execute("sound.lane.add", &json!({ "clip": clip, "param": "volume", "points": [[0, -12.0]] }))
        .expect("the lane is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let quiet = peak_of(&output_path);
    assert!(quiet > 0.1 && quiet < 0.15, "the curve is on: {quiet}");
    // With the gain bypassed, the clip is as loud as it is.
    session.execute("sound.clip.gain.bypass", &json!({ "clips": [clip] })).expect("the gain is bypassed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let loud = peak_of(&output_path);
    assert!(loud > 0.45, "the clip is as loud as it is: {loud}");
    // And taking the bypass off brings the curve back.
    session.execute("sound.clip.gain.bypass", &json!({ "clips": [clip] })).expect("the bypass is taken off");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let quiet = peak_of(&output_path);
    assert!(quiet > 0.1 && quiet < 0.15, "the curve is back: {quiet}");
}

/// Only a **clip** has a gain.
#[test]
fn only_a_clip_has_a_gain() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let refused = session
        .execute("sound.clip.gain", &json!({ "clips": [track], "db": -6.0 }))
        .expect_err("a track is not a clip");
    assert!(refused.to_string().contains("not a clip"), "the error says why: {refused}");
}
