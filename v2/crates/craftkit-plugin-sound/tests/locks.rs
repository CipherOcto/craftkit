//! The **locks** on a clip: what may not change, and what may not
//! move.
//!
//! The tree's sound editor has two locks on a clip, and they are two
//! different claims: the **edit lock** says the sound inside the clip
//! is finished, and the **time lock** says the clip stays where it is.

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

/// A **track** with two clips on it, and the ids of the two.
fn two_clips(session: &mut Session) -> (u64, Vec<u64>) {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let mut clips = Vec::new();
    for start in [0, 1000] {
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": start, "length": 500 }))
            .expect("the clip is added");
        let id = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.clip")
            .last()
            .map(|node| node.id)
            .expect("the clip is there");
        clips.push(id);
    }
    (track, clips)
}

/// Where the clips of a track **start**.
fn starts_of(session: &Session, track: u64) -> Vec<i64> {
    let document = session.document();
    let parent_of = |id: u64| {
        document.walk().find(|node| node.children.iter().any(|child| child.id == id)).map(|node| node.id)
    };
    let mut starts: Vec<(i64, u64)> = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        let Some(owner) = parent_of(node.id) else { continue };
        let owner = if owner == track { owner } else { parent_of(owner).unwrap_or(owner) };
        if owner != track {
            continue;
        }
        let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        starts.push((start, node.id));
    }
    starts.sort();
    starts.into_iter().map(|(start, _)| start).collect()
}

/// A clip's **edit lock** may be set and taken off, and the two locks
/// are two different locks.
#[test]
fn the_two_locks_are_two_locks() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clips[0], "locked": true }))
        .expect("the edit lock is set");
    session
        .execute("sound.clip.lock.time", &json!({ "clip": clips[0], "locked": true }))
        .expect("the time lock is set");
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("edit_locked").and_then(|locked| locked.as_bool()), Some(true));
    assert_eq!(node.data.get("time_locked").and_then(|locked| locked.as_bool()), Some(true));
    // Taking one off leaves the other.
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clips[0], "locked": false }))
        .expect("the edit lock is taken off");
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("edit_locked").and_then(|locked| locked.as_bool()), Some(false));
    assert_eq!(node.data.get("time_locked").and_then(|locked| locked.as_bool()), Some(true));
}

/// A lock that is not named is **toggled**.
#[test]
fn a_lock_that_is_not_named_is_toggled() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session.execute("sound.clip.lock.edit", &json!({ "clip": clips[0] })).expect("the clip is locked");
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("edit_locked").and_then(|locked| locked.as_bool()), Some(true));
    session.execute("sound.clip.lock.edit", &json!({ "clip": clips[0] })).expect("the clip is unlocked");
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("edit_locked").and_then(|locked| locked.as_bool()), Some(false));
}

/// A clip whose **content** is locked is finished: it is not trimmed,
/// not faded and not stretched.
#[test]
fn a_locked_sound_is_finished() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clips[0], "locked": true }))
        .expect("the edit lock is set");
    let refused = session
        .execute("sound.clip.trim", &json!({ "clip": clips[0], "from": 100, "to": 400 }))
        .expect_err("the trim is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    let refused = session
        .execute("sound.clip.fade", &json!({ "clip": clips[0], "fade_in": 100 }))
        .expect_err("the fade is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    let refused = session
        .execute("sound.clip.stretch", &json!({ "clip": clips[0], "stretch": 2.0 }))
        .expect_err("the stretch is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    // The clip is where it was.
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("start").and_then(|start| start.as_i64()), Some(0));
    assert_eq!(node.data.get("length").and_then(|length| length.as_i64()), Some(500));
}

/// A clip whose **place** is locked stays where it is: it is not
/// spaced and not snapped.
#[test]
fn a_locked_place_stays_where_it_is() {
    let mut session = session();
    let (track, clips) = two_clips(&mut session);
    session
        .execute("sound.clip.lock.time", &json!({ "clip": clips[1], "locked": true }))
        .expect("the time lock is set");
    let refused = session
        .execute("sound.clip.space", &json!({ "clips": clips, "gap": 0 }))
        .expect_err("the spacing is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    let refused = session
        .execute("sound.clip.snap", &json!({ "clips": [clips[1]], "direction": "previous" }))
        .expect_err("the snap is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    assert_eq!(starts_of(&session, track), vec![0, 1000], "the clips are where they were");
}

/// A locked clip is a **destination** and not an obstacle: another
/// clip may still be snapped against it, because the locked clip stays
/// where it is and the other one moves.
#[test]
fn a_locked_clip_is_a_destination() {
    let mut session = session();
    let (track, clips) = two_clips(&mut session);
    session
        .execute("sound.clip.lock.time", &json!({ "clip": clips[1], "locked": true }))
        .expect("the time lock is set");
    session
        .execute("sound.clip.snap", &json!({ "clips": [clips[0]], "direction": "next" }))
        .expect("the clip is snapped against the locked one");
    assert_eq!(starts_of(&session, track), vec![500, 1000], "the other clip moved; the locked one did not");
}

/// A clip whose **content** is locked does not move either: a clip
/// that moves is a clip that changed.
#[test]
fn a_locked_sound_does_not_move_either() {
    let mut session = session();
    let (track, clips) = two_clips(&mut session);
    session
        .execute("sound.clip.lock.edit", &json!({ "clip": clips[0], "locked": true }))
        .expect("the edit lock is set");
    let refused = session
        .execute("sound.clip.space", &json!({ "clips": clips, "gap": 0 }))
        .expect_err("the spacing is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    assert_eq!(starts_of(&session, track), vec![0, 1000], "the clips are where they were");
}

/// A clip that is **not** locked is trimmed, faded and stretched as
/// ever — the lock is a lock and not a dead clip.
#[test]
fn an_unlocked_clip_is_changed_as_ever() {
    let mut session = session();
    let (_, clips) = two_clips(&mut session);
    session
        .execute("sound.clip.trim", &json!({ "clip": clips[0], "from": 100, "to": 400 }))
        .expect("the clip is trimmed");
    let node = session.document().node(clips[0]).expect("the clip is there");
    assert_eq!(node.data.get("start").and_then(|start| start.as_i64()), Some(100));
    assert_eq!(node.data.get("length").and_then(|length| length.as_i64()), Some(300));
}
