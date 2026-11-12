//! The **voice** of a track: whether it is heard at all.
//!
//! The tree's track header says "dyn" or "off", and "off" makes the
//! track **inactive**: it is not in the mix, not on a bus and not a
//! stem, and it is still in the document with its clips and its
//! settings.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-inactive-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of that level: one frame, so that the mix is one frame
/// and the tracks can be told apart.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with two tracks, `kick` and `snare`, each with a sound of
/// its own.
fn two_tracks(session: &mut Session, directory: &str) {
    for (name, level) in [("kick", 0.5_f32), ("snare", 0.25_f32)] {
        let path = format!("{directory}/{name}.wav");
        a_sound(&path, level);
        session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
        let sources: Vec<u64> = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .collect();
        let source = sources.last().copied().expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
        let track = session.document().node_by_name(name).expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
            .expect("the clip is added");
    }
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The **files** in a directory, by name.
fn files_in(directory: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("the directory is there")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// Whether a track's **voice is off**.
fn inactive_of(session: &Session, track: u64) -> Option<bool> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("inactive").and_then(|inactive| inactive.as_bool())
}

/// A track's voice is **on** to begin with.
#[test]
fn a_track_carries_its_voice() {
    let mut session = session();
    let directory = directory("carries");
    two_tracks(&mut session, &directory);
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    assert_eq!(inactive_of(&session, kick), Some(false), "the voice is on");
}

/// A track's voice is turned **off** and on again.
#[test]
fn a_voice_is_turned_off_and_on_again() {
    let mut session = session();
    let directory = directory("turned");
    two_tracks(&mut session, &directory);
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    session
        .execute("sound.track.inactive", &json!({ "track": kick, "inactive": true }))
        .expect("the voice is off");
    assert_eq!(inactive_of(&session, kick), Some(true), "the voice is off");
    session
        .execute("sound.track.inactive", &json!({ "track": kick, "inactive": false }))
        .expect("the voice is on");
    assert_eq!(inactive_of(&session, kick), Some(false), "the voice is on");
}

/// An inactive track is **not heard**: it is not in the mix.
#[test]
fn an_inactive_track_is_not_heard() {
    let mut session = session();
    let directory = directory("heard");
    two_tracks(&mut session, &directory);
    let mix_path = format!("{directory}/mix.wav");
    let quiet_path = format!("{directory}/quiet.wav");
    session.execute("sound.render", &json!({ "path": mix_path })).expect("the sound is rendered");
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    session
        .execute("sound.track.inactive", &json!({ "track": kick, "inactive": true }))
        .expect("the voice is off");
    session.execute("sound.render", &json!({ "path": quiet_path })).expect("the sound is rendered");

    let loud = rendered(&mix_path);
    let quiet = rendered(&quiet_path);
    assert!((loud.samples[0] - 0.75).abs() < 1e-4, "both tracks were heard: {}", loud.samples[0]);
    assert!((quiet.samples[0] - 0.25).abs() < 1e-4, "and now only one: {}", quiet.samples[0]);
}

/// The track that is not heard is **not gone**: its clip is still in
/// the document, and its voice can be turned on again.
#[test]
fn an_inactive_track_is_not_gone() {
    let mut session = session();
    let directory = directory("not-gone");
    two_tracks(&mut session, &directory);
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    session
        .execute("sound.track.inactive", &json!({ "track": kick, "inactive": true }))
        .expect("the voice is off");
    let clips = session.document().walk().filter(|node| node.kind == "sound.clip").count();
    assert_eq!(clips, 2, "the clips are still there");
    let mix_path = format!("{directory}/mix.wav");
    session.execute("sound.render", &json!({ "path": mix_path })).expect("the sound is rendered");
    let mix = rendered(&mix_path);
    assert!((mix.samples[0] - 0.25).abs() < 1e-4, "and the track is still quiet: {}", mix.samples[0]);
}

/// An inactive track is **not bounced**: a stem of silence is a file
/// nobody asked for.
#[test]
fn an_inactive_track_is_not_bounced() {
    let mut session = session();
    let directory = directory("bounced");
    two_tracks(&mut session, &directory);
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    session
        .execute("sound.track.inactive", &json!({ "track": kick, "inactive": true }))
        .expect("the voice is off");
    let stems = format!("{directory}/stems");
    session.execute("sound.render.stems", &json!({ "directory": stems })).expect("the stems are bounced");
    assert_eq!(files_in(&stems), vec!["snare.wav".to_string()]);
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    two_tracks(&mut session, &directory);
    let kick = session.document().node_by_name("kick").expect("the track is there").id;
    let refused =
        session.execute("sound.track.inactive", &json!({ "track": kick + 1_000_000, "inactive": true }));
    assert!(refused.is_err(), "there is no such track");
}
