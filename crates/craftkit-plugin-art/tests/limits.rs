//! The **limits of the art battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.
//!
//! **The rate is a knob, and it is clamped.** A rate outside its
//! bounds is not refused: it is brought inside them, and the command
//! says which rate the element plays at. This is a deliberate
//! departure from the tree's own "refuse, do not clamp" rule, and the
//! tests below say plainly what the knob does.

use craftkit_plugin_art::{MAX_RATE, MIN_RATE};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_art::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A project with a scene, one video track and one element on it, and
/// the **id of the element**.
fn an_element() -> (Session, u64) {
    let ecosystem = ecosystem();
    let mut session = ecosystem.session();
    let outcome =
        session.execute("art.project.add", &json!({ "name": "the film" })).expect("the project is added");
    let project = outcome.data["id"].as_u64().expect("the project has an id");
    let outcome = session
        .execute("art.scene.add", &json!({ "project": project, "name": "act one", "main": true }))
        .expect("the scene is added");
    let scene = outcome.data["id"].as_u64().expect("the scene has an id");
    let outcome = session
        .execute("art.track.video.add", &json!({ "scene": scene, "name": "V1" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute(
            "art.element.video.add",
            &json!({ "track": track, "name": "the shot", "start": 48, "duration": 100, "media": "m_1" }),
        )
        .expect("the element is added");
    let element = outcome.data["id"].as_u64().expect("the element has an id");
    (session, element)
}

/// A rate **over `MAX_RATE`** is clamped to `MAX_RATE`.
///
/// The rate is a knob and not a fact about the media, and the command's
/// own documentation says that a rate outside the bounds is clamped
/// rather than refused. A rate of a hundred is a rate of five.
#[test]
fn a_rate_over_the_limit_is_clamped_to_the_limit() {
    let (mut session, element) = an_element();
    let outcome = session
        .execute("art.element.retime", &json!({ "element": element, "rate": 100.0 }))
        .expect("the element is retimed");
    let rate = outcome.data["rate"].as_f64().expect("the answer has a rate");
    assert_eq!(rate, MAX_RATE, "a rate of a hundred plays at five: {outcome:?}");
}

/// A rate **under `MIN_RATE`** is clamped to `MIN_RATE`.
///
/// The other end of the same knob: a rate of a thousandth is a rate of
/// a hundredth.
#[test]
fn a_rate_under_the_limit_is_clamped_to_the_limit() {
    let (mut session, element) = an_element();
    let outcome = session
        .execute("art.element.retime", &json!({ "element": element, "rate": 0.001 }))
        .expect("the element is retimed");
    let rate = outcome.data["rate"].as_f64().expect("the answer has a rate");
    assert_eq!(rate, MIN_RATE, "a rate of a thousandth plays at a hundredth: {outcome:?}");
}

/// A rate **within the bounds** is kept as it is.
///
/// This is the other half of the two tests above: a knob that clamped
/// everything would not be a knob.
#[test]
fn a_rate_within_the_limits_is_kept() {
    let (mut session, element) = an_element();
    let outcome = session
        .execute("art.element.retime", &json!({ "element": element, "rate": 2.5 }))
        .expect("the element is retimed");
    let rate = outcome.data["rate"].as_f64().expect("the answer has a rate");
    assert_eq!(rate, 2.5, "the rate is the rate that was asked for: {outcome:?}");
}

/// A rate of **zero** is refused.
///
/// A rate is a knob within bounds, and a rate of zero is not a rate at
/// all: there is no lower bound to clamp it to, so it is an error.
#[test]
fn a_rate_of_zero_is_refused() {
    let (mut session, element) = an_element();
    let error = session
        .execute("art.element.retime", &json!({ "element": element, "rate": 0.0 }))
        .expect_err("a rate of zero is not a rate");
    assert!(error.to_string().contains("a rate is a number above zero"), "{error}");
}
