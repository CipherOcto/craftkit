//! The **ripple** of a speed change.
//!
//! A speed change makes a clip longer or shorter. Without a ripple the
//! change is bounded by the space there is — and here it is refused. With
//! a ripple the clips that were after it move **up** by the change, and
//! the time the clip took is time that is free.

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

/// The **three numbers** of a clip: where it is, how long it is and
/// where in the media it begins.
fn clock_of(session: &Session, clip: u64) -> (u64, u64, u64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    (start, duration, source_in)
}

/// The **span** of a transition: where it is and how long it is.
fn span_of(session: &Session, transition: u64) -> (u64, u64) {
    let node = session.document().node(transition).expect("the transition is there");
    let start = node.data["start"].as_u64().expect("the transition has a start");
    let duration = node.data["duration"].as_u64().expect("the transition has a duration");
    (start, duration)
}

/// Three clips side by side, each fifty frames long.
fn three_clips(session: &mut Session) -> (u64, u64, u64, u64, u64) {
    let (item, track) = a_timeline(session, 200);
    let first = a_clip(session, track, item, 0, 50);
    let second = a_clip(session, track, item, 50, 50);
    let third = a_clip(session, track, item, 100, 50);
    (item, track, first, second, third)
}

/// Without a ripple, a clip that would run into its neighbour is
/// **refused**.
#[test]
fn without_a_ripple_the_change_is_refused() {
    let mut session = session();
    let (_, _, first, _, _) = three_clips(&mut session);
    let refused = session.execute("film.clip.speed", &json!({ "clip": first, "speed": 0.5 }));
    assert!(refused.is_err(), "the clip would run into the one after it");
}

/// With a ripple, the clip gets the length it needs and the clips
/// after it **move up**.
#[test]
fn a_ripple_moves_the_clips_after_it() {
    let mut session = session();
    let (_, _, first, second, third) = three_clips(&mut session);
    session
        .execute("film.clip.speed", &json!({ "clip": first, "speed": 0.5, "ripple": true }))
        .expect("the speed is set");
    assert_eq!(clock_of(&session, first).1, 100, "the clip is twice as long");
    assert_eq!(clock_of(&session, second).0, 100, "the next clip moved up");
    assert_eq!(clock_of(&session, third).0, 150, "and so did the one after it");
}

/// The clips keep their own **length** when they move: only where they
/// are changes.
#[test]
fn the_moved_clips_keep_their_length() {
    let mut session = session();
    let (_, _, first, second, third) = three_clips(&mut session);
    session
        .execute("film.clip.speed", &json!({ "clip": first, "speed": 0.5, "ripple": true }))
        .expect("the speed is set");
    assert_eq!(clock_of(&session, second).1, 50, "the clip is as long as it was");
    assert_eq!(clock_of(&session, third).1, 50, "and so is the one after it");
}

/// A ripple that makes the clip **shorter** moves the clips after it
/// **down**.
#[test]
fn a_shorter_clip_moves_the_clips_down() {
    let mut session = session();
    let (_, _, first, second, third) = three_clips(&mut session);
    session
        .execute("film.clip.speed", &json!({ "clip": first, "speed": 2.0, "ripple": true }))
        .expect("the speed is set");
    assert_eq!(clock_of(&session, first).1, 25, "the clip is half as long");
    assert_eq!(clock_of(&session, second).0, 25, "the next clip moved down");
    assert_eq!(clock_of(&session, third).0, 75, "and so did the one after it");
}

/// The clips after the clip that changed are the ones that move: a
/// clip that is **before** it stays where it is.
#[test]
fn the_clips_before_it_do_not_move() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    session
        .execute("film.clip.speed", &json!({ "clip": second, "speed": 0.5, "ripple": true }))
        .expect("the speed is set");
    assert_eq!(clock_of(&session, first).0, 0, "the first clip stays where it is");
    assert_eq!(clock_of(&session, first).1, 50, "and it is as long as it was");
}

/// The **transitions** follow the cuts: one that joins two clips that
/// moved moves with them.
#[test]
fn the_transitions_follow_the_cuts() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let outcome = session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 10 }),
        )
        .expect("the transition is added");
    let transition = outcome.data["id"].as_u64().expect("the transition has an id");
    let before = span_of(&session, transition);
    session
        .execute("film.clip.speed", &json!({ "clip": first, "speed": 0.5, "ripple": true }))
        .expect("the speed is set");
    let after = span_of(&session, transition);
    assert_eq!(after.0, before.0 + 50, "the transition moved with the cut");
    assert_eq!(after.1, before.1, "and it is as long as it was");
}

/// A transition that no longer fits inside the two clips it joins is
/// brought back inside them.
#[test]
fn a_transition_that_no_longer_fits_is_brought_back() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let outcome = session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 40 }),
        )
        .expect("the transition is added");
    let transition = outcome.data["id"].as_u64().expect("the transition has an id");
    // The second clip becomes much shorter, and the transition that
    // joins it to the first one no longer fits inside the two.
    session
        .execute("film.clip.speed", &json!({ "clip": second, "speed": 4.0, "ripple": true }))
        .expect("the speed is set");
    let (start, duration) = span_of(&session, transition);
    let (first_start, first_duration, _) = clock_of(&session, first);
    let (_, second_duration, _) = clock_of(&session, second);
    assert!(
        start >= first_start && start + duration <= first_start + first_duration + second_duration,
        "the transition is inside the two clips: {start} and {duration}"
    );
}

/// A **locked** track refuses the change, ripple or not.
#[test]
fn a_locked_track_refuses_the_ripple() {
    let mut session = session();
    let (_, track, first, _, _) = three_clips(&mut session);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let refused = session.execute("film.clip.speed", &json!({ "clip": first, "speed": 0.5, "ripple": true }));
    assert!(refused.is_err(), "a locked track refuses the change");
}
