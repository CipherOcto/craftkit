//! The **metronome**: the click at every beat.
//!
//! §146 found that the sound battery could play a sound and could not
//! count it out. The family has a click — a blip on every beat, and a
//! different blip on the first beat of a bar — and this is the arc that
//! brings it.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-click-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A session with a **silent** track, a clip of that many frames, and
/// the metronome as the caller asked for it.
fn a_sound(session: &mut Session, source_path: &str, frames: usize, metronome: bool) -> String {
    let bytes = craftkit_wav::write(1, 48_000, &vec![0.0_f32; frames]);
    std::fs::write(source_path, bytes).expect("the sound is written");
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": frames, "source": source }))
        .expect("the clip is added");
    session.execute("sound.session.set", &json!({ "metronome": metronome })).expect("the metronome is set");
    source_path.to_string()
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The loudest sample in a stretch of the sound.
fn peak(samples: &[f32], from: usize, len: usize) -> f32 {
    samples
        .get(from..(from + len).min(samples.len()))
        .unwrap_or(&[])
        .iter()
        .fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()))
}

/// How many times the sound **crosses zero** in a stretch of it.
///
/// A blip is a sine, and a sine crosses zero twice a period, so the
/// crossings say the pitch: a click an octave higher crosses twice as
/// often.
fn crossings(samples: &[f32], from: usize, len: usize) -> usize {
    let mut count = 0;
    for index in from..(from + len).min(samples.len()) {
        let before = index > 0 && samples[index - 1] < 0.0;
        let after = samples[index] < 0.0;
        if before != after {
            count += 1;
        }
    }
    count
}

/// The click is **not heard** unless it is asked for.
#[test]
fn the_click_is_not_heard_unless_it_is_asked_for() {
    let directory = directory("quiet");
    let source_path = format!("{directory}/silent.wav");
    let output_path = format!("{directory}/out.wav");
    let mut session = session();
    a_sound(&mut session, &source_path, 48_000, false);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert_eq!(sound.frames, 48_000, "the clip is a second long");
    assert!(sound.samples.iter().all(|sample| sample.abs() < 1e-6), "and the sound is silent");
}

/// The click is **heard** on the beat.
///
/// The tempo is 120 beats a minute, so a beat is half a second, and
/// the click is at the start of the sound and half a second later.
#[test]
fn the_click_is_heard_on_the_beat() {
    let directory = directory("heard");
    let source_path = format!("{directory}/silent.wav");
    let output_path = format!("{directory}/out.wav");
    let mut session = session();
    a_sound(&mut session, &source_path, 48_000, true);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!(peak(&sound.samples, 0, 1_440) > 1e-6, "there is a click at the start");
    assert!(peak(&sound.samples, 24_000, 1_440) > 1e-6, "and one at the beat");
    assert!(peak(&sound.samples, 10_000, 1_440) < 1e-6, "and none between the beats");
    assert!(peak(&sound.samples, 30_000, 1_440) < 1e-6, "and none after the beat");
}

/// The **accent** is on the first beat of the bar.
///
/// The accent is a note an octave higher, so it crosses zero twice as
/// often.
#[test]
fn the_accent_is_on_the_first_beat_of_the_bar() {
    let directory = directory("accent");
    let source_path = format!("{directory}/silent.wav");
    let output_path = format!("{directory}/out.wav");
    let mut session = session();
    a_sound(&mut session, &source_path, 48_000, true);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    let accent = crossings(&sound.samples, 0, 1_440);
    let plain = crossings(&sound.samples, 24_000, 1_440);
    assert!(accent > plain, "the accent is the higher click: {accent} crossings and {plain}");
}

/// The click does **not** pass through a track.
///
/// The click is mixed in after the master, so a clip that is silent is
/// silent and the click is still there: the click is a voice of its
/// own.
#[test]
fn the_click_does_not_pass_through_a_track() {
    let directory = directory("bypasses");
    let source_path = format!("{directory}/silent.wav");
    let output_path = format!("{directory}/out.wav");
    let mut session = session();
    a_sound(&mut session, &source_path, 48_000, true);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sound = rendered(&output_path);
    assert!(peak(&sound.samples, 0, 1_440) > 1e-6, "the click is heard");
}

/// The **level** of the click is the caller's.
///
/// The click is quieter when the caller says so, and the setting is a
/// level in decibels like any other.
#[test]
fn the_level_of_the_click_is_the_callers() {
    let directory = directory("level");
    let source_path = format!("{directory}/silent.wav");
    let loud_path = format!("{directory}/loud.wav");
    let quiet_path = format!("{directory}/quiet.wav");
    let mut session = session();
    a_sound(&mut session, &source_path, 48_000, true);
    session.execute("sound.render", &json!({ "path": loud_path })).expect("the sound is rendered");
    session.execute("sound.session.set", &json!({ "click_volume_db": -60.0 })).expect("the level is set");
    session.execute("sound.render", &json!({ "path": quiet_path })).expect("the sound is rendered");
    let loud = peak(&rendered(&loud_path).samples, 0, 1_440);
    let quiet = peak(&rendered(&quiet_path).samples, 0, 1_440);
    assert!(loud > quiet, "the quiet click is quieter: {quiet} and {loud}");
}
