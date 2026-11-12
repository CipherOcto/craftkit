//! The **fades** of a range of a track.
//!
//! The tree's `create_fades` over a selection: a clip that starts
//! inside the range fades in and a clip that ends inside it fades out.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-rangefades-{name}-{unique}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
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

/// The **fade in** of a clip: its length and its shape.
fn fade_in_of(session: &Session, clip: u64) -> (i64, String) {
    let node = session.document().node(clip).expect("the clip is there");
    let len = node.data["fade_in"]["len"].as_i64().unwrap_or(0);
    let shape = node.data["fade_in"]["shape"].as_str().unwrap_or_default().to_string();
    (len, shape)
}

/// The **fade out** of a clip: its length and its shape.
fn fade_out_of(session: &Session, clip: u64) -> (i64, String) {
    let node = session.document().node(clip).expect("the clip is there");
    let len = node.data["fade_out"]["len"].as_i64().unwrap_or(0);
    let shape = node.data["fade_out"]["shape"].as_str().unwrap_or_default().to_string();
    (len, shape)
}

/// A range that covers the **start** of a clip fades it in.
#[test]
fn a_range_that_covers_a_start_fades_the_clip_in() {
    let mut session = session();
    let (track, clip) = a_track(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 0, "to": 4_800 }))
        .expect("the range is faded");
    let (len, shape) = fade_in_of(&session, clip);
    assert_eq!(len, 4_800, "the fade in is the rest of the range");
    assert_eq!(shape, "equal_power", "and the tree's default shape");
}

/// A range that covers the **end** of a clip fades it out.
#[test]
fn a_range_that_covers_an_end_fades_the_clip_out() {
    let mut session = session();
    let (track, clip) = a_track(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 43_200, "to": 48_000 }))
        .expect("the range is faded");
    let (len, shape) = fade_out_of(&session, clip);
    assert_eq!(len, 4_800, "the fade out is the beginning of the range");
    assert_eq!(shape, "equal_power", "and the tree's default shape");
}

/// The **shape** may be the tree's other shapes.
#[test]
fn the_shape_may_be_the_trees_other_shapes() {
    let mut session = session();
    let (track, clip) = a_track(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 0, "to": 4_800, "shape": "s_curve" }))
        .expect("the range is faded");
    let (_, shape) = fade_in_of(&session, clip);
    assert_eq!(shape, "s_curve", "the curve is the one that was asked for");
}

/// A shape that is **not a shape** is refused.
#[test]
fn a_shape_that_is_not_a_shape_is_refused() {
    let mut session = session();
    let (track, _clip) = a_track(&mut session, "the track");
    let error = session
        .execute("sound.track.fade", &json!({ "track": track, "from": 0, "to": 4_800, "shape": "spring" }))
        .expect_err("the shape is refused");
    assert!(error.to_string().contains("no fade shape called"), "the refusal says why: {error}");
}

/// A range that covers **neither** a start nor an end is refused: the
/// tree says the selection must cover one or the other.
#[test]
fn a_range_that_covers_nothing_is_refused() {
    let mut session = session();
    let (track, _clip) = a_track(&mut session, "the track");
    let error = session
        .execute("sound.track.fade", &json!({ "track": track, "from": 10_000, "to": 20_000 }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("neither the start nor the end"), "the refusal says why: {error}");
}

/// An **empty** range is refused.
#[test]
fn an_empty_range_is_refused() {
    let mut session = session();
    let (track, _clip) = a_track(&mut session, "the track");
    let error = session
        .execute("sound.track.fade", &json!({ "track": track, "from": 100, "to": 100 }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("the range is empty"), "the refusal says why: {error}");
}

/// The fades of a range may be **taken away**.
#[test]
fn the_fades_of_a_range_may_be_taken_away() {
    let mut session = session();
    let (track, clip) = a_track(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 0, "to": 4_800 }))
        .expect("the range is faded");
    assert_eq!(fade_in_of(&session, clip).0, 4_800, "the clip is faded in");
    session
        .execute("sound.track.fade.delete", &json!({ "track": track, "from": 0, "to": 4_800 }))
        .expect("the fades are taken away");
    assert_eq!(fade_in_of(&session, clip).0, 0, "and the fade in is gone");
    assert_eq!(fade_out_of(&session, clip).0, 0, "and the fade out too");
}

/// A fade **in** and a fade **out** may both be made, and both taken
/// away.
#[test]
fn a_fade_in_and_a_fade_out_may_both_be_made() {
    let mut session = session();
    let (track, clip) = a_track(&mut session, "the track");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 0, "to": 4_800 }))
        .expect("the range is faded");
    session
        .execute("sound.track.fade", &json!({ "track": track, "from": 43_200, "to": 48_000 }))
        .expect("the range is faded");
    assert_eq!(fade_in_of(&session, clip).0, 4_800, "the clip is faded in");
    assert_eq!(fade_out_of(&session, clip).0, 4_800, "and faded out");
    session
        .execute("sound.track.fade.delete", &json!({ "track": track, "from": 0, "to": 48_000 }))
        .expect("the fades are taken away");
    assert_eq!(fade_in_of(&session, clip).0, 0, "and the fade in is gone");
    assert_eq!(fade_out_of(&session, clip).0, 0, "and the fade out too");
}

/// A track that is **not there** is refused.
#[test]
fn a_track_that_is_not_there_is_refused() {
    let mut session = session();
    let (_track, _clip) = a_track(&mut session, "the track");
    let error = session
        .execute("sound.track.fade", &json!({ "track": 1_000_000, "from": 0, "to": 4_800 }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("there is no node"), "the refusal says why: {error}");
}
