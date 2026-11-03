//! The **surround pan** of a track: where its sound is in the room.
//!
//! The surround pan is a **position** and not a balance: the balance
//! says how much of the sound is on the left, and this says where the
//! sound is. The **gains** are derived from the position and the
//! session's layout, and they are the power of the sound on each
//! speaker.

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

/// A **track**, and its id.
fn a_track(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    session.document().node_by_name("drums").expect("the track is there").id
}

/// The **surround pan** of a track, as the document holds it.
fn surround_of(session: &Session, track: u64) -> Option<craftkit_sdk::serde_json::Value> {
    let node = session.document().node(track).expect("the track is there");
    node.data.get("surround").filter(|surround| !surround.is_null()).cloned()
}

/// The **gains** the command reports.
fn gains_of(outcome: &craftkit_sdk::Outcome) -> Vec<f64> {
    outcome.data["gains"]
        .as_array()
        .expect("the gains are a list")
        .iter()
        .map(|gain| gain.as_f64().expect("a gain is a number"))
        .collect()
}

/// The surround pan is a **fact** of the track.
#[test]
fn the_surround_pan_is_a_fact_of_the_track() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.surround", &json!({ "track": track, "x": -0.5, "y": 0.2 }))
        .expect("the track is in the room");
    let surround = surround_of(&session, track).expect("the track has a surround pan");
    assert_eq!(surround["x"].as_f64(), Some(-0.5), "the pan is to the left: {surround:?}");
    assert_eq!(surround["y"].as_f64(), Some(0.2), "and a little back: {surround:?}");
}

/// The fields that are left out keep what they were.
#[test]
fn the_fields_that_are_left_out_keep_what_they_were() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.surround", &json!({ "track": track, "x": -0.5, "y": 0.2 }))
        .expect("the track is in the room");
    session
        .execute("sound.track.surround", &json!({ "track": track, "z": 0.5 }))
        .expect("the track is in the room");
    let surround = surround_of(&session, track).expect("the track has a surround pan");
    assert_eq!(surround["x"].as_f64(), Some(-0.5), "the x is still there: {surround:?}");
    assert_eq!(surround["z"].as_f64(), Some(0.5), "and the z is too: {surround:?}");
}

/// The **gains** are derived from the pan and the layout.
#[test]
fn the_gains_are_derived() {
    let mut session = session();
    let track = a_track(&mut session);
    let outcome = session
        .execute("sound.track.surround", &json!({ "track": track, "x": -0.5, "y": 1.0 }))
        .expect("the track is in the room");
    let gains = gains_of(&outcome);
    assert_eq!(gains.len(), 2, "the stereo layout has two speakers: {gains:?}");
    assert!(gains[0] > gains[1], "and the sound is on the left: {gains:?}");
}

/// The gains of the speakers that are not the LFE add up to **one**:
/// the sound is not quieter when it is spread.
#[test]
fn the_gains_add_up_to_one() {
    let mut session = session();
    let track = a_track(&mut session);
    session.execute("sound.session.set", &json!({ "layout": "5.1" })).expect("the room is five point one");
    let outcome = session
        .execute("sound.track.surround", &json!({ "track": track, "x": 0.3, "y": 0.4 }))
        .expect("the track is in the room");
    let gains = gains_of(&outcome);
    assert_eq!(gains.len(), 6, "the layout has six speakers: {gains:?}");
    let power: f64 = gains.iter().map(|gain| gain * gain).sum();
    assert!((power - 1.0).abs() < 1e-6, "the power is one: {gains:?}");
}

/// The **LFE** is never panned to.
#[test]
fn the_lfe_is_never_panned_to() {
    let mut session = session();
    let track = a_track(&mut session);
    session.execute("sound.session.set", &json!({ "layout": "5.1" })).expect("the room is five point one");
    let outcome = session
        .execute("sound.track.surround", &json!({ "track": track, "x": 1.0, "y": 1.0 }))
        .expect("the track is in the room");
    let gains = gains_of(&outcome);
    assert_eq!(gains[3].abs(), 0.0, "the LFE is the fourth speaker: {gains:?}");
}

/// A puck in the **corner** of the room is exactly on one speaker.
#[test]
fn a_puck_in_a_corner_is_on_one_speaker() {
    let mut session = session();
    let track = a_track(&mut session);
    session.execute("sound.session.set", &json!({ "layout": "7.1" })).expect("the room is seven point one");
    let outcome = session
        .execute("sound.track.surround", &json!({ "track": track, "x": -1.0, "y": -1.0 }))
        .expect("the track is in the room");
    let gains = gains_of(&outcome);
    let loudest = gains
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .map(|(index, _)| index)
        .expect("there are gains");
    assert_eq!(loudest, 4, "the back left surround is the fifth speaker: {gains:?}");
    assert!((gains[4] - 1.0).abs() < 1e-6, "and it has all the sound: {gains:?}");
}

/// The **layout** is a setting of the session.
#[test]
fn the_layout_is_a_setting_of_the_session() {
    let mut session = session();
    let track = a_track(&mut session);
    let outcome = session
        .execute("sound.session.set", &json!({ "layout": "7.1.4" }))
        .expect("the room is seven point one with four heights");
    assert_eq!(outcome.data["layout"].as_str(), Some("7.1.4"), "the layout is set: {outcome:?}");
    let outcome = session
        .execute("sound.track.surround", &json!({ "track": track, "x": 0.0, "y": 1.0 }))
        .expect("the track is in the room");
    let gains = gains_of(&outcome);
    assert_eq!(gains.len(), 12, "the layout has twelve speakers: {gains:?}");
}

/// A layout that is **not one of them** is refused.
#[test]
fn a_layout_that_is_not_one_of_them_is_refused() {
    let mut session = session();
    let refused = session.execute("sound.session.set", &json!({ "layout": "quad" }));
    assert!(refused.is_err(), "there is no quad layout here");
}

/// A position **outside the room** is refused, and not pulled to the
/// edge.
#[test]
fn a_position_outside_the_room_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let refused = session.execute("sound.track.surround", &json!({ "track": track, "x": 2.0 }));
    assert!(refused.is_err(), "a position is between minus one and one");
    let refused = session.execute("sound.track.surround", &json!({ "track": track, "z": -1.0 }));
    assert!(refused.is_err(), "a height is between zero and one");
    let refused = session.execute("sound.track.surround", &json!({ "track": track, "center": 200.0 }));
    assert!(refused.is_err(), "a share is a percentage");
}

/// The surround pan is **taken away**, and the track is in the middle
/// of the room again.
#[test]
fn the_surround_pan_is_taken_away() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("sound.track.surround", &json!({ "track": track, "x": -0.5 }))
        .expect("the track is in the room");
    session
        .execute("sound.track.surround.delete", &json!({ "track": track }))
        .expect("the pan is taken away");
    assert!(surround_of(&session, track).is_none(), "and the track has no pan");
}

/// A track that is **not there** is refused.
#[test]
fn a_track_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session.execute("sound.track.surround", &json!({ "track": 999, "x": 0.5 }));
    assert!(refused.is_err(), "there is no such track");
}
