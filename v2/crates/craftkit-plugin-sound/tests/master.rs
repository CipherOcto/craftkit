//! The **master**: a track whose fader is over the whole mix.
//!
//! The tree's mixer runs the master faders over the main mix, and the
//! metronome is summed after them. Here the master was a track like any
//! other, and its fader moved nothing but its own (empty) signal.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-master-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source: quiet enough that the track and the
/// bus together stay inside the file.
const QUIET: f32 = 0.4;

/// A session with a **track** that plays a quiet source.
fn a_track(session: &mut Session, path: &str) -> u64 {
    session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    track
}

/// A **master** track, with a fader six decibels down.
fn a_master(session: &mut Session, name: &str) -> u64 {
    session
        .execute("sound.track.add", &json!({ "name": name, "track_kind": "master" }))
        .expect("the master is added");
    let master = session.document().node_by_name(name).expect("the master is there").id;
    session
        .execute("sound.lane.add", &json!({ "track": master, "param": "volume", "points": [[0, -6.0]] }))
        .expect("the fader is there");
    master
}

/// The first sample of what was rendered.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples[0]
}

/// A **master fader** moves the whole mix: the track is heard at half.
#[test]
fn a_master_fader_moves_the_whole_mix() {
    let directory = directory("mix");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    a_track(&mut session, &source_path);
    a_master(&mut session, "Master");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    let expected = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the master moves the mix: {sample}");
}

/// The **master fader** comes after the buses: the copy on the bus is
/// moved with the rest of the mix.
#[test]
fn the_master_fader_is_after_the_buses() {
    let directory = directory("buses");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let track = a_track(&mut session, &source_path);
    session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
    let bus = session.document().node_by_name("fx").expect("the bus is there").id;
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0 }))
        .expect("the send is added");
    a_master(&mut session, "Master");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track and the copy are both heard, and the master moves both.
    let expected = (QUIET + QUIET) * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the master is after the buses: {sample}");
}

/// Two **masters** are a stack: each one moves what the one before it
/// left, as the tree's mixer runs them in the order of the tree.
#[test]
fn two_masters_are_a_stack() {
    let directory = directory("stack");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    a_track(&mut session, &source_path);
    a_master(&mut session, "Master");
    a_master(&mut session, "Master 2");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    let gain = 10.0_f32.powf(-6.0 / 20.0);
    let expected = QUIET * gain * gain;
    assert!((sample - expected).abs() < 0.01, "the masters are a stack: {sample}");
}

/// A **master** holds no clips: a clip on a master would be a clip that
/// is never heard, and it is refused by name.
#[test]
fn a_master_holds_no_clips() {
    let directory = directory("clip");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    a_track(&mut session, &source_path);
    let master = a_master(&mut session, "Master");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    let refused = session
        .execute("sound.clip.add", &json!({ "track": master, "start": 0, "length": 2, "source": source }));
    let message = match refused {
        Ok(_) => panic!("a clip on a master is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("a master holds no clips"), "the refusal says why: {message}");
}
