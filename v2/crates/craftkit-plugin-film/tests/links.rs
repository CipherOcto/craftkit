//! The **link** of a clip.
//!
//! Two clips may be linked: they are the same thing on two tracks — a
//! picture and its sound — and they move as one.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **bin**, an **item** in it, and **two tracks** in the item.
fn two_tracks(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 200 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "picture", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let picture = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let sound = outcome.data["id"].as_u64().expect("the track has an id");
    (item, picture, sound)
}

/// A **clip** on a track.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// Where a clip **starts**.
fn start_of(session: &Session, clip: u64) -> u64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["start"].as_u64().expect("the clip has a start")
}

/// The **link** of a clip.
fn link_of(session: &Session, clip: u64) -> Option<u64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["link"].as_u64()
}

/// Two clips are linked, and the link is kept with them.
#[test]
fn two_clips_are_linked() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    assert_eq!(link_of(&session, first), Some(first.min(second)), "the group is named by its first clip");
    assert_eq!(link_of(&session, second), Some(first.min(second)), "and both are in it");
}

/// A **move** of one clip moves its partner by the same offset.
#[test]
fn a_move_moves_the_partner() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session
        .execute("film.clip.move", &json!({ "clip": first, "track": picture, "start": 20 }))
        .expect("the clip is moved");
    assert_eq!(start_of(&session, first), 20, "the clip is moved");
    assert_eq!(start_of(&session, second), 20, "and its partner with it");
}

/// A partner on a **locked** track stays where it is.
#[test]
fn a_partner_on_a_locked_track_stays() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session
        .execute("film.track.lock", &json!({ "track": sound, "locked": true }))
        .expect("the track is locked");
    session
        .execute("film.clip.move", &json!({ "clip": first, "track": picture, "start": 20 }))
        .expect("the clip is moved");
    assert_eq!(start_of(&session, first), 20, "the clip is moved");
    assert_eq!(start_of(&session, second), 0, "and the partner stays");
}

/// A clip that is **unlinked** leaves the others linked.
#[test]
fn an_unlinked_clip_leaves_the_others() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    let third = a_clip(&mut session, sound, item, 60, 40);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": third }))
        .expect("the clips are linked");
    session.execute("film.clip.unlink", &json!({ "clip": third })).expect("the clip is unlinked");
    assert_eq!(link_of(&session, third), None, "the clip is on its own");
    assert!(link_of(&session, first).is_some(), "the others are still linked");
    assert!(link_of(&session, second).is_some(), "and the others");
}

/// A partner left **alone** in its link is unlinked too: a link with
/// nothing to follow reads as if it were something.
#[test]
fn a_partner_left_alone_is_unlinked() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session.execute("film.clip.unlink", &json!({ "clip": second })).expect("the clip is unlinked");
    assert_eq!(link_of(&session, second), None, "the clip is on its own");
    assert_eq!(link_of(&session, first), None, "and the partner that is left alone");
}

/// A clip that is **not linked** cannot be unlinked.
#[test]
fn a_clip_that_is_not_linked_cannot_be_unlinked() {
    let mut session = session();
    let (item, picture, _) = two_tracks(&mut session);
    let clip = a_clip(&mut session, picture, item, 0, 50);
    let outcome = session.execute("film.clip.unlink", &json!({ "clip": clip }));
    assert!(outcome.is_err(), "the clip that is not linked is refused");
}

/// A clip may not be linked to **itself**.
#[test]
fn a_clip_may_not_be_linked_to_itself() {
    let mut session = session();
    let (item, picture, _) = two_tracks(&mut session);
    let clip = a_clip(&mut session, picture, item, 0, 50);
    let outcome = session.execute("film.clip.link", &json!({ "clip": clip, "with": clip }));
    assert!(outcome.is_err(), "the clip linked to itself is refused");
}

/// Two clips in **different sequences** may not be linked.
#[test]
fn two_clips_in_different_sequences_are_refused() {
    let mut session = session();
    let (item, picture, _) = two_tracks(&mut session);
    let clip = a_clip(&mut session, picture, item, 0, 50);
    let outcome =
        session.execute("film.bin.add", &json!({ "name": "another bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "other media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let other_item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "other", "sequence": other_item, "kind": "video" }))
        .expect("the track is added");
    let other_track = outcome.data["id"].as_u64().expect("the track has an id");
    let other = a_clip(&mut session, other_track, other_item, 0, 50);
    let outcome = session.execute("film.clip.link", &json!({ "clip": clip, "with": other }));
    assert!(outcome.is_err(), "the clips of two sequences are refused");
}

/// The **trim** of a linked clip trims the **group**: the tree trims
/// the group as one edit.
#[test]
fn the_trim_of_a_linked_clip_trims_the_group() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "out", "delta": 5 }))
        .expect("the group is trimmed");
    let duration_of = |session: &Session, clip: u64| {
        let node = session.document().node(clip).expect("the clip is there");
        node.data["duration"].as_u64().expect("the clip has a duration")
    };
    assert_eq!(duration_of(&session, first), 55, "the clip is trimmed");
    assert_eq!(duration_of(&session, second), 55, "and its partner with it");
}

/// The **split** of a linked clip splits the **group**: the tree
/// splits the group as one edit.
#[test]
fn the_split_of_a_linked_clip_splits_the_group() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session.execute("film.clip.split", &json!({ "clip": first, "at": 10 })).expect("the group is split");
    let duration_of = |session: &Session, clip: u64| {
        let node = session.document().node(clip).expect("the clip is there");
        node.data["duration"].as_u64().expect("the clip has a duration")
    };
    assert_eq!(duration_of(&session, first), 10, "the clip is split");
    assert_eq!(duration_of(&session, second), 10, "and its partner with it");
}
