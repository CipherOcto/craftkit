//! The **solo-safe** track: a track that a solo does not silence.
//!
//! A solo silences the other tracks, and a solo-safe track is not
//! silenced by it: it is a track that stays when somebody listens to
//! one thing on purpose — a reference, a click, a talk track.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-solosafe-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of that level, one frame long.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with two tracks, `drums` and `click`, each with a sound
/// of its own, and their ids.
fn two_tracks(session: &mut Session, directory: &str) -> (u64, u64) {
    let mut ids = Vec::new();
    for (name, level) in [("drums", 0.5_f32), ("click", 0.25_f32)] {
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
        ids.push(track);
    }
    (ids[0], ids[1])
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The solo-safe a track carries.
fn safe_of(session: &Session, track: u64) -> Option<bool> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("solo_safe").and_then(|safe| safe.as_bool())
}

/// A track carries a **solo safe**, and it is off.
#[test]
fn a_track_carries_its_solo_safe() {
    let mut session = session();
    let directory = directory("carries");
    let (drums, _) = two_tracks(&mut session, &directory);
    assert_eq!(safe_of(&session, drums), Some(false), "the track is not solo safe");
}

/// A solo silences the other tracks.
#[test]
fn a_solo_silences_the_other_tracks() {
    let directory = directory("solo");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    a_sound(&source_path, 0.5);

    let mut session = session();
    let (drums, _) = two_tracks(&mut session, &directory);
    session
        .execute("sound.track.solo", &json!({ "track": drums, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-3, "the soloed track is heard: {}", sound.samples[0]);
}

/// A **solo-safe** track is not silenced by the solo: it stays when
/// somebody listens to one thing on purpose.
#[test]
fn a_solo_safe_track_stays_when_the_solo_is_on() {
    let directory = directory("safe");
    let output_path = format!("{directory}/out.wav");
    a_sound(&format!("{directory}/sound.wav"), 0.5);

    let mut session = session();
    let (drums, click) = two_tracks(&mut session, &directory);
    session
        .execute("sound.track.solo.safe", &json!({ "track": click, "solo_safe": true }))
        .expect("the track is solo safe");
    session
        .execute("sound.track.solo", &json!({ "track": drums, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    // The soloed track at half and the safe track at a quarter.
    assert!((sound.samples[0] - 0.75).abs() < 1e-3, "both are heard: {}", sound.samples[0]);
}

/// A track that is **not** solo safe is silenced by the solo.
#[test]
fn a_track_that_is_not_solo_safe_is_silenced() {
    let directory = directory("not-safe");
    let output_path = format!("{directory}/out.wav");
    a_sound(&format!("{directory}/sound.wav"), 0.5);

    let mut session = session();
    let (drums, _) = two_tracks(&mut session, &directory);
    session
        .execute("sound.track.solo", &json!({ "track": drums, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-3, "only the soloed track is heard: {}", sound.samples[0]);
}

/// The solo safe can be **taken off**, and the track is silenced
/// again.
#[test]
fn the_solo_safe_can_be_taken_off() {
    let directory = directory("taken-off");
    let output_path = format!("{directory}/out.wav");
    a_sound(&format!("{directory}/sound.wav"), 0.5);

    let mut session = session();
    let (drums, click) = two_tracks(&mut session, &directory);
    session
        .execute("sound.track.solo.safe", &json!({ "track": click, "solo_safe": true }))
        .expect("the track is solo safe");
    session
        .execute("sound.track.solo.safe", &json!({ "track": click, "solo_safe": false }))
        .expect("the track is not solo safe any more");
    session
        .execute("sound.track.solo", &json!({ "track": drums, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");

    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-3, "only the soloed track is heard: {}", sound.samples[0]);
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let (drums, _) = two_tracks(&mut session, &directory);
    let refused =
        session.execute("sound.track.solo.safe", &json!({ "track": drums + 1_000_000, "solo_safe": true }));
    assert!(refused.is_err(), "there is no such track");
}
