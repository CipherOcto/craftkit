//! The **transitions** of a track.
//!
//! A transition sits over the cut between two clips: it has a span on
//! the timeline, a duration, and an alignment on the cut.

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
fn a_timeline(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
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

/// A track with two clips that are **next to each other**: the first
/// from frame zero to frame fifty, the second from frame fifty to a
/// hundred.
fn two_clips(session: &mut Session) -> (u64, u64, u64) {
    let (item, track) = a_timeline(session);
    let first = a_clip(session, track, item, 0, 50);
    let second = a_clip(session, track, item, 50, 50);
    (track, first, second)
}

/// Add a transition over the cut between two clips.
fn add(session: &mut Session, track: u64, from: u64, to: u64, duration: u64, align: &str) -> u64 {
    let outcome = session
        .execute(
            "film.transition.add",
            &json!({
                "track": track,
                "from": from,
                "to": to,
                "duration": duration,
                "align": align,
            }),
        )
        .expect("the transition is added");
    outcome.data["id"].as_u64().expect("the transition has an id")
}

/// The **span** of a transition: where it starts and how long it is.
fn span_of(session: &Session, transition: u64) -> (u64, u64) {
    let node = session.document().node(transition).expect("the transition is there");
    let start = node.data["start"].as_u64().expect("the transition has a start");
    let duration = node.data["duration"].as_u64().expect("the transition has a duration");
    (start, duration)
}

/// A transition is **centred** on the cut.
#[test]
fn a_transition_is_centred_on_the_cut() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "center");
    assert_eq!(span_of(&session, transition), (40, 20), "the transition is half before the cut");
}

/// A transition may **start** at the cut.
#[test]
fn a_transition_may_start_at_the_cut() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "start");
    assert_eq!(span_of(&session, transition), (50, 20), "the transition starts at the cut");
}

/// A transition may **end** at the cut.
#[test]
fn a_transition_may_end_at_the_cut() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "end");
    assert_eq!(span_of(&session, transition), (30, 20), "the transition ends at the cut");
}

/// A transition is a thing of the **track**.
#[test]
fn a_transition_is_a_thing_of_the_track() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "center");
    let node = session.document().node(track).expect("the track is there");
    assert!(node.children.iter().any(|child| child.id == transition), "the transition is on the track");
}

/// Two clips that are **not next to each other** have no cut between
/// them.
#[test]
fn two_clips_that_are_not_next_to_each_other_are_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 60, 40);
    let outcome = session.execute(
        "film.transition.add",
        &json!({ "track": track, "from": first, "to": second, "duration": 10 }),
    );
    assert!(outcome.is_err(), "the clips with a gap between them are refused");
}

/// A transition that does not fit between the clips is **refused**.
#[test]
fn a_transition_that_does_not_fit_is_refused() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let outcome = session.execute(
        "film.transition.add",
        &json!({ "track": track, "from": first, "to": second, "duration": 200 }),
    );
    assert!(outcome.is_err(), "the transition that leaves the clips is refused");
}

/// A **locked** track refuses a transition.
#[test]
fn a_locked_track_refuses_a_transition() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let outcome = session.execute(
        "film.transition.add",
        &json!({ "track": track, "from": first, "to": second, "duration": 10 }),
    );
    assert!(outcome.is_err(), "the locked track refuses the transition");
}

/// A transition that is already there is **replaced**.
#[test]
fn a_transition_that_is_already_there_is_replaced() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let first_transition = add(&mut session, track, first, second, 20, "center");
    let second_transition = add(&mut session, track, first, second, 10, "center");
    let node = session.document().node(track).expect("the track is there");
    let transitions: Vec<u64> =
        node.children.iter().filter(|child| child.kind == "film.transition").map(|child| child.id).collect();
    assert_eq!(transitions.len(), 1, "there is one transition over the cut");
    assert_eq!(transitions[0], second_transition, "and it is the new one");
    // The old transition is gone from the track. The **id** may be the
    // same one: the core's ids are the highest in the document plus
    // one, and the old transition was the highest.
    let node = session.document().node(second_transition).expect("the transition is there");
    assert_eq!(node.data["duration"].as_u64(), Some(10), "the new transition is the one that is there");
    let _ = first_transition;
}

/// A transition can be **taken away**, and the clips stay.
#[test]
fn a_transition_can_be_taken_away() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "center");
    session
        .execute("film.transition.remove", &json!({ "transition": transition }))
        .expect("the transition is taken away");
    assert!(session.document().node(transition).is_none(), "the transition is gone");
    assert!(session.document().node(first).is_some(), "the first clip is there");
    assert!(session.document().node(second).is_some(), "and the second");
}

/// The **duration** of a transition may be changed, and the start
/// follows the alignment.
#[test]
fn the_duration_may_be_changed() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "center");
    session
        .execute("film.transition.set", &json!({ "transition": transition, "duration": 40 }))
        .expect("the transition is set");
    assert_eq!(span_of(&session, transition), (30, 40), "the transition is longer and still centred");
}

/// The **alignment** of a transition may be changed.
#[test]
fn the_alignment_may_be_changed() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let transition = add(&mut session, track, first, second, 20, "center");
    session
        .execute("film.transition.set", &json!({ "transition": transition, "align": "end" }))
        .expect("the transition is set");
    assert_eq!(span_of(&session, transition), (30, 20), "the transition ends at the cut");
}

/// An alignment that is **not** an alignment is refused.
#[test]
fn an_alignment_that_is_not_an_alignment_is_refused() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let outcome = session.execute(
        "film.transition.add",
        &json!({ "track": track, "from": first, "to": second, "duration": 10, "align": "sideways" }),
    );
    assert!(outcome.is_err(), "the alignment that is not one is refused");
}

/// A transition that this tree does not have is **refused**.
#[test]
fn a_transition_this_tree_does_not_have_is_refused() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let outcome = session.execute(
        "film.transition.add",
        &json!({ "track": track, "from": first, "to": second, "duration": 10, "effect": "wipe" }),
    );
    assert!(outcome.is_err(), "the transition that is not here is refused");
}

/// A transition of **no frames** is refused.
#[test]
fn a_transition_of_no_frames_is_refused() {
    let mut session = session();
    let (track, first, second) = two_clips(&mut session);
    let outcome = session.execute(
        "film.transition.add",
        &json!({ "track": track, "from": first, "to": second, "duration": 0 }),
    );
    assert!(outcome.is_err(), "the transition of no frames is refused");
}
