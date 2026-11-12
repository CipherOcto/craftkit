//! The **transitions** of a trim.
//!
//! A transition joins two clips, and it may not leave them. So when a
//! trim makes a clip shorter, the transitions that join it are brought
//! back inside the two clips they join.

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

/// A **transition** over the cut between two clips.
fn a_transition(session: &mut Session, track: u64, from: u64, to: u64, duration: u64, align: &str) -> u64 {
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

/// Where a transition **sits**, how long it is and how it sits on the
/// cut.
fn of_transition(session: &Session, transition: u64) -> (u64, u64, String) {
    let node = session.document().node(transition).expect("the transition is there");
    let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    let align = node.data.get("align").and_then(|align| align.as_str()).unwrap_or("center").to_string();
    (start, duration, align)
}

/// A transition that reaches **past** the clips it joins is moved back
/// inside them.
#[test]
fn a_transition_that_reaches_past_the_clips_is_moved_back_inside_them() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 20, "end");
    assert_eq!(of_transition(&session, transition).0, 30, "the transition ends at the cut");
    // The **head** of the first clip moves: the clip starts later and is
    // shorter, and the transition no longer fits where it was.
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 35 }))
        .expect("the clip is trimmed");
    let (start, duration, _) = of_transition(&session, transition);
    assert_eq!(start, 35, "the transition starts where the clip starts");
    assert_eq!(duration, 20, "and it is as long as it was");
}

/// A transition that is **shorter** than the clips it joins is
/// shortened.
#[test]
fn a_transition_that_is_too_long_is_shortened() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 10);
    let transition = a_transition(&mut session, track, first, second, 40, "end");
    assert_eq!(of_transition(&session, transition).0, 10, "the transition ends at the cut");
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 30 }))
        .expect("the clip is trimmed");
    let (start, duration, _) = of_transition(&session, transition);
    assert_eq!(duration, 30, "the transition is as long as the clips are");
    assert_eq!(start, 30, "and it is inside them");
}

/// The **alignment** is worked out again from where the transition
/// ends up.
#[test]
fn the_alignment_is_worked_out_again() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 40, "end");
    assert_eq!(of_transition(&session, transition).2, "end", "the transition ends at the cut");
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 20 }))
        .expect("the clip is trimmed");
    let (start, duration, align) = of_transition(&session, transition);
    assert_eq!(start, 20, "the transition starts where the clip starts");
    assert_eq!(duration, 40, "and it is as long as it was");
    assert_eq!(align, "center", "and it no longer ends at the cut");
}

/// A transition that **fits** is left alone.
#[test]
fn a_transition_that_fits_is_left_alone() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 20, "end");
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 5 }))
        .expect("the clip is trimmed");
    assert_eq!(
        of_transition(&session, transition),
        (30, 20, "end".to_string()),
        "the transition has not moved"
    );
}

/// A transition of **another** cut is not touched.
#[test]
fn a_transition_of_another_cut_is_not_touched() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let third = a_clip(&mut session, track, item, 100, 50);
    let transition = a_transition(&mut session, track, second, third, 20, "end");
    session
        .execute("film.clip.trim", &json!({ "clip": first, "edge": "in", "delta": 35 }))
        .expect("the clip is trimmed");
    assert_eq!(
        of_transition(&session, transition),
        (80, 20, "end".to_string()),
        "the transition has not moved"
    );
}

/// A transition that **starts at the cut** keeps that when it is moved.
#[test]
fn a_transition_that_starts_at_the_cut_keeps_that() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    let second = a_clip(&mut session, track, item, 50, 50);
    let transition = a_transition(&mut session, track, first, second, 20, "start");
    assert_eq!(of_transition(&session, transition).0, 50, "the transition starts at the cut");
    session
        .execute("film.clip.trim", &json!({ "clip": second, "edge": "out", "delta": -30 }))
        .expect("the clip is trimmed");
    let (start, _, align) = of_transition(&session, transition);
    assert_eq!(start, 50, "the transition is still at the cut");
    assert_eq!(align, "start", "and it still starts there");
}
