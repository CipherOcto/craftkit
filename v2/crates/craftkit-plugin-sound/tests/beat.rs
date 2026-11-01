//! The **beat** inside a clip: where the beat falls within it.
//!
//! The tree's sound editor has a sync point — a place inside a clip
//! where the beat falls, and the thing a clip is lined up by. The tree
//! keeps it as a number where zero means *none*, and this tree keeps
//! it as an option: no beat, or a beat at some sample in the clip.

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

/// A **track** with one clip on it, and the ids of the two.
fn one_clip(session: &mut Session, start: i64, length: i64) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": start, "length": length }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The **beat** of a clip, as the document holds it.
fn the_beat(session: &Session, clip: u64) -> Option<i64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("sync").and_then(|sync| sync.as_i64())
}

/// A beat may be **set**, and it is an offset in the clip's own
/// samples.
#[test]
fn a_beat_is_set_where_the_caller_says() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 1000, 500);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 1200 })).expect("the beat is set");
    assert_eq!(the_beat(&session, clip), Some(200), "the beat is 200 samples into the clip");
}

/// A beat of **zero** is a beat at the clip's head, and it is not the
/// same as no beat at all.
///
/// This is the difference from the tree: the tree's sync point is a
/// number where zero means *none*, so a clip whose beat is at its
/// first sample cannot say so.
#[test]
fn a_beat_at_the_head_is_not_no_beat() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 0, 500);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 0 })).expect("the beat is set");
    assert_eq!(the_beat(&session, clip), Some(0), "the beat is at the clip's head");
    session.execute("sound.clip.sync.remove", &json!({ "clip": clip })).expect("the beat is removed");
    assert_eq!(the_beat(&session, clip), None, "the clip has no beat");
}

/// A beat **outside** the clip is refused.
///
/// The tree's editor **clamps** the place into the clip, so a beat
/// asked for outside lands at the edge without saying so. Here it is
/// refused: a beat that was moved without being asked for is a beat the
/// caller did not want.
#[test]
fn a_beat_outside_the_clip_is_refused() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 1000, 500);
    let refused = session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 900 }));
    assert!(refused.is_err(), "the beat outside the clip is refused");
    assert_eq!(the_beat(&session, clip), None, "the clip still has no beat");
}

/// A beat **follows the audio**: a trim of the head brings the beat
/// nearer the head by the number of samples that were cut.
///
/// The audio does not move when the clip is trimmed — the trim takes
/// the first samples away and leaves the rest where it was — so the
/// beat moves with it.
#[test]
fn a_beat_follows_the_audio_of_a_trim() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 0, 1000);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 800 })).expect("the beat is set");
    session
        .execute("sound.clip.trim", &json!({ "clip": clip, "from": 200, "to": 1000 }))
        .expect("the clip is trimmed");
    assert_eq!(the_beat(&session, clip), Some(600), "the beat moved with the audio");
}

/// A beat the trim left **outside** the clip is gone: a beat past the
/// end of the clip is not a beat of that clip.
#[test]
fn a_beat_the_trim_left_outside_is_gone() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 0, 1000);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 300 })).expect("the beat is set");
    session
        .execute("sound.clip.trim", &json!({ "clip": clip, "from": 500, "to": 1000 }))
        .expect("the clip is trimmed");
    assert_eq!(the_beat(&session, clip), None, "the beat is gone");
}

/// The beat of a **copy** of a clip is the copy's own, and a copy cut
/// short loses a beat that is past the cut.
#[test]
fn a_copy_cut_short_loses_a_beat_past_the_cut() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 0, 100);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 80 })).expect("the beat is set");
    session.execute("sound.clip.loop", &json!({ "clip": clip, "length": 150 })).expect("the clip is looped");
    let mut beats: Vec<Option<i64>> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| node.data.get("sync").and_then(|sync| sync.as_i64()))
        .collect();
    beats.sort();
    assert_eq!(beats, vec![None, Some(80)], "the short copy has no beat");
}

/// The beat is **read** back with the place it falls in the bar, and
/// the place is worked out from the tempo map rather than kept.
#[test]
fn a_beat_is_read_with_the_place_it_falls_in() {
    let mut session = session();
    session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
    session.execute("sound.tempo.set", &json!({ "tick": 0, "bpm": 120.0 })).expect("the tempo is set");
    session
        .execute("sound.meter.set", &json!({ "tick": 0, "numerator": 4, "denominator": 4 }))
        .expect("the meter is set");
    let (_, clip) = one_clip(&mut session, 0, 48_000);
    session.execute("sound.clip.sync.set", &json!({ "clip": clip, "at": 24_000 })).expect("the beat is set");
    let outcome =
        session.execute("sound.clip.sync.read", &json!({ "clip": clip })).expect("the beat is read");
    assert_eq!(outcome.data["sync"], json!(24_000), "the beat is 24000 samples in");
    assert_eq!(outcome.data["at"], json!(24_000), "the beat falls at 24000 on the timeline");
    // 120 bpm at 48 kHz: a beat is half a second, so 24000 samples is
    // the second beat of the first bar.
    assert_eq!(outcome.data["position"], json!("1.2.0"), "the beat falls on the second beat of the bar");
}

/// A clip with **no beat** reads as no beat, and taking the beat out of
/// a clip that has none changes nothing — and leaves no undo step.
#[test]
fn a_clip_with_no_beat_says_so() {
    let mut session = session();
    let (_, clip) = one_clip(&mut session, 0, 100);
    let outcome =
        session.execute("sound.clip.sync.read", &json!({ "clip": clip })).expect("the beat is read");
    assert_eq!(outcome.data["sync"], json!(null), "the clip has no beat");
    let removed =
        session.execute("sound.clip.sync.remove", &json!({ "clip": clip })).expect("the beat is taken away");
    assert!(!removed.changed, "nothing changed, and there is no undo step");
}
