//! The **transitions** of a split.
//!
//! A split cuts a clip in two, and the transitions of the track have to
//! answer for the cut: the ones that follow it join the piece after it,
//! and the ones that span it keep the piece before it.

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

/// A **bin**, an **item** in it, a **track** in the item.
fn a_timeline(session: &mut Session, media: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": media }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
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

/// A **transition** over the cut between two clips, centred on it.
fn a_transition(session: &mut Session, track: u64, from: u64, to: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": from, "to": to, "duration": duration }),
        )
        .expect("the transition is added");
    outcome.data["id"].as_u64().expect("the transition has an id")
}

/// The clip a transition joins on its **outgoing** side.
fn from_of(session: &Session, transition: u64) -> Option<u64> {
    let node = session.document().node(transition).expect("the transition is there");
    node.data.get("from").and_then(|from| from.as_u64())
}

/// The clip a transition joins on its **incoming** side.
fn to_of(session: &Session, transition: u64) -> Option<u64> {
    let node = session.document().node(transition).expect("the transition is there");
    node.data.get("to").and_then(|to| to.as_u64())
}

/// Where a transition **begins**.
fn start_of(session: &Session, transition: u64) -> u64 {
    let node = session.document().node(transition).expect("the transition is there");
    node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0)
}

/// A transition that **follows** the cut joins the piece after it.
#[test]
fn a_transition_after_the_cut_joins_the_piece_after_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 10);
    assert_eq!(start_of(&session, transition), 45, "the transition is centred on the cut at 50");
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": first, "at": 40 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("the split has a new clip");
    assert_eq!(from_of(&session, transition), Some(new), "the transition joins the piece after the cut");
    assert_eq!(start_of(&session, transition), 45, "and the transition has not moved");
}

/// A transition that **spans** the cut keeps the piece before it: the
/// tree's rule is about the transition's **start**.
#[test]
fn a_transition_that_spans_the_cut_keeps_the_piece_before_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 10);
    session.execute("film.clip.split", &json!({ "clip": first, "at": 47 })).expect("the clip is split");
    assert_eq!(from_of(&session, transition), Some(first), "the transition keeps the piece before the cut");
}

/// A transition that **ends** at the clip is not moved: the tree moves
/// the outgoing side and not the incoming one.
#[test]
fn a_transition_that_ends_at_the_clip_is_not_moved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 30);
    let second = a_clip(&mut session, track, item, 30, 50);
    a_clip(&mut session, track, item, 80, 50);
    let transition = a_transition(&mut session, track, first, second, 10);
    assert_eq!(start_of(&session, transition), 25, "the transition is centred on the cut at 30");
    session.execute("film.clip.split", &json!({ "clip": second, "at": 60 })).expect("the clip is split");
    assert_eq!(
        to_of(&session, transition),
        Some(second),
        "the transition still ends at the piece before the cut"
    );
    assert_eq!(from_of(&session, transition), Some(first), "and it still begins at the clip before it");
}

/// A transition that does not touch the clip is not touched at all.
#[test]
fn a_transition_of_another_cut_is_not_touched() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 30);
    let second = a_clip(&mut session, track, item, 30, 50);
    let third = a_clip(&mut session, track, item, 80, 50);
    let before = a_transition(&mut session, track, first, second, 10);
    let after = a_transition(&mut session, track, second, third, 10);
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": second, "at": 40 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("the split has a new clip");
    assert_eq!(from_of(&session, before), Some(first), "the transition before the cut has not moved");
    assert_eq!(to_of(&session, before), Some(second), "and it still ends at the piece before the cut");
    assert_eq!(start_of(&session, after), 75, "the transition after the cut has not moved");
    assert_eq!(from_of(&session, after), Some(new), "and it joins the piece after the cut");
}

/// The transition after the cut of a **linked group** follows the cut
/// on every track of the group.
#[test]
fn the_transitions_of_a_linked_group_follow_the_cut() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 10);
    // A **second track** with the same two clips, linked to the first.
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the other track", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let other = outcome.data["id"].as_u64().expect("the track has an id");
    let sound = a_clip(&mut session, other, item, 0, 50);
    let sound_next = a_clip(&mut session, other, item, 50, 50);
    let other_transition = a_transition(&mut session, other, sound, sound_next, 10);
    session
        .execute("film.clip.link", &json!({ "clip": first, "with": sound }))
        .expect("the clips are linked");
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": first, "at": 40 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("the split has a new clip");
    assert_eq!(from_of(&session, transition), Some(new), "the transition of the picture follows the cut");
    let new_sound = outcome.data["new_group"]
        .as_array()
        .expect("the split has a new group")
        .iter()
        .filter_map(|id| id.as_u64())
        .find(|id| *id != new)
        .expect("the sound was split too");
    assert_eq!(
        from_of(&session, other_transition),
        Some(new_sound),
        "and so does the transition of the sound"
    );
}

/// The split says which transitions it moved.
#[test]
fn the_split_says_which_transitions_it_moved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 10);
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": first, "at": 40 })).expect("the clip is split");
    let moved = outcome.data["transitions"].as_array().expect("the split says what it moved");
    assert_eq!(moved.len(), 1, "one transition moved");
    assert_eq!(moved[0].as_u64(), Some(transition), "and it is the one that followed the cut");
}
