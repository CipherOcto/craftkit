//! The **loudness** of a clip: the ITU-R BS.1770 loudness, as EBU R128
//! asks for it, and the true peak beside it.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-loudness-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **tone**: a sine of a frequency and a level.
fn tone(hz: f32, level: f32, seconds: f32) -> Vec<f32> {
    (0..(48_000_f32 * seconds) as usize)
        .map(|index| (index as f32 * hz * std::f32::consts::TAU / 48_000.0).sin() * level)
        .collect()
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

/// The **loudness** of a clip.
fn loudness(session: &mut Session, clip: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("sound.clip.loudness.read", &json!({ "clip": clip })).expect("the clip is measured");
    outcome.data.clone()
}

/// **A tone of half the scale is about nine LUFS.**
#[test]
fn a_tone_of_half_the_scale_is_about_nine_lufs() {
    let mut session = session();
    let directory = directory("tone");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &tone(1000.0, 0.5, 0.5));
    let measured = loudness(&mut session, clip);
    let momentary = measured["momentary_lufs"].as_f64().expect("the momentary loudness is said");
    assert!((-12.0..-6.0).contains(&momentary), "a tone of half the scale is about nine LUFS: {momentary}");
    let integrated = measured["integrated_lufs"].as_f64().expect("the integrated loudness is said");
    assert!(
        (momentary - 1.0..momentary + 1.0).contains(&integrated),
        "and the integrated loudness is the same: {integrated} against {momentary}"
    );
}

/// **The true peak of a tone is its own level.**
#[test]
fn the_true_peak_of_a_tone_is_its_own_level() {
    let mut session = session();
    let directory = directory("peak");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &tone(1000.0, 0.5, 0.5));
    let measured = loudness(&mut session, clip);
    let peak = measured["true_peak_db"].as_f64().expect("the true peak is said");
    assert!((-6.6..-5.5).contains(&peak), "half the scale is about six decibels down: {peak}");
}

/// **A silence is the least loudness there is.**
#[test]
fn a_silence_is_the_least_loudness_there_is() {
    let mut session = session();
    let directory = directory("silence");
    let path = format!("{directory}/sound.wav");
    let clip = a_clip(&mut session, &path, &tone(1000.0, 0.0, 0.5));
    let measured = loudness(&mut session, clip);
    assert_eq!(measured["momentary_lufs"], json!(-144.0), "a silence is nothing: {measured:?}");
    assert_eq!(measured["true_peak_db"], json!(-144.0), "and its peak is nothing too");
}

/// **A clip that plays no sound has nothing to measure, and is
/// refused.**
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
    let refusal = session
        .execute("sound.clip.loudness.read", &json!({ "clip": clip }))
        .expect_err("the clip is refused");
    let message = refusal.to_string();
    assert!(message.contains("plays no sound"), "the refusal says so: {message}");
}
