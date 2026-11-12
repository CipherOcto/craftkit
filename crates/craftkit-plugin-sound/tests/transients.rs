//! The **separation of a clip at its transients**: the places where the
//! sound starts again.
//!
//! The detector is the tree's own: it watches the **spectrum** of the
//! sound and finds the places where the bands of the spectrum rise.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-transients-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A number that is not quite a number, and not the same one twice.
fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// A **hit**: a burst of noise that dies away, the way a drum does.
fn hit(level: f32, decay: f32, seed: &mut u32) -> Vec<f32> {
    let mut previous = 0.0;
    (0..(48_000_f32 * decay * 6.0) as usize)
        .map(|index| {
            let value = noise(seed);
            let high = value - previous;
            previous = value;
            level * high * (-(index as f32) / 48_000.0 / decay).exp()
        })
        .collect()
}

/// A sound of two hits: a quarter of a second of silence, a hit, and
/// then the same again.
fn two_hits() -> Vec<f32> {
    let mut seed = 7;
    let mut samples: Vec<f32> = Vec::new();
    for _ in 0..2 {
        samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
        samples.extend(hit(0.9, 0.04, &mut seed));
    }
    samples
}

/// A session with a track, a source of that sound and a clip that
/// plays all of it.
fn a_clip(session: &mut Session, path: &str, sound: &[f32]) -> (u64, u64) {
    let bytes = craftkit_wav::write(1, 48_000, sound);
    std::fs::write(path, bytes).expect("the sound is written");
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": 0, "length": sound.len(), "source": source }),
        )
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The clips of the document, as `(start, length)`.
fn clips_of(session: &Session) -> Vec<(i64, i64)> {
    let mut clips: Vec<(i64, i64)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| {
            let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
            let length = node.data.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
            (start, length)
        })
        .collect();
    clips.sort();
    clips
}

/// A clip is separated at the **transients** of its sound: the two hits
/// make three pieces.
#[test]
fn a_clip_is_separated_at_its_transients() {
    let mut session = session();
    let directory = directory("separate");
    let path = format!("{directory}/sound.wav");
    let sound = two_hits();
    let (_, clip) = a_clip(&mut session, &path, &sound);
    let separated = session
        .execute("sound.clip.separate.transients", &json!({ "clip": clip }))
        .expect("the clip is separated");
    assert_eq!(separated.data["transients"], 2, "the two hits are found: {separated:?}");
    assert_eq!(separated.data["pieces"], 3, "and the clip is in three");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 3, "the clip is in three: {clips:?}");
    // The detector places an onset within a few milliseconds of the
    // attack it found, and three milliseconds is the tree's own
    // tolerance.
    assert!(
        clips[1].0 >= 4_800 - 150 && clips[1].0 <= 4_800 + 150,
        "the first cut is at the first hit: {clips:?}"
    );
    // The second hit comes after the first one has died away.
    let second = (4_800 + sound.len() / 2) as i64;
    assert!(
        clips[2].0 >= second - 150 && clips[2].0 <= second + 150,
        "and the second at the second: {clips:?}"
    );
}

/// A clip that plays **no sound** has no transients to be separated
/// at, and it is refused.
#[test]
fn a_clip_without_a_source_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4_800 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let refused = session.execute("sound.clip.separate.transients", &json!({ "clip": clip }));
    assert!(refused.is_err(), "there is no sound to find the transients in");
}

/// The **sensitivity** is how small a rise may be: a hit that is very
/// quiet is found by a sensitive detector and not by a blunt one.
///
/// The threshold is a **fixed amount** and not one relative to the
/// loudest hit, so that a quiet hit counts — the tree's own rule, and
/// the reason a quiet hit is found at all.
#[test]
fn the_sensitivity_is_how_small_a_rise_may_be() {
    let mut seed = 7;
    // A loud hit and then one that is sixty decibels below it.
    let mut sound: Vec<f32> = Vec::new();
    sound.extend(std::iter::repeat_n(0.0_f32, 4_800));
    sound.extend(hit(0.9, 0.04, &mut seed));
    sound.extend(std::iter::repeat_n(0.0_f32, 4_800));
    sound.extend(hit(0.001, 0.03, &mut seed));

    let mut blunt = session();
    let blunt_directory = directory("blunt");
    let path = format!("{blunt_directory}/sound.wav");
    let (_, clip) = a_clip(&mut blunt, &path, &sound);
    let found = blunt
        .execute("sound.clip.separate.transients", &json!({ "clip": clip, "sensitivity": 0.0 }))
        .expect("the clip is separated");
    assert_eq!(found.data["transients"], 1, "the blunt detector hears the loud hit: {found:?}");

    let mut sensitive = session();
    let sensitive_directory = directory("sensitive");
    let path = format!("{sensitive_directory}/sound.wav");
    let (_, clip) = a_clip(&mut sensitive, &path, &sound);
    let found = sensitive
        .execute("sound.clip.separate.transients", &json!({ "clip": clip, "sensitivity": 1.0 }))
        .expect("the clip is separated");
    assert_eq!(found.data["transients"], 2, "the sensitive one hears the quiet hit too: {found:?}");
}
