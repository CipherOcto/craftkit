//! The **fader** comes after the inserts, and a send may be taken
//! before it.
//!
//! The tree's mixer runs the track's inserts and then its fader, and a
//! send may be a copy of the signal from before the fader. Here the
//! fader came before the inserts and a send was always taken after it.
//!
//! The sound is **quiet** on purpose: the master is a file and a file
//! cannot hold more than one, so a sum over one would be clipped and
//! the two copies would not be told apart.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-fader-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source: quiet enough that the track and the
/// bus together stay inside the file.
const QUIET: f32 = 0.4;

/// A track with a **fader** six decibels down, a bus, and the source
/// the track plays.
fn a_faded_track(session: &mut Session, path: &str) -> (u64, u64) {
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
    // The **fader**: the track's volume, six decibels down.
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "volume", "points": [[0, -6.0]] }))
        .expect("the fader is there");
    session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
    let bus = session.document().node_by_name("fx").expect("the bus is there").id;
    (track, bus)
}

/// The first sample of what was rendered.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples[0]
}

/// A **pre-fader** send is a copy of the signal from before the
/// fader: the track is quiet and the bus hears the sound as it was.
#[test]
fn a_pre_fader_send_is_heard_before_the_fader() {
    let directory = directory("pre");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, bus) = a_faded_track(&mut session, &source_path);
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0, "pre_fader": true }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track is at half and the copy is the whole sound: the two
    // together are one and a half times the quiet source.
    let half = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    let expected = QUIET + half;
    assert!((sample - expected).abs() < 0.01, "the copy is from before the fader: {sample}");
}

/// A **muted** track is not heard anywhere: the mute gates the audio
/// and the track's pre-fader tap, so the bus hears nothing either.
#[test]
fn a_muted_track_is_not_heard_by_its_pre_fader_send() {
    let directory = directory("mute");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, bus) = a_faded_track(&mut session, &source_path);
    // The **mute**: the track is silenced from the start.
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "mute", "points": [[0, 1.0]] }))
        .expect("the mute is there");
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0, "pre_fader": true }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!(sample.abs() < 0.01, "a muted track is not heard anywhere: {sample}");
}

/// An **insert** is run before the fader, so a pre-fader copy carries
/// the insert and not the fader: the insert is heard, the fader is not.
#[test]
fn an_insert_is_heard_by_a_pre_fader_send() {
    let directory = directory("insert");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, bus) = a_faded_track(&mut session, &source_path);
    // An **insert**, six decibels down.
    session
        .execute("sound.insert.add", &json!({ "track": track, "plugin": "gain" }))
        .expect("the insert is added");
    let insert =
        session.document().walk().find(|node| node.kind == "sound.insert").expect("the insert is there");
    session
        .execute("sound.insert.set", &json!({ "insert": insert.id, "parameters": { "gain_db": -6.0 } }))
        .expect("the insert is set");
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0, "pre_fader": true }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The insert is at half and the fader is at half: the track is a
    // quarter and the copy is a half.
    let gain = 10.0_f32.powf(-6.0 / 20.0);
    let expected = QUIET * gain * gain + QUIET * gain;
    assert!((sample - expected).abs() < 0.01, "the copy carries the insert: {sample}");
}

/// A **post-fader** send is a copy of the signal from after the fader:
/// the track is quiet and the bus hears it quiet too.
#[test]
fn a_post_fader_send_is_heard_after_the_fader() {
    let directory = directory("post");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, bus) = a_faded_track(&mut session, &source_path);
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0 }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track is at half and the copy is at half: the two together
    // are the quiet source again.
    let half = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    let expected = half + half;
    assert!((sample - expected).abs() < 0.01, "the copy is from after the fader: {sample}");
}
