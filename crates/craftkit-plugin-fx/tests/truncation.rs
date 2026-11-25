//! The **cuts of the fx battery**: the places where it stops early and
//! says so.
//!
//! A reader that stops at a bound must say that it stopped: a time that
//! is held at the last frame the tree holds is not the time the caller
//! asked for, and the answer says that it was held. Every cut of this
//! battery is named here, and the bound it stops at is named in **code**,
//! so that the proof and the bound cannot drift apart.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_fx::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the fx battery.
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

/// A **composition**, and its id.
fn a_comp(session: &mut Session) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    outcome.data["id"].as_u64().expect("the comp has an id")
}

/// The time is **held** at the last frame the tree holds, and the answer
/// says so.
///
/// The tree holds a billion frames, and a step that lands past it is
/// held there: the answer says that the step was held, so a caller
/// knows the time is not where it was asked to go.
#[test]
fn the_time_is_held_at_the_last_frame_and_the_answer_says_so() {
    let mut session = session();
    let comp = a_comp(&mut session);
    let outcome = session
        .execute("fx.time.step", &json!({ "comp": comp, "frames": 2_000_000_000i64 }))
        .expect("the time is stepped");
    assert_eq!(outcome.data["clamped"], json!(true), "the answer says it was held: {outcome:?}");
    let frame = outcome.data["frame"].as_u64().expect("the answer has a frame");
    assert_eq!(frame, 1_000_000_000, "the time is at the last frame: {outcome:?}");
}

/// The time that is stepped **within** the tree's frames is not held,
/// and the answer says so.
#[test]
fn the_time_within_the_frames_is_not_held() {
    let mut session = session();
    let comp = a_comp(&mut session);
    let outcome =
        session.execute("fx.time.step", &json!({ "comp": comp, "frames": 5 })).expect("the time is stepped");
    assert_eq!(outcome.data["clamped"], json!(false), "nothing was held: {outcome:?}");
    let frame = outcome.data["frame"].as_u64().expect("the answer has a frame");
    assert_eq!(frame, 5, "the time is where it was asked to go: {outcome:?}");
}
