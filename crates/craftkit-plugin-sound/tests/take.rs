//! The **take**: one of a track's playlists is the one that is played.
//!
//! The tree's track has several playlists and an index of the one that
//! is played. Here a track had playlists and every one of them was
//! played at once.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-take-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source.
const QUIET: f32 = 0.25;

/// How long the source is, in samples.
const LONG: usize = 1000;

/// A session with a **track** that plays a quiet source, and the
/// source's id.
fn a_track(session: &mut Session, path: &str) -> (u64, u64) {
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
    (track, source)
}

/// A **clip** in a take of the track.
fn a_clip(session: &mut Session, track: u64, source: u64, take: &str, start: i64) {
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "playlist": take, "start": start, "length": LONG, "source": source }),
        )
        .expect("the clip is added");
}

/// The sample of what was rendered, at a place.
fn rendered(path: &str, at: usize) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples.get(at).copied().unwrap_or(0.0)
}

/// Only the take that is **played** is heard: the other take is
/// silent.
#[test]
fn only_the_take_that_is_played_is_heard() {
    let directory = directory("played");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, source) = a_track(&mut session, &source_path);
    session
        .execute("sound.playlist.add", &json!({ "track": track, "name": "B" }))
        .expect("the take is added");
    a_clip(&mut session, track, source, "A", 0);
    a_clip(&mut session, track, source, "B", 2 * LONG as i64);
    session
        .execute("sound.playlist.active", &json!({ "track": track, "playlist": "B" }))
        .expect("the take is played");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let first = rendered(&output_path, 0);
    let second = rendered(&output_path, 2 * LONG);
    assert!(first.abs() < 0.01, "the take that is not played is silent: {first}");
    assert!((second - QUIET).abs() < 0.01, "the take that is played is heard: {second}");
}

/// The first take is played when nobody has said otherwise.
#[test]
fn the_first_take_is_played_by_default() {
    let directory = directory("default");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, source) = a_track(&mut session, &source_path);
    session
        .execute("sound.playlist.add", &json!({ "track": track, "name": "B" }))
        .expect("the take is added");
    a_clip(&mut session, track, source, "A", 0);
    a_clip(&mut session, track, source, "B", 2 * LONG as i64);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let first = rendered(&output_path, 0);
    let second = rendered(&output_path, 2 * LONG);
    assert!((first - QUIET).abs() < 0.01, "the first take is heard: {first}");
    assert!(second.abs() < 0.01, "the other take is silent: {second}");
}

/// A take that is **not there** is refused by name: the command says
/// which take it meant.
#[test]
fn a_take_that_is_not_there_is_refused() {
    let directory = directory("missing");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, _source) = a_track(&mut session, &source_path);
    let refused = session.execute("sound.playlist.active", &json!({ "track": track, "playlist": "C" }));
    let message = match refused {
        Ok(_) => panic!("a take that is not there is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("no take called"), "the refusal says why: {message}");
    let twice = session.execute("sound.playlist.add", &json!({ "track": track, "name": "A" }));
    assert!(twice.is_err(), "a take with the same name twice is refused");
}
