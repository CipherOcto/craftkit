//! The **modes** of the strip of silence, and its **preview**.
//!
//! The tree's editor strips silence in three ways: it **strips** the
//! silence and keeps the loud parts, it **extracts** the loud parts and
//! keeps the silence, and it **separates** the clip at the edges of the
//! loud parts and keeps everything. It also has a **preview**: it finds
//! the silence and says where the parts are, without changing
//! anything.

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
        craftkit_scratch::dir(&format!("craftkit-v2-stripmode-{name}")).expect("the directory is made");
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
/// plays all of it, and the clip's id.
fn a_clip(session: &mut Session, name: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/sound.wav");
    let bytes = craftkit_wav::write(1, 48_000, &two_parts());
    std::fs::write(&path, bytes).expect("the sound is written");
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 33_600, "source": source }))
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
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

/// The **extract** keeps the quiet parts and throws the loud ones
/// away: the silence is what is left.
#[test]
fn the_loud_parts_are_extracted() {
    let mut session = session();
    let clip = a_clip(&mut session, "extract");
    let extracted = session
        .execute("sound.clip.strip.silence", &json!({ "clip": clip, "mode": "extract" }))
        .expect("the sound is extracted");
    assert_eq!(extracted.data["kept"], 2, "the two quiet parts are kept: {extracted:?}");
    assert_eq!(extracted.data["removed"], 2, "and the loud ones are thrown away");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 2, "the clip is in two: {clips:?}");
    // The first piece is the silence **before** the first loud part,
    // and the second is the silence **between** the two.
    assert!(clips[0].1 <= 4_800, "the silence at the head: {clips:?}");
    assert!(clips[1].0 >= 16_000, "and the silence between: {clips:?}");
}

/// The **separate** cuts the clip at the edges of the loud parts and
/// keeps everything: nothing is thrown away.
#[test]
fn the_clip_is_separated_at_the_edges_of_the_loud_parts() {
    let mut session = session();
    let clip = a_clip(&mut session, "separate");
    let separated = session
        .execute("sound.clip.strip.silence", &json!({ "clip": clip, "mode": "separate" }))
        .expect("the clip is separated");
    assert_eq!(separated.data["removed"], 0, "nothing is thrown away: {separated:?}");
    // The clip is cut at the **edges** of the two loud parts, and the
    // last edge is the end of the clip, so there are three cuts and
    // four pieces.
    assert_eq!(separated.data["kept"], 4, "and the four pieces are kept: {separated:?}");
    let clips = clips_of(&session);
    assert_eq!(clips.len(), 4, "the clip is in four: {clips:?}");
    // And the pieces are the whole of the clip: nothing was lost.
    let total: i64 = clips.iter().map(|(_, length)| *length).sum();
    assert_eq!(total, 33_600, "the clip is all of it still: {clips:?}");
}

/// A mode this battery does not know is **refused**, and nothing is
/// changed.
#[test]
fn an_unknown_mode_is_refused() {
    let mut session = session();
    let clip = a_clip(&mut session, "unknown");
    let refused = session.execute("sound.clip.strip.silence", &json!({ "clip": clip, "mode": "throw" }));
    assert!(refused.is_err(), "there is no mode called `throw`");
    assert_eq!(clips_of(&session).len(), 1, "and the clip is still there");
}

/// The **preview** says where the loud parts are, and it changes
/// nothing.
#[test]
fn the_preview_says_where_the_loud_parts_are() {
    let mut session = session();
    let clip = a_clip(&mut session, "preview");
    let preview = session
        .execute("sound.clip.strip.silence.read", &json!({ "clip": clip }))
        .expect("the clip is looked at");
    let ranges = preview.data["ranges"].as_array().expect("the ranges are there");
    assert_eq!(ranges.len(), 2, "there are two loud parts: {preview:?}");
    // The first loud part is a little after the head of the sound.
    let first = ranges[0].as_array().expect("a range is a pair");
    assert!(first[0].as_i64().unwrap_or(0) >= 3_000, "the first part: {preview:?}");
    // And the document is **not** changed.
    assert_eq!(clips_of(&session).len(), 1, "and the clip is still one: {:?}", clips_of(&session));
}
