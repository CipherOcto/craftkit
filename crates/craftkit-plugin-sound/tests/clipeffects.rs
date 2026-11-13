//! The **clip's own effects**: an insert on a clip and not on the
//! track.
//!
//! The tree's clip effects are an equaliser and a compressor on one
//! clip and not on the track, and they run over that clip's own sound
//! before it is mixed into the track.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-clipeffects-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source.
const QUIET: f32 = 0.25;

/// How long the source is, in samples.
const LONG: usize = 1000;

/// A session with a **track** that plays a quiet source.
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

/// A **clip** on the track, and its id.
fn a_clip(session: &mut Session, track: u64, source: u64, start: i64) -> u64 {
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": start, "length": LONG, "source": source }),
        )
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// A **gain** insert of six decibels down, on the track or the clip.
fn a_gain_insert(session: &mut Session, owner: &str, id: u64) {
    session
        .execute("sound.insert.add", &json!({ owner: id, "plugin": "gain" }))
        .expect("the insert is added");
    let insert =
        session.document().walk().find(|node| node.kind == "sound.insert").expect("the insert is there");
    session
        .execute("sound.insert.set", &json!({ "insert": insert.id, "parameters": { "gain_db": -6.0 } }))
        .expect("the insert is set");
}

/// The sample of what was rendered, at a place.
fn rendered(path: &str, at: usize) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples.get(at).copied().unwrap_or(0.0)
}

/// A clip may carry an **effect** of its own: the clip is heard at
/// half.
#[test]
fn a_clip_may_carry_an_effect() {
    let directory = directory("one");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, source) = a_track(&mut session, &source_path);
    let clip = a_clip(&mut session, track, source, 0);
    a_gain_insert(&mut session, "clip", clip);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path, 0);
    let expected = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the clip's effect is heard: {sample}");
}

/// The effect is the **clip's own**: the other clip on the same track
/// is heard whole.
#[test]
fn the_effect_is_the_clip_s_own() {
    let directory = directory("own");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, source) = a_track(&mut session, &source_path);
    let first = a_clip(&mut session, track, source, 0);
    a_gain_insert(&mut session, "clip", first);
    let second = a_clip(&mut session, track, source, 2 * LONG as i64);
    let _ = second;
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let with = rendered(&output_path, 0);
    let without = rendered(&output_path, 2 * LONG);
    let half = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((with - half).abs() < 0.01, "the first clip is at half: {with}");
    assert!((without - QUIET).abs() < 0.01, "the second clip is whole: {without}");
}

/// An insert is on **one** thing: a track or a clip, and not both and
/// not neither.
#[test]
fn an_insert_is_on_one_thing() {
    let directory = directory("one-thing");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, source) = a_track(&mut session, &source_path);
    let clip = a_clip(&mut session, track, source, 0);
    let neither = session.execute("sound.insert.add", &json!({ "plugin": "gain" }));
    assert!(neither.is_err(), "an insert on nothing is refused");
    let both =
        session.execute("sound.insert.add", &json!({ "track": track, "clip": clip, "plugin": "gain" }));
    let message = match both {
        Ok(_) => panic!("an insert on two things is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("one thing"), "the refusal says why: {message}");
}
