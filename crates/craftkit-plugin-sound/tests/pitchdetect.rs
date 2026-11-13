//! The **pitch** of a clip: the notes its sound sings.
//!
//! The detector is the tree's own: **YIN**, the cumulative mean
//! normalised difference, and the runs of frames that sing one note.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-pitch-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **tone**: a sine of a frequency, half a second long.
fn tone(hz: f32) -> Vec<f32> {
    (0..24_000).map(|index| (index as f32 * hz * std::f32::consts::TAU / 48_000.0).sin() * 0.5).collect()
}

/// A **silence**: half a second of nothing.
fn silence() -> Vec<f32> {
    vec![0.0f32; 24_000]
}

/// A clip that plays a sound.
fn a_clip(session: &mut Session, path: &str, sound: &[f32]) -> u64 {
    let bytes = craftkit_wav::write(1, 48_000, sound);
    std::fs::write(path, bytes).expect("the sound is written");
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "the track" })).expect("the track is added");
    let track = session.document().node_by_name("the track").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 0, "length": sound.len(), "source": source }),
        )
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// The **notes** of a clip.
fn notes(session: &mut Session, clip: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("sound.clip.pitch.read", &json!({ "clip": clip })).expect("the clip is heard");
    outcome.data["notes"].as_array().expect("the notes are a list").clone()
}

/// **A tone of four hundred and forty hertz is heard as the note
/// sixty-nine.**
#[test]
fn a_tone_of_four_hundred_and_forty_hertz_is_heard() {
    let mut session = session();
    let directory = directory("a440");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &tone(440.0));
    let notes = notes(&mut session, clip);
    assert_eq!(notes.len(), 1, "one note is heard: {notes:?}");
    assert_eq!(notes[0]["pitch"], json!(69), "and it is the middle A: {notes:?}");
    let start = notes[0]["start"].as_f64().expect("the note starts");
    let end = notes[0]["end"].as_f64().expect("the note ends");
    assert!(start < 0.01, "the note starts at once: {start}");
    assert!(end > 0.4 && end < 0.6, "and it lasts the tone: {end}");
    let velocity = notes[0]["velocity"].as_u64().expect("the note has a velocity");
    assert!(velocity > 0 && velocity < 128, "and it is loud enough to hear: {velocity}");
}

/// **A tone of another pitch is heard as its own note.**
#[test]
fn a_tone_of_another_pitch_is_heard_as_its_own_note() {
    let mut session = session();
    let directory = directory("a220");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &tone(220.0));
    let notes = notes(&mut session, clip);
    assert_eq!(notes.len(), 1, "one note is heard: {notes:?}");
    assert_eq!(notes[0]["pitch"], json!(57), "and it is the A below the middle: {notes:?}");
}

/// **A silence is heard as no note.**
#[test]
fn a_silence_is_heard_as_no_note() {
    let mut session = session();
    let directory = directory("silence");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &silence());
    let notes = notes(&mut session, clip);
    assert!(notes.is_empty(), "a silence sings nothing: {notes:?}");
}

/// **The bounds of the search are the tree's own, and a range outside
/// them is refused.**
#[test]
fn the_bounds_of_the_search_are_refused_when_they_are_outside() {
    let mut session = session();
    let directory = directory("bounds");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &tone(440.0));
    let refusal = session
        .execute("sound.clip.pitch.read", &json!({ "clip": clip, "min_hz": 10_000.0, "max_hz": 20_000.0 }))
        .expect_err("the bounds are refused");
    let message = refusal.to_string();
    assert!(message.contains("hertz"), "the refusal says the bound: {message}");
}

/// **A clip that plays no sound has nothing to hear, and is refused.**
#[test]
fn a_clip_that_plays_no_sound_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "the track" })).expect("the track is added");
    let track = session.document().node_by_name("the track").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let refusal =
        session.execute("sound.clip.pitch.read", &json!({ "clip": clip })).expect_err("the clip is refused");
    let message = refusal.to_string();
    assert!(message.contains("plays no sound"), "the refusal says so: {message}");
}
