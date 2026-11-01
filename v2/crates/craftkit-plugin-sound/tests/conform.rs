//! The **conform** of a clip: making the material fit the tempo.
//!
//! The tree's sound editor conforms a clip to the tempo of the
//! document. Here the tempo the material is at is a field of the clip,
//! and the beat follows the audio when the clip is conformed.

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

/// A track with a clip that has a **source**, and the id of the clip.
fn one_clip(session: &mut Session) -> u64 {
    session
        .execute("sound.source.add", &json!({ "path": "/tmp/craftkit-v2-conform.wav" }))
        .expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000, "source": source }))
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// A number from the clip's payload.
fn number_of(session: &Session, clip: u64, key: &str) -> Option<f64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get(key).and_then(|value| value.as_f64())
}

/// The clip is **conformed**: the tempo of the material is remembered
/// and the clip is as it is.
#[test]
fn the_clip_is_conformed() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.conform", &json!({ "clip": clip })).expect("the clip is conformed");
    assert_eq!(number_of(&session, clip, "source_bpm"), Some(120.0), "the material is at 120");
    assert_eq!(number_of(&session, clip, "stretch"), Some(1.0), "and the clip is not stretched");
    assert_eq!(number_of(&session, clip, "length"), Some(48_000.0), "and it is as long as it was");
}

/// The material **fits the tempo**: a material at 120 in a document at
/// 60 is played twice as slowly, and the clip is twice as long.
#[test]
fn the_material_fits_the_tempo() {
    let mut session = session();
    let clip = one_clip(&mut session);
    // The clip is conformed while the document is at 120, so the
    // material is remembered as a material at 120.
    session.execute("sound.clip.conform", &json!({ "clip": clip })).expect("the clip is conformed");
    session.execute("sound.tempo.set", &json!({ "bpm": 60.0 })).expect("the tempo is set");
    session.execute("sound.clip.conform", &json!({ "clip": clip })).expect("the clip is conformed");
    assert_eq!(number_of(&session, clip, "stretch"), Some(2.0), "the material is played twice as slowly");
    assert_eq!(number_of(&session, clip, "length"), Some(96_000.0), "and the clip is twice as long");
}

/// The tempo of the material may be **given**, and it is remembered.
#[test]
fn the_tempo_of_the_material_may_be_given() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session
        .execute("sound.clip.conform", &json!({ "clip": clip, "source_bpm": 60.0 }))
        .expect("the clip is conformed");
    assert_eq!(number_of(&session, clip, "source_bpm"), Some(60.0), "the material is at 60");
    assert_eq!(number_of(&session, clip, "stretch"), Some(0.5), "and the clip is played twice as fast");
    assert_eq!(number_of(&session, clip, "length"), Some(24_000.0), "and it is half as long");
}

/// The **beat** follows the audio: the clip's timeline is what changed,
/// and the beat is a place in it.
#[test]
fn the_beat_follows_the_audio() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 4800 })).expect("the beat is set");
    session.execute("sound.clip.conform", &json!({ "clip": clip })).expect("the clip is conformed");
    session.execute("sound.tempo.set", &json!({ "bpm": 60.0 })).expect("the tempo is set");
    session.execute("sound.clip.conform", &json!({ "clip": clip })).expect("the clip is conformed");
    // The clip is twice as long, so the beat is twice as far in.
    assert_eq!(number_of(&session, clip, "sync"), Some(9600.0), "the beat moved with the audio");
}

/// A clip whose **content** is locked is finished, and a conform
/// changes the sound.
#[test]
fn a_locked_clip_is_not_conformed() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clip, "locked": true }))
        .expect("the clip is locked");
    let refused = session
        .execute("sound.clip.conform", &json!({ "clip": clip }))
        .expect_err("a locked clip is not conformed");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
}

/// A clip with **no sound** has nothing to conform.
#[test]
fn a_clip_with_no_sound_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 500 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let refused = session
        .execute("sound.clip.conform", &json!({ "clip": clip }))
        .expect_err("there is no sound to conform");
    assert!(refused.to_string().contains("no sound"), "the error says why: {refused}");
}

/// A tempo of the material outside the range is **refused**.
#[test]
fn a_tempo_outside_the_range_is_refused() {
    let mut session = session();
    let clip = one_clip(&mut session);
    let refused = session
        .execute("sound.clip.conform", &json!({ "clip": clip, "source_bpm": 2000.0 }))
        .expect_err("two thousand beats a minute is not a tempo");
    assert!(refused.to_string().contains("beats a minute"), "the error says why: {refused}");
    assert_eq!(number_of(&session, clip, "source_bpm"), None, "the clip does not know a tempo");
}

/// Only a **clip** is conformed.
#[test]
fn only_a_clip_is_conformed() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let refused =
        session.execute("sound.clip.conform", &json!({ "clip": track })).expect_err("a track is not a clip");
    assert!(refused.to_string().contains("not a clip"), "the error says why: {refused}");
}
