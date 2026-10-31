//! The **send**: a copy of the track's signal, panned and muted.
//!
//! A send carries a copy of its track's signal to a bus, and the copy
//! has a level and a pan of its own — the tree's send has both, and a
//! mute as well, and the command could set neither the pan nor the
//! mute.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-send-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stereo** source of that level on both sides.
fn a_source(path: &str) {
    let bytes = craftkit_wav::write(2, 48_000, &[0.25, 0.25, 0.25, 0.25]);
    std::fs::write(path, bytes).expect("the source is written");
}

/// A session with a **stereo track** and a **bus**, and the send
/// between them.
fn a_routed(session: &mut Session, path: &str) -> (u64, u64) {
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
    session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
    let bus = session.document().node_by_name("fx").expect("the bus is there").id;
    (track, bus)
}

/// The sound that was rendered, and the two sides of it.
fn rendered(path: &str) -> (f32, f32) {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    (sound.samples[0], sound.samples[1])
}

/// A **panned** send is heard on its side: the copy is on the left and
/// the track is in the middle, so the left is louder than the right.
#[test]
fn a_panned_send_is_heard_on_its_side() {
    let directory = directory("pan");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);
    let mut session = session();
    let (track, bus) = a_routed(&mut session, &source_path);
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0, "pan": -1.0 }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let (left, right) = rendered(&output_path);
    assert!((left - 0.5).abs() < 1e-3, "the track and the copy are both on the left: {left}");
    assert!((right - 0.25).abs() < 1e-3, "and the copy is not on the right: {right}");
}

/// A **muted** send is a copy of nothing: the track is still heard and
/// the bus hears nothing.
#[test]
fn a_muted_send_is_a_copy_of_nothing() {
    let directory = directory("mute");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);
    let mut session = session();
    let (track, bus) = a_routed(&mut session, &source_path);
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0, "mute": true }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let (left, right) = rendered(&output_path);
    assert!((left - 0.25).abs() < 1e-3, "the track is still heard: {left}");
    assert!((right - 0.25).abs() < 1e-3, "and the bus hears nothing: {right}");
}

/// A pan that is **not a pan** is refused.
#[test]
fn a_pan_that_is_not_a_pan_is_refused() {
    let directory = directory("bad-pan");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let mut session = session();
    let (track, bus) = a_routed(&mut session, &source_path);
    let error = session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "pan": 2.0 }))
        .expect_err("a pan is between minus one and one");
    assert!(matches!(error, craftkit_sdk::EngineError::Failed { .. }), "{error:?}");
}
