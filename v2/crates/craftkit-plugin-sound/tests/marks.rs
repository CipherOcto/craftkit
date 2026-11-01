//! The **marks** on a clip: the rating a person gives it, and the
//! colour it is drawn with.
//!
//! The tree's sound editor has two marks on a clip: a rating of stars,
//! and a tint for its bar. Only one of the two is the battery's own.

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

/// A track with one clip on it, and the id of the clip.
fn one_clip(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 500 }))
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// The rating a clip carries.
fn rating_of(session: &Session, clip: u64) -> Option<u64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("rating").and_then(|rating| rating.as_u64())
}

/// A clip is **rated**, and the rating is on the clip.
#[test]
fn a_clip_is_rated() {
    let mut session = session();
    let clip = one_clip(&mut session);
    assert_eq!(rating_of(&session, clip), Some(0), "a new clip has no stars");
    session.execute("sound.clip.rating", &json!({ "clip": clip, "rating": 4 })).expect("the clip is rated");
    assert_eq!(rating_of(&session, clip), Some(4), "the clip has four stars");
}

/// A rating of **none** is a rating: the clip is rated zero and not
/// unrated.
#[test]
fn no_stars_is_a_rating() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.rating", &json!({ "clip": clip, "rating": 3 })).expect("the clip is rated");
    session
        .execute("sound.clip.rating", &json!({ "clip": clip, "rating": 0 }))
        .expect("the rating is taken away");
    assert_eq!(rating_of(&session, clip), Some(0), "the clip has no stars");
}

/// A rating outside the range is **refused**, and the clip keeps the
/// rating it had. The tree's command clamps such a rating into the
/// range; a rating of nine stars is not a rating.
#[test]
fn a_rating_outside_the_range_is_refused() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.rating", &json!({ "clip": clip, "rating": 4 })).expect("the clip is rated");
    let refused = session
        .execute("sound.clip.rating", &json!({ "clip": clip, "rating": 9 }))
        .expect_err("nine stars is not a rating");
    assert!(refused.to_string().contains("rating"), "the error says why: {refused}");
    assert_eq!(rating_of(&session, clip), Some(4), "the clip keeps its four stars");
    let refused = session
        .execute("sound.clip.rating", &json!({ "clip": clip, "rating": -1 }))
        .expect_err("minus one star is not a rating");
    assert!(refused.to_string().contains("rating"), "the error says why: {refused}");
    assert_eq!(rating_of(&session, clip), Some(4), "the clip keeps its four stars");
}

/// A rating is a **whole** number of stars.
#[test]
fn half_a_star_is_not_a_rating() {
    let mut session = session();
    let clip = one_clip(&mut session);
    let refused = session
        .execute("sound.clip.rating", &json!({ "clip": clip, "rating": 2.5 }))
        .expect_err("two and a half stars is not a rating");
    assert!(refused.to_string().contains("whole"), "the error says why: {refused}");
    assert_eq!(rating_of(&session, clip), Some(0), "the clip is unrated");
}

/// The rating is in the **plan**: the clip's own kind says what the
/// clip means, and the rating is part of that.
#[test]
fn the_rating_is_in_the_plan() {
    let mut session = session();
    let clip = one_clip(&mut session);
    session.execute("sound.clip.rating", &json!({ "clip": clip, "rating": 5 })).expect("the clip is rated");
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == clip && item.kind == "sound.clip")
        .expect("the clip is in the plan");
    assert_eq!(item.value.get("rating").and_then(|rating| rating.as_u64()), Some(5));
}

/// The clip's **colour** is the kit's appearance and not a field of the
/// clip: one command sets the colour of any node, and the clip is a
/// node.
#[test]
fn the_colour_of_a_clip_is_its_appearance() {
    let mut session = session();
    let clip = one_clip(&mut session);
    assert!(
        session.document().appearance_of(session.document().node(clip).expect("the clip is there")).is_none()
    );
    session
        .execute("core.appearance.set", &json!({ "node": clip, "fill": "#ff8800" }))
        .expect("the clip is given a colour");
    let node = session.document().node(clip).expect("the clip is there");
    let appearance = session.document().appearance_of(node).expect("the clip has an appearance");
    let colour = appearance.fill.colour();
    assert_eq!(colour.0[0], 1.0, "the red channel is full");
    assert!((colour.0[1] - 0.533_333_3).abs() < 0.001, "the green channel is the colour asked for");
    assert!((colour.0[2] - 0.0).abs() < 0.001, "the blue channel is empty");
    // And the rating is still the clip's own business: the colour did
    // not touch it.
    assert_eq!(rating_of(&session, clip), Some(0), "the clip is still unrated");
}

/// A rating on a node that is not a clip is **refused**: the rating is
/// the clip's own.
#[test]
fn only_a_clip_can_be_rated() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let refused = session
        .execute("sound.clip.rating", &json!({ "clip": track, "rating": 3 }))
        .expect_err("a track is not a clip");
    assert!(refused.to_string().contains("not a clip"), "the error says why: {refused}");
}
