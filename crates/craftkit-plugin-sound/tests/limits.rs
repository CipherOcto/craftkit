//! The **limits of the sound battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.
//!
//! **Two of them are knobs.** A repeat count is clamped into its bounds
//! and the command says so in its own documentation; a knob is not a
//! fact about the material, and the test below says what the knob does.

use craftkit_plugin_sound::MAX_NAME_CHARS;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-limits-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **mono** sound of a second.
fn a_sound(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.wav");
    let mut samples = Vec::with_capacity(48_000);
    for frame in 0..48_000_u32 {
        samples.push(frame as f32 / 48_000.0);
    }
    std::fs::write(&path, craftkit_wav::write(1, 48_000, &samples)).expect("the file is on the disk");
    path
}

/// A session with a track and a clip of a second on it.
fn a_track(session: &mut Session, name: &str) -> (u64, u64) {
    let path = a_sound(name);
    let source = session
        .execute("sound.source.add", &json!({ "path": path, "channels": 1, "frames": 48_000 }))
        .expect("the source is added")
        .data["id"]
        .as_u64()
        .expect("the source has an id");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// A repeat count of **zero** is clamped to **one**.
///
/// The count is a knob and not a fact about the material, and the
/// command's own documentation says the number is clamped into its
/// bounds.
#[test]
fn a_repeat_count_of_zero_is_clamped_to_one() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "repeat-zero");
    let outcome = session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 0, "end": 24_000, "count": 0 }))
        .expect("the range is repeated");
    assert_eq!(outcome.data["repeats"], 1, "one is none: {outcome:?}");
}

/// A repeat count **over the limit** is clamped to the limit.
///
/// The tree's repeat is a number from one to nine hundred and
/// ninety-nine, and a repeat of ten thousand times is a track of ten
/// thousand times the material.
#[test]
fn a_repeat_count_over_the_limit_is_clamped() {
    let mut session = session();
    let (track, _) = a_track(&mut session, "repeat-many");
    let outcome = session
        .execute("sound.track.repeat", &json!({ "track": track, "start": 0, "end": 24_000, "count": 1_000 }))
        .expect("the range is repeated");
    assert_eq!(outcome.data["repeats"], 999, "nine hundred and ninety-nine: {outcome:?}");
}

/// A **name** over `MAX_NAME_CHARS` is refused.
#[test]
fn a_name_over_the_limit_is_refused() {
    let mut session = session();
    let (_, clip) = a_track(&mut session, "rename");
    let name = "n".repeat(MAX_NAME_CHARS + 1);
    let error = session
        .execute("sound.clip.rename", &json!({ "clip": clip, "name": name }))
        .expect_err("that name is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **pitch range** over the limit is refused.
///
/// The detector looks for notes between two hertz, and a range outside
/// the tree's own bounds is not a range of notes.
#[test]
fn a_pitch_range_over_the_limit_is_refused() {
    let mut session = session();
    let (_, clip) = a_track(&mut session, "pitch");
    let error = session
        .execute("sound.clip.pitch.read", &json!({ "clip": clip, "min_hz": 100.0, "max_hz": 6_000.0 }))
        .expect_err("that range is too wide");
    assert!(error.to_string().contains("hertz"), "{error}");
}
