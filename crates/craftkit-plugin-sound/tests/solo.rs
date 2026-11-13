//! The **solo**: a track that is heard alone.
//!
//! §150 found that a track could be muted and could not be soloed, and
//! that the family's mixer silences the other tracks when one of them
//! is soloed. This is the arc that brings that back.

use craftkit_sdk::document::RecordingCanvas;
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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-solo-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A session with a sound, a track and a clip on it.
fn a_track(session: &mut Session, source_path: &str, name: &str) -> u64 {
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    track
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// A sound of that many full samples.
fn a_sound(path: &str, frames: usize, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &vec![level; frames]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A soloed track silences the **others**.
#[test]
fn a_soloed_track_silences_the_others() {
    let directory = directory("silences");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    // A quarter each, so that two of them together are half of full
    // and the file does not clip them.
    a_sound(&source_path, 48_000, 0.25);
    let mut session = session();
    let first = a_track(&mut session, &source_path, "drums");
    a_track(&mut session, &source_path, "bass");
    session
        .execute("sound.track.solo", &json!({ "track": first, "solo": true }))
        .expect("the track is soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    // The soloed track is at full and the other is silent, so the mix
    // is at full and not at twice it.
    assert!((sound.samples[0] - 0.25).abs() < 1e-4, "the soloed track is heard: {}", sound.samples[0]);
}

/// A track that is **not** soloed is heard with the others.
#[test]
fn a_track_that_is_not_soloed_is_heard_with_the_others() {
    let directory = directory("together");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    // A quarter each, so that two of them together are half of full
    // and the file does not clip them.
    a_sound(&source_path, 48_000, 0.25);
    let mut session = session();
    a_track(&mut session, &source_path, "drums");
    a_track(&mut session, &source_path, "bass");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-4, "both tracks are heard: {}", sound.samples[0]);
}

/// **Un**soloing a track puts the others back.
#[test]
fn unsoloing_a_track_puts_the_others_back() {
    let directory = directory("unsolo");
    let source_path = format!("{directory}/sound.wav");
    let output_path = format!("{directory}/out.wav");
    // A quarter each, so that two of them together are half of full
    // and the file does not clip them.
    a_sound(&source_path, 48_000, 0.25);
    let mut session = session();
    let first = a_track(&mut session, &source_path, "drums");
    a_track(&mut session, &source_path, "bass");
    session
        .execute("sound.track.solo", &json!({ "track": first, "solo": true }))
        .expect("the track is soloed");
    session
        .execute("sound.track.solo", &json!({ "track": first, "solo": false }))
        .expect("the track is not soloed");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!((sound.samples[0] - 0.5).abs() < 1e-4, "both tracks are heard: {}", sound.samples[0]);
}

/// The solo is **in the document**.
///
/// The payload says so, and a payload is what is saved and what is
/// shared.
#[test]
fn the_solo_is_in_the_document() {
    let mut session = session();
    let directory = directory("payload");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4_800, 1.0);
    let track = a_track(&mut session, &source_path, "drums");
    session
        .execute("sound.track.solo", &json!({ "track": track, "solo": true }))
        .expect("the track is soloed");
    let payload = &session.document().node(track).expect("the track is there").data;
    assert_eq!(payload.get("solo").and_then(|solo| solo.as_bool()), Some(true), "the track is soloed");
}

/// The soloed track is **drawn** as a double rule.
///
/// The row says which track is heard alone.
#[test]
fn the_soloed_track_is_drawn_as_a_double_rule() {
    let mut session = session();
    let directory = directory("drawn");
    let source_path = format!("{directory}/sound.wav");
    a_sound(&source_path, 4_800, 1.0);
    let track = a_track(&mut session, &source_path, "drums");
    let mut plain = RecordingCanvas::default();
    session.paint(&mut plain);
    let rules =
        |canvas: &RecordingCanvas| canvas.drawn.iter().filter(|drawn| drawn.starts_with("path(")).count();
    let before = rules(&plain);
    session
        .execute("sound.track.solo", &json!({ "track": track, "solo": true }))
        .expect("the track is soloed");
    let mut soloed = RecordingCanvas::default();
    session.paint(&mut soloed);
    assert_eq!(rules(&soloed), before + 1, "the row has a second rule: {:?}", soloed.drawn);
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let refused = session.execute("sound.track.solo", &json!({ "track": 1, "solo": true }));
    assert!(refused.is_err(), "there is no track there");
}
