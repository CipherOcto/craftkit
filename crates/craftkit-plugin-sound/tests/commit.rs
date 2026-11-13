//! The **commit** of a track: its sound written to a file, and a new
//! track of that file.
//!
//! The tree's commit renders the sound of a track before its fader,
//! writes it to a file, makes a new track of that file, and turns the
//! voice of the old track off.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-commit-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of four frames, all of them at that level.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level, level, level, level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and a sound of four frames
/// on it.
fn a_track(session: &mut Session, directory: &str) -> u64 {
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
    track
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// Whether a track's **voice is off**.
fn inactive_of(session: &Session, track: u64) -> Option<bool> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("inactive").and_then(|inactive| inactive.as_bool())
}

/// The **tracks** of the document, by name.
fn track_names(session: &Session) -> Vec<String> {
    let mut names: Vec<String> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.track")
        .map(|node| node.name.clone())
        .collect();
    names.sort();
    names
}

/// A commit writes a **file** and makes a **new track** of it.
#[test]
fn a_commit_makes_a_new_track_of_a_file() {
    let mut session = session();
    let directory = directory("new-track");
    let track = a_track(&mut session, &directory);
    let out = format!("{directory}/out");
    let outcome = session
        .execute("sound.track.commit", &json!({ "track": track, "directory": out }))
        .expect("the track is committed");
    let path = outcome.data["path"].as_str().expect("the path is there").to_string();
    assert!(std::path::Path::new(&path).exists(), "the file is written: {path}");
    assert_eq!(track_names(&session), vec!["drums".to_string(), "drums.cm".to_string()]);
    // The new track has a **clip**, and the clip plays the file.
    let committed = outcome.data["committed"].as_u64().expect("the new track is there");
    let clip = outcome.data["clip"].as_u64().expect("the clip is there");
    let source = outcome.data["source"].as_u64().expect("the source is there");
    let node = session.document().node(clip).expect("the clip is there");
    assert_eq!(node.kind, "sound.clip", "it is a clip");
    assert_eq!(
        node.data.get("source").and_then(|source| source.as_u64()),
        Some(source),
        "and it plays the file"
    );
    assert!(session.document().node(committed).is_some(), "the new track is there");
}

/// The old track is **not gone**: it is in the document and its voice
/// is off.
#[test]
fn the_old_track_is_quiet_and_it_is_still_there() {
    let mut session = session();
    let directory = directory("quiet");
    let track = a_track(&mut session, &directory);
    let out = format!("{directory}/out");
    session
        .execute("sound.track.commit", &json!({ "track": track, "directory": out }))
        .expect("the track is committed");
    assert_eq!(inactive_of(&session, track), Some(true), "the old track is quiet");
    let clips = session.document().walk().filter(|node| node.kind == "sound.clip").count();
    assert_eq!(clips, 2, "the old clip is still there and the new one is there too");
}

/// The sound of the document is the **same** after the commit: the
/// committed file is the track before its fader, and the new track
/// carries the fader.
#[test]
fn the_sound_is_the_same_after_the_commit() {
    let mut session = session();
    let directory = directory("same");
    let track = a_track(&mut session, &directory);
    let before_path = format!("{directory}/before.wav");
    session.execute("sound.render", &json!({ "path": before_path })).expect("the sound is rendered");
    let out = format!("{directory}/out");
    session
        .execute("sound.track.commit", &json!({ "track": track, "directory": out }))
        .expect("the track is committed");
    let after_path = format!("{directory}/after.wav");
    session.execute("sound.render", &json!({ "path": after_path })).expect("the sound is rendered");

    let before = rendered(&before_path);
    let after = rendered(&after_path);
    assert!((before.samples[0] - 0.5).abs() < 1e-4, "the track was heard: {}", before.samples[0]);
    assert!((after.samples[0] - 0.5).abs() < 1e-4, "and it is heard after: {}", after.samples[0]);
}

/// The **fader** is not counted twice: the new track carries the old
/// track's fader, and the level is the level.
#[test]
fn the_fader_is_not_counted_twice() {
    let mut session = session();
    let directory = directory("fader");
    let track = a_track(&mut session, &directory);
    session.execute("sound.track.fader", &json!({ "track": track, "db": -6.0 })).expect("the fader is down");
    let before_path = format!("{directory}/before.wav");
    session.execute("sound.render", &json!({ "path": before_path })).expect("the sound is rendered");
    let out = format!("{directory}/out");
    session
        .execute("sound.track.commit", &json!({ "track": track, "directory": out }))
        .expect("the track is committed");
    let after_path = format!("{directory}/after.wav");
    session.execute("sound.render", &json!({ "path": after_path })).expect("the sound is rendered");

    let before = rendered(&before_path);
    let after = rendered(&after_path);
    assert!((before.samples[0] - 0.25).abs() < 1e-3, "the fader was down: {}", before.samples[0]);
    assert!((after.samples[0] - 0.25).abs() < 1e-3, "and it is down after: {}", after.samples[0]);
}

/// The committed track takes the **place** of the old one: the mix is
/// in the same order as it was.
#[test]
fn the_committed_track_takes_the_place_of_the_old_one() {
    let mut session = session();
    let directory = directory("place");
    let track = a_track(&mut session, &directory);
    let out = format!("{directory}/out");
    session
        .execute("sound.track.commit", &json!({ "track": track, "directory": out }))
        .expect("the track is committed");
    let names: Vec<String> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.track")
        .map(|node| node.name.clone())
        .collect();
    assert_eq!(
        names,
        vec!["drums".to_string(), "drums.cm".to_string()],
        "the new track is after the old one"
    );
}

/// A track with **no sound** is refused: there is nothing to write
/// down.
#[test]
fn a_track_with_no_sound_is_refused() {
    let mut session = session();
    let directory = directory("no-sound");
    session.execute("sound.track.add", &json!({ "name": "empty" })).expect("the track is added");
    let track = session.document().node_by_name("empty").expect("the track is there").id;
    let out = format!("{directory}/out");
    let error = session
        .execute("sound.track.commit", &json!({ "track": track, "directory": out }))
        .expect_err("the commit is refused");
    assert!(error.to_string().contains("no sound"), "the error says why: {error}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let track = a_track(&mut session, &directory);
    let out = format!("{directory}/out");
    let error = session
        .execute("sound.track.commit", &json!({ "track": track + 1_000_000, "directory": out }))
        .expect_err("the commit is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
