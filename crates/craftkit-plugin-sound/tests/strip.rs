//! The **stripping of silence**: the quiet parts of a clip are thrown
//! away and the loud parts are kept.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-strip-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A sound of two loud parts with a silence between them.
fn two_parts() -> Vec<f32> {
    let mut samples: Vec<f32> = Vec::new();
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    samples.extend(std::iter::repeat_n(1.0_f32, 12_000));
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    samples.extend(std::iter::repeat_n(1.0_f32, 12_000));
    samples
}

/// A session with a track, a source of that sound and a clip that
/// plays all of it.
fn a_clip(session: &mut Session, path: &str) -> (u64, u64) {
    let bytes = craftkit_wav::write(1, 48_000, &two_parts());
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

/// The **silence** is stripped out: the quiet piece between the two
/// loud parts is thrown away, and the two loud parts are kept.
#[test]
fn the_silence_is_stripped_out() {
    let mut session = session();
    let directory = directory("strip");
    let path = format!("{directory}/sound.wav");
    let (_, clip) = a_clip(&mut session, &path);
    let stripped = session
        .execute("sound.clip.strip.silence", &json!({ "clip": clip }))
        .expect("the silence is stripped");
    assert_eq!(stripped.data["kept"], 2, "the two loud parts are kept: {stripped:?}");
    // Two quiet pieces are thrown away: the silence before the first
    // loud part and the silence between the two.
    assert_eq!(stripped.data["removed"], 2, "and the quiet ones are thrown away");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 2, "the clip is in two: {clips:?}");
    // The first part starts at the first loud part and the second at
    // the second one — a little either way, because the level of a
    // frame is read over a window.
    assert!(clips[0].0 >= 3_000 && clips[0].0 <= 5_000, "the first part: {clips:?}");
    assert!(clips[1].0 >= 20_000 && clips[1].0 <= 21_800, "and the second: {clips:?}");
}

/// The kept parts are **named** from the auto rename scheme, in time
/// order, and the parts that were thrown away take no number.
#[test]
fn the_kept_parts_are_named_from_the_scheme() {
    let mut session = session();
    let directory = directory("named");
    let path = format!("{directory}/sound.wav");
    let (_, clip) = a_clip(&mut session, &path);
    session
        .execute("sound.clip.autoname", &json!({ "clips": [clip], "name": "Hit" }))
        .expect("the scheme is set");
    session.execute("sound.clip.strip.silence", &json!({ "clip": clip })).expect("the silence is stripped");
    let names: Vec<String> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| node.name.clone())
        .collect();
    assert!(names.contains(&"Hit1".to_string()), "the first part: {names:?}");
    assert!(names.contains(&"Hit2".to_string()), "and the second: {names:?}");
    assert_eq!(names.len(), 2, "and no name is left over: {names:?}");
}

/// A clip that is **silent** has nothing to keep, and it is refused:
/// a clip that is thrown away is a clip nobody asked for.
#[test]
fn a_silent_clip_is_refused() {
    let mut session = session();
    let directory = directory("silent");
    let path = format!("{directory}/sound.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![0.0_f32; 48_000]);
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let refused = session.execute("sound.clip.strip.silence", &json!({ "clip": clip }));
    assert!(refused.is_err(), "there is nothing to keep");
    assert_eq!(clips_of(&session).len(), 1, "and the clip is still there");
}

/// A clip that plays **no sound** has no silence in it, and it is
/// refused.
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
    let refused = session.execute("sound.clip.strip.silence", &json!({ "clip": clip }));
    assert!(refused.is_err(), "there is no sound and no silence");
}
