//! The **split** of a linked group.
//!
//! The tree splits a linked group as **one edit**: the clip and its
//! partners are cut at the same place, and the right-hand pieces get a
//! **new** link, so that they are linked to each other and not to the
//! left-hand ones.

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

/// The **duration** of a clip.
fn duration_of(session: &Session, clip: u64) -> u64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["duration"].as_u64().expect("the clip has a duration")
}

/// The **link** of a clip.
fn link_of(session: &Session, clip: u64) -> Option<u64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["link"].as_u64()
}

/// The **clips** on a track.
fn clips_on(session: &Session, track: u64) -> Vec<u64> {
    let node = session.document().node(track).expect("the track is there");
    node.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect()
}

/// The split of a linked clip splits its **partner** too.
#[test]
fn the_partner_is_split_too() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session.execute("film.clip.split", &json!({ "clip": first, "at": 10 })).expect("the group is split");
    assert_eq!(duration_of(&session, first), 10, "the clip is split");
    assert_eq!(duration_of(&session, second), 10, "and its partner with it");
    assert_eq!(clips_on(&session, picture).len(), 2, "the track has two clips");
    assert_eq!(clips_on(&session, sound).len(), 2, "and the other track too");
}

/// The right-hand pieces are linked to **each other** and not to the
/// left-hand ones.
#[test]
fn the_right_hand_pieces_are_a_new_group() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": first, "at": 10 })).expect("the group is split");
    let new_group = outcome.data["new_group"].as_array().expect("the new group is named");
    let right_picture = new_group[0].as_u64().expect("the new clip has an id");
    let right_sound = new_group[1].as_u64().expect("the new clip has an id");
    // The left halves keep the old link.
    assert_eq!(link_of(&session, first), link_of(&session, second), "the left halves are still linked");
    // The right halves are linked to each other and not to the left.
    let link = link_of(&session, right_picture).expect("the right half is linked");
    assert_eq!(link_of(&session, right_sound), Some(link), "the right halves are linked to each other");
    assert_ne!(Some(link), link_of(&session, first), "and not to the left halves");
}

/// The **media** moves with the cut for every member.
#[test]
fn the_media_moves_with_the_cut() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": first, "at": 10 })).expect("the group is split");
    let new_group = outcome.data["new_group"].as_array().expect("the new group is named");
    for member in new_group {
        let clip = member.as_u64().expect("the new clip has an id");
        let node = session.document().node(clip).expect("the clip is there");
        assert_eq!(node.data["source_in"].as_u64(), Some(10), "the right half begins ten frames in");
    }
}

/// A member that does **not** cover the place is left alone: the tree
/// splits the members of a group that are under the place.
#[test]
fn a_member_that_does_not_cover_the_place_is_left_alone() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    let second = a_clip(&mut session, sound, item, 20, 30);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    session.execute("film.clip.split", &json!({ "clip": first, "at": 10 })).expect("the group is split");
    assert_eq!(duration_of(&session, first), 10, "the clip is split");
    assert_eq!(duration_of(&session, second), 30, "and the partner is not");
    assert_eq!(clips_on(&session, sound).len(), 1, "the other track still has one clip");
}

/// A place that is **not inside** the clip is refused.
#[test]
fn a_place_that_is_not_inside_is_refused() {
    let mut session = session();
    let (item, picture, _) = two_tracks(&mut session);
    let clip = a_clip(&mut session, picture, item, 0, 50);
    let outcome = session.execute("film.clip.split", &json!({ "clip": clip, "at": 90 }));
    assert!(outcome.is_err(), "the place outside the clip is refused");
}
