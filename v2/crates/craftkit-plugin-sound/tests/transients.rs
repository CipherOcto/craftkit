//! The **separation of a clip at its transients**: the places where the
//! sound starts again.

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

/// A sound of two hits: a quarter of a second of silence, a hit, and
/// then the same again.
fn two_hits() -> Vec<f32> {
    let mut samples: Vec<f32> = Vec::new();
    for _ in 0..2 {
        samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
        samples.extend(std::iter::repeat_n(1.0_f32, 12_000));
    }
    samples
}

/// A session with a track, a source of that sound and a clip that
/// plays all of it.
fn a_clip(session: &mut Session, path: &str) -> (u64, u64) {
    let bytes = craftkit_wav::write(1, 48_000, &two_hits());
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 33_600, "source": source }))
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
    let (_, clip) = a_clip(&mut session, &path);
    let separated = session
        .execute("sound.clip.separate.transients", &json!({ "clip": clip }))
        .expect("the clip is separated");
    assert_eq!(separated.data["transients"], 2, "the two hits are found: {separated:?}");
    assert_eq!(separated.data["pieces"], 3, "and the clip is in three");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 3, "the clip is in three: {clips:?}");
    assert!(clips[1].0 >= 4_800 && clips[1].0 <= 5_400, "the first cut is at the first hit: {clips:?}");
    // The second hit starts at 21_600: the silence comes before each
    // of them.
    assert!(clips[2].0 >= 21_600 && clips[2].0 <= 22_200, "and the second at the second: {clips:?}");
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

/// The **sensitivity** is how small a rise may be: a sensitive detector
/// finds the quiet hits that a blunt one does not.
#[test]
fn the_sensitivity_is_how_small_a_rise_may_be() {
    let mut session = session();
    let directory = directory("sensitivity");
    let path = format!("{directory}/sound.wav");
    // A loud hit and then a quiet one.
    let mut samples: Vec<f32> = Vec::new();
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    samples.extend(std::iter::repeat_n(1.0_f32, 12_000));
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    samples.extend(std::iter::repeat_n(0.05_f32, 12_000));
    let bytes = craftkit_wav::write(1, 48_000, &samples);
    std::fs::write(&path, bytes).expect("the sound is written");
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 33_600, "source": source }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let blunt = session
        .execute("sound.clip.separate.transients", &json!({ "clip": clip, "sensitivity": 0.0 }))
        .expect("the clip is separated");
    assert_eq!(blunt.data["transients"], 1, "the blunt detector hears the loud hit: {blunt:?}");
}
