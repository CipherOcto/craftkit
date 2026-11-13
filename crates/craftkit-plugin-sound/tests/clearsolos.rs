//! The **solos** of every track cleared.
//!
//! The tree's rules: every track of the document is unsoloed at once,
//! and the flag that says a track is heard even when another is soloed
//! is not a solo and is left as it is.

use std::sync::atomic::{AtomicUsize, Ordering};

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
///
/// The name is **unique per call**: the tests of one binary run in
/// parallel threads, and two tests that shared a directory would race —
/// one of them removing the folder the other is writing into.
fn directory(name: &str) -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = craftkit_scratch::dir(&format!("craftkit-v2-clearsolos-{name}-{unique}"))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **track**, and its id.
fn a_track(session: &mut Session, name: &str) -> u64 {
    let outcome = session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// A track with a clip on it, and its id.
fn a_track_with_a_clip(session: &mut Session, source_path: &str, name: &str) -> u64 {
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    let track = a_track(session, name);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    track
}

/// Whether the track is **soloed**.
fn soloed(session: &Session, track: u64) -> bool {
    let node = session.document().node(track).expect("the track is there");
    node.data["solo"].as_bool().expect("the track has a solo")
}

/// A sound of that many full samples.
fn a_sound(path: &str, frames: usize, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &vec![level; frames]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// Every soloed track is **unsoloed** at once.
#[test]
fn every_soloed_track_is_unsoloed_at_once() {
    let mut session = session();
    let one = a_track(&mut session, "one");
    let two = a_track(&mut session, "two");
    for track in [one, two] {
        session
            .execute("sound.track.solo", &json!({ "track": track, "solo": true }))
            .expect("the track is soloed");
    }
    let outcome = session.execute("sound.track.solo.clear", &json!({})).expect("the solos are cleared");
    assert_eq!(outcome.data["cleared"], json!(2), "both tracks are unsoloed: {outcome:?}");
    assert!(!soloed(&session, one), "the first track is not soloed");
    assert!(!soloed(&session, two), "the second track is not soloed");
}

/// A track that is **not soloed** is left as it is.
#[test]
fn a_track_that_is_not_soloed_is_left_alone() {
    let mut session = session();
    let track = a_track(&mut session, "one");
    let outcome = session.execute("sound.track.solo.clear", &json!({})).expect("the solos are cleared");
    assert!(!outcome.changed, "nothing was unsoloed: {outcome:?}");
    assert!(!soloed(&session, track), "the track is not soloed");
}

/// The clearing puts the **other tracks** back.
#[test]
fn the_clearing_puts_the_other_tracks_back() {
    let directory = directory("back");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    // A quarter each, so that two of them together are half of full
    // and the file does not clip them.
    a_sound(&source_path, 48_000, 0.25);
    let mut session = session();
    let first = a_track_with_a_clip(&mut session, &source_path, "drums");
    a_track_with_a_clip(&mut session, &source_path, "bass");
    session
        .execute("sound.track.solo", &json!({ "track": first, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.25).abs() < 1e-4, "the soloed track is heard alone: {}", sound.samples[0]);
    session.execute("sound.track.solo.clear", &json!({})).expect("the solos are cleared");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-4, "both tracks are heard: {}", sound.samples[0]);
}

/// The **solo safe** of a track is left as it is.
///
/// The tree's flag says a track is heard even when another is soloed.
/// It is not a solo, and clearing the solos does not touch it.
#[test]
fn the_solo_safe_of_a_track_is_left_as_it_is() {
    let mut session = session();
    let track = a_track(&mut session, "one");
    session
        .execute("sound.track.solo.safe", &json!({ "track": track, "solo_safe": true }))
        .expect("the track is solo safe");
    session
        .execute("sound.track.solo", &json!({ "track": track, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.track.solo.clear", &json!({})).expect("the solos are cleared");
    let node = session.document().node(track).expect("the track is there");
    assert_eq!(node.data["solo_safe"], json!(true), "the track is still solo safe");
}

/// The solo of a track that is **mixed** with others is its own.
///
/// This tree does not share the solo with the group (§ the track is
/// soloed), and so the track that is not soloed is not cleared by the
/// clearing of the one that is — it was not soloed to begin with.
#[test]
fn the_solo_of_a_mixed_track_is_its_own() {
    let mut session = session();
    let one = a_track(&mut session, "one");
    let two = a_track(&mut session, "two");
    let outcome = session
        .execute("sound.track.group", &json!({ "name": "kit", "tracks": [one, two] }))
        .expect("the tracks are in a group");
    let group = outcome.data["group"].as_u64().expect("the group has an id");
    session
        .execute("sound.track.group.set", &json!({ "group": group, "mix": true }))
        .expect("the group mixes its members");
    session.execute("sound.track.solo", &json!({ "track": one, "solo": true })).expect("the track is soloed");
    let outcome = session.execute("sound.track.solo.clear", &json!({})).expect("the solos are cleared");
    assert_eq!(outcome.data["cleared"], json!(1), "one track was soloed: {outcome:?}");
    assert!(!soloed(&session, one), "the first track is not soloed");
    assert!(!soloed(&session, two), "the second track was never soloed");
}
