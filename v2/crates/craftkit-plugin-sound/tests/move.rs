//! The **moving** of a clip, and the **end of its material**.

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
fn new_session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A session with a track, a source of that many frames and a clip of
/// that many samples.
fn a_clip(session: &mut Session, length: i64, frames: i64) -> (u64, u64) {
    session
        .execute("sound.source.add", &json!({ "path": "kit/kick.wav", "channels": 1, "frames": frames }))
        .expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": length, "source": source }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The place of the clip on the timeline.
fn start_of(session: &Session, clip: u64) -> i64 {
    session
        .document()
        .node(clip)
        .and_then(|node| node.data.get("start").and_then(|start| start.as_i64()))
        .unwrap_or(0)
}

/// A clip is **moved** later, and its content goes with it.
#[test]
fn a_clip_is_moved() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session, 24_000, 48_000);
    session
        .execute("sound.clip.move", &json!({ "clips": [clip], "delta": 1_000 }))
        .expect("the clip is moved");
    assert_eq!(start_of(&session, clip), 1_000, "the clip is a second later");
    session
        .execute("sound.clip.move", &json!({ "clips": [clip], "delta": -500 }))
        .expect("the clip is moved back");
    assert_eq!(start_of(&session, clip), 500, "and half a second earlier");
}

/// A **locked** clip does not move: the edit lock and the time lock
/// both hold it.
#[test]
fn a_locked_clip_does_not_move() {
    for lock in ["sound.clip.lock.edit", "sound.clip.lock.time"] {
        let mut session = new_session();
        let (_, clip) = a_clip(&mut session, 24_000, 48_000);
        session.execute(lock, &json!({ "clip": clip })).expect("the clip is locked");
        let moved = session
            .execute("sound.clip.move", &json!({ "clips": [clip], "delta": 1_000 }))
            .expect("the move is made");
        assert_eq!(moved.data["moved"], 0, "{lock} holds the clip");
        assert_eq!(start_of(&session, clip), 0, "and the clip did not move");
    }
}

/// A move that would take a clip **before the beginning** is refused,
/// and no clip moves.
#[test]
fn a_move_before_the_beginning_is_refused() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session, 24_000, 48_000);
    let refused = session.execute("sound.clip.move", &json!({ "clips": [clip], "delta": -1_000 }));
    assert!(refused.is_err(), "the clip would go before the beginning");
    assert_eq!(start_of(&session, clip), 0, "and the clip did not move");
}

/// A clip that reaches past the end of its **material** is trimmed to
/// the end of it: the part that is past the end is silence, and a clip
/// that is half silence says something false about its own sound.
#[test]
fn a_clip_is_trimmed_to_its_material() {
    let mut session = new_session();
    // The clip plays twice as long as the material holds.
    let (_, clip) = a_clip(&mut session, 96_000, 48_000);
    let trimmed =
        session.execute("sound.clip.trim.to.file", &json!({ "clips": [clip] })).expect("the clip is trimmed");
    assert_eq!(trimmed.data["trimmed"], 1, "the clip was trimmed");
    let node = session.document().node(clip).expect("the clip is there");
    assert_eq!(node.data["length"], json!(48_000), "the clip is as long as the material");
}

/// A clip that is **already** inside its material is not trimmed, and
/// a clip that plays no sound at all is not either.
#[test]
fn a_clip_inside_its_material_is_not_trimmed() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session, 24_000, 48_000);
    let trimmed =
        session.execute("sound.clip.trim.to.file", &json!({ "clips": [clip] })).expect("the clip is trimmed");
    assert_eq!(trimmed.data["trimmed"], 0, "the clip was already inside");
    let node = session.document().node(clip).expect("the clip is there");
    assert_eq!(node.data["length"], json!(24_000), "and its length is its own");
}
