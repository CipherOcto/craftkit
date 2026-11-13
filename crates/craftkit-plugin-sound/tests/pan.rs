//! The **pan** of a track: the balance of its sound.
//!
//! The tree's mixer keeps a pan on every track, and a stereo track
//! has one panner for each of its channels. This tree had a pan
//! **lane** and no pan of its own, and the lane's law was the tree's
//! constant-power one — which is a law for placing a mono sound in a
//! stereo field, and takes three decibels off a stereo sound in the
//! middle.
//!
//! The pan here is a **balance**: the middle is the middle, and the
//! sound is not quieter there than it is at the side.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-pan-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stereo** source: half on the left and a quarter on the right,
/// so that a balance is visible.
fn a_source(path: &str) {
    let bytes = craftkit_wav::write(2, 48_000, &[0.5, 0.25, 0.5, 0.25]);
    std::fs::write(path, bytes).expect("the source is written");
}

/// A session with a track called `drums` and the source on it.
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    track
}

/// The rendered sound, and the two sides of it.
fn rendered(path: &str) -> (f32, f32) {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    (sound.samples[0], sound.samples[1])
}

/// The pan a track carries.
fn pan_of(session: &Session, track: u64) -> Option<f64> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("pan").and_then(|pan| pan.as_f64())
}

/// A track carries a **pan**, and it is in the middle.
#[test]
fn a_track_carries_its_pan() {
    let mut session = session();
    let directory = directory("carries");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let track = a_clip(&mut session, &source_path);
    assert_eq!(pan_of(&session, track), Some(0.0), "the pan is in the middle");
}

/// A pan is **set** on a track.
#[test]
fn a_pan_is_set_on_the_track() {
    let mut session = session();
    let directory = directory("set");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let track = a_clip(&mut session, &source_path);
    session.execute("sound.track.pan", &json!({ "track": track, "pan": -0.5 })).expect("the pan is set");
    assert_eq!(pan_of(&session, track), Some(-0.5), "the pan is set");
}

/// A pan that is **out of range** is refused: the tree's pan runs from
/// minus one to one, and a pan outside that is refused and not pulled
/// to the edge.
#[test]
fn a_pan_that_is_out_of_range_is_refused() {
    let mut session = session();
    let directory = directory("out-of-range");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let track = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.pan", &json!({ "track": track, "pan": 2.0 }));
    assert!(refused.is_err(), "a pan is bounded");
    assert_eq!(pan_of(&session, track), Some(0.0), "and the pan did not move");
}

/// A pan that is **not a number** is refused.
#[test]
fn a_pan_that_is_not_a_number_is_refused() {
    let mut session = session();
    let directory = directory("not-a-number");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let track = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.pan", &json!({ "track": track, "pan": "left" }));
    assert!(refused.is_err(), "a pan is a number");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let track = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.track.pan", &json!({ "track": track + 1_000_000, "pan": -0.5 }));
    assert!(refused.is_err(), "there is no such track");
}

/// The pan is **heard**: a track panned to the left is on the left.
#[test]
fn the_pan_is_heard() {
    let directory = directory("heard");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);

    let mut session = session();
    let track = a_clip(&mut session, &source_path);
    session.execute("sound.track.pan", &json!({ "track": track, "pan": -1.0 })).expect("the pan is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let (left, right) = rendered(&output_path);
    assert!((left - 0.5).abs() < 1e-4, "the left is the source's: {left}");
    assert!(right.abs() < 1e-4, "and the right is silent: {right}");
}

/// A pan in the **middle** is the middle: the sound is not quieter
/// there than it is at the side, which is what the tree's balance law
/// says and what its constant-power law does not.
#[test]
fn a_pan_in_the_middle_is_the_middle() {
    let directory = directory("middle");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);

    let mut session = session();
    a_clip(&mut session, &source_path);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let (left, right) = rendered(&output_path);
    assert!((left - 0.5).abs() < 1e-4, "the left is the source's: {left}");
    assert!((right - 0.25).abs() < 1e-4, "and the right is the source's: {right}");
}

/// A **pan lane** overrides the pan of the track, the way a volume
/// lane overrides the fader.
#[test]
fn a_pan_lane_overrides_the_pan_of_the_track() {
    let directory = directory("lane");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);

    let mut session = session();
    let track = a_clip(&mut session, &source_path);
    session.execute("sound.track.pan", &json!({ "track": track, "pan": 1.0 })).expect("the pan is set");
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "pan", "points": [[0, -1.0]] }))
        .expect("the lane is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let (left, right) = rendered(&output_path);
    assert!((left - 0.5).abs() < 1e-4, "the lane says left: {left}");
    assert!(right.abs() < 1e-4, "and the right is silent: {right}");
}

/// A track that **feeds a bus** is balanced before it reaches the
/// bus: the tree's pan is applied when the track is routed.
#[test]
fn a_routed_track_is_balanced_before_it_reaches_the_bus() {
    let directory = directory("routed");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);

    let mut session = session();
    let track = a_clip(&mut session, &source_path);
    session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
    let bus = session.document().node_by_name("fx").expect("the bus is there").id;
    session.execute("sound.route.add", &json!({ "track": track, "bus": bus })).expect("the track is routed");
    session.execute("sound.track.pan", &json!({ "track": track, "pan": -1.0 })).expect("the pan is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let (left, right) = rendered(&output_path);
    assert!((left - 0.5).abs() < 1e-4, "the left is the source's: {left}");
    assert!(right.abs() < 1e-4, "and the right is silent: {right}");
}

/// The pan is **not linked** by the group: the tree's group shares
/// the volume, the mute, the solo and the record — and not the pan.
#[test]
fn the_pan_is_not_linked_by_the_group() {
    let mut session = session();
    let directory = directory("group");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    session.execute("sound.track.add", &json!({ "name": "kick" })).expect("the track is added");
    session.execute("sound.track.add", &json!({ "name": "snare" })).expect("the track is added");
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    let snare = session.document().node_by_name("snare").expect("the track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "drums", "tracks": [kick, snare] }))
        .expect("the tracks are grouped");
    session.execute("sound.track.pan", &json!({ "track": kick, "pan": -1.0 })).expect("the pan is set");
    assert_eq!(pan_of(&session, snare), Some(0.0), "the pan is the track's own");
}
