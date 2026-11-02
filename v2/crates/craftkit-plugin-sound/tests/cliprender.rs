//! The **render** of a clip: the sound of a clip, written to a file.
//!
//! The tree's editor exports the clips as files. This tree's battery
//! could render the mix and the stems, and not one clip of them.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-cliprender-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of four frames, all of them at that level.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level, level, level, level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track and a clip of four frames on it.
fn a_clip(session: &mut Session, directory: &str) -> u64 {
    let path = format!("{directory}/sound.wav");
    a_sound(&path, 0.5);
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
        .expect("the clip is added");
    session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// A clip is **rendered** to a file, and the file holds its sound.
#[test]
fn a_clip_is_rendered_to_a_file() {
    let mut session = session();
    let directory = directory("rendered");
    let clip = a_clip(&mut session, &directory);
    let out = format!("{directory}/out");
    let outcome = session
        .execute("sound.clip.render", &json!({ "clip": clip, "directory": out }))
        .expect("the clip is rendered");
    let path = outcome.data["path"].as_str().expect("the path is there").to_string();
    assert!(std::path::Path::new(&path).exists(), "the file is written: {path}");
    let sound = rendered(&path);
    assert_eq!(sound.frames, 4, "the file is as long as the clip: {}", sound.frames);
    assert!((sound.samples[0] - 0.5).abs() < 1e-3, "and it holds the sound: {}", sound.samples[0]);
}

/// The file is named after the **clip**.
#[test]
fn the_file_is_named_after_the_clip() {
    let mut session = session();
    let directory = directory("named");
    let clip = a_clip(&mut session, &directory);
    session
        .execute("sound.clip.rename", &json!({ "clip": clip, "name": "kick" }))
        .expect("the clip is renamed");
    let out = format!("{directory}/out");
    let outcome = session
        .execute("sound.clip.render", &json!({ "clip": clip, "directory": out }))
        .expect("the clip is rendered");
    let path = outcome.data["path"].as_str().expect("the path is there").to_string();
    assert!(path.ends_with("kick.wav"), "the file is named after the clip: {path}");
}

/// A **MIDI clip** is refused: it holds notes and not a sound.
#[test]
fn a_midi_clip_is_refused() {
    let mut session = session();
    let directory = directory("midi");
    session.execute("sound.track.add", &json!({ "name": "keys" })).expect("the track is added");
    let track = session.document().node_by_name("keys").expect("the track is there").id;
    session
        .execute("sound.clip.midi.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    let out = format!("{directory}/out");
    let error = session
        .execute("sound.clip.render", &json!({ "clip": clip, "directory": out }))
        .expect_err("the render is refused");
    assert!(error.to_string().contains("no sound"), "the error says why: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let directory = directory("not-a-clip");
    let clip = a_clip(&mut session, &directory);
    let out = format!("{directory}/out");
    let error = session
        .execute("sound.clip.render", &json!({ "clip": clip + 1_000_000, "directory": out }))
        .expect_err("the render is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
