//! The **trim**: the level of a track's input, and its phase.
//!
//! The tree's mixer applies the trim to the strip before its inserts,
//! so an insert sees the trimmed sound, and the phase is a sign and
//! not a level: the sound is the same and upside down.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-trim-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source.
const QUIET: f32 = 0.25;

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

/// The first sample of what was rendered.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples[0]
}

/// A **trim** is the level of the track's input: the sound is heard at
/// the trim's level.
#[test]
fn a_trim_is_the_level_of_the_input() {
    let directory = directory("trim");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let track = a_track(&mut session, &source_path);
    session.execute("sound.track.set", &json!({ "track": track, "trim_db": -6.0 })).expect("the trim is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    let expected = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the trim is the level of the input: {sample}");
}

/// The **phase** is a sign and not a level: the sound is the same and
/// upside down.
#[test]
fn a_phase_is_a_sign_and_not_a_level() {
    let directory = directory("phase");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let track = a_track(&mut session, &source_path);
    session.execute("sound.track.set", &json!({ "track": track, "phase": true })).expect("the phase is set");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!((sample + QUIET).abs() < 0.01, "the sound is upside down: {sample}");
}

/// A trim that is **not a trim** is refused: the tree's trim runs from
/// minus one hundred and forty-four decibels to twenty-four.
#[test]
fn a_trim_that_is_not_a_trim_is_refused() {
    let directory = directory("bad");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let track = a_track(&mut session, &source_path);
    let refused = session.execute("sound.track.set", &json!({ "track": track, "trim_db": 40.0 }));
    assert!(refused.is_err(), "a trim of forty decibels is refused");
}
