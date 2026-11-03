//! The **trim** of a linked group.
//!
//! The tree trims a linked group as **one edit**: the clip and its
//! partners take the same delta, and the delta is the one every member
//! can take.

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

/// The **source** of a clip: where in the media it begins.
fn source_of(session: &Session, clip: u64) -> u64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["source_in"].as_u64().expect("the clip has a source")
}

/// Two clips on two tracks, linked, and both fifty frames long.
fn linked(session: &mut Session) -> (u64, u64) {
    let (item, picture, sound) = two_tracks(session);
    let first = a_clip(session, picture, item, 0, 50);
    let second = a_clip(session, sound, item, 0, 50);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    (first, second)
}

/// The trim of a linked clip trims its **partner** too.
#[test]
fn the_partner_is_trimmed_too() {
    let mut session = session();
    let (first, second) = linked(&mut session);
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "out", "delta": 10 }))
        .expect("the group is trimmed");
    assert_eq!(duration_of(&session, first), 60, "the clip is trimmed");
    assert_eq!(duration_of(&session, second), 60, "and its partner with it");
}

/// The **in edge** of a group moves the media of every member.
#[test]
fn the_in_edge_moves_the_media_of_every_member() {
    let mut session = session();
    let (first, second) = linked(&mut session);
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 5 }))
        .expect("the group is trimmed");
    assert_eq!(source_of(&session, first), 5, "the media of the clip moves on");
    assert_eq!(source_of(&session, second), 5, "and the media of its partner");
}

/// A member that **cannot take** the delta refuses the whole trim: the
/// tree clamps the delta to the smallest bound, and here the trim is
/// refused and the member is named.
#[test]
fn a_member_that_cannot_take_it_refuses_the_trim() {
    let mut session = session();
    let (item, picture, sound) = two_tracks(&mut session);
    let first = a_clip(&mut session, picture, item, 0, 50);
    // The sound is only five frames long, so it cannot take a trim of
    // ten: a clip is at least one frame long.
    let second = a_clip(&mut session, sound, item, 0, 5);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": second }))
        .expect("the clips are linked");
    let outcome = session.execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 10 }));
    assert!(outcome.is_err(), "the trim that a member cannot take is refused");
    assert_eq!(duration_of(&session, first), 50, "and nothing is trimmed");
    assert_eq!(duration_of(&session, second), 5, "nor its partner");
}

/// The outcome names the **group** the trim was made in.
#[test]
fn the_outcome_names_the_group() {
    let mut session = session();
    let (first, second) = linked(&mut session);
    let outcome = session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "out", "delta": 5 }))
        .expect("the group is trimmed");
    let group = outcome.data["group"].as_array().expect("the group is named");
    assert_eq!(group.len(), 2, "the group has two members");
    assert!(group.iter().any(|member| member.as_u64() == Some(first)), "the clip is in it");
    assert!(group.iter().any(|member| member.as_u64() == Some(second)), "and its partner");
}

/// A clip that is **not** linked is trimmed on its own.
#[test]
fn a_clip_that_is_not_linked_is_trimmed_on_its_own() {
    let mut session = session();
    let (item, picture, _) = two_tracks(&mut session);
    let clip = a_clip(&mut session, picture, item, 0, 50);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect("the clip is trimmed");
    assert_eq!(duration_of(&session, clip), 60, "the clip is trimmed");
}
