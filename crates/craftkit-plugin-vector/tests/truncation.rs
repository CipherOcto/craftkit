//! The **erase** of a path, and the geometry it is coarse about.
//!
//! A side of a contour is sampled at no more than `MAX_SAMPLES` points,
//! and a side that is longer than that against a narrow eraser is
//! sampled more coarsely than the caller asked for. The answer says so:
//! `clamped`.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

// The battery's **anchor**: the one name that keeps its commands
// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_vector::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the vector battery.
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

/// The paths in the document, in the order they are in it.
fn paths(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "vector.path").map(|node| node.id).collect()
}

/// An **open path** of one straight side from the origin to that far
/// across, and its id.
fn a_line(session: &mut Session, length: f64) -> u64 {
    session
        .execute(
            "vector.path.create",
            &json!({ "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": length, "y": 0.0 }] }),
        )
        .expect("the path is added");
    paths(session)[0]
}

/// Whether the answer says that the geometry was **coarser** than the
/// caller asked for.
fn clamped_of(outcome: &craftkit_sdk::Outcome) -> Option<bool> {
    outcome.data.get("clamped").and_then(|clamped| clamped.as_bool())
}

/// A side **long enough** that the samples the eraser asks for are more
/// than the bound allows.
///
/// The samples a side is walked at are three times its longest edge
/// over the half-width of the eraser, so a side four times the bound
/// against an eraser of eight is well past it.
fn a_long_length() -> f64 {
    craftkit_plugin_vector::MAX_VECTOR_SAMPLES as f64 * 4.0
}

/// **A path short enough to be sampled as finely as it was asked for is
/// not clamped.**
#[test]
fn a_short_path_is_not_clamped() {
    let mut session = session();
    let id = a_line(&mut session, 20.0);
    let outcome = session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 5.0]], "width": 4.0 }),
        )
        .expect("the path is erased");
    assert_eq!(
        clamped_of(&outcome),
        Some(false),
        "the geometry was as fine as it was asked for: {outcome:?}"
    );
}

/// **A path too long to be sampled as finely as it was asked for is
/// clamped, and the answer says so.**
///
/// The side is four times the bound long and the eraser is eight wide:
/// the samples the width asks for are more than the bound allows, and
/// the geometry is coarser than the caller asked for.
#[test]
fn a_long_path_is_clamped_and_says_so() {
    let mut session = session();
    let length = a_long_length();
    let id = a_line(&mut session, length);
    let outcome = session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[length / 2.0, -5.0], [length / 2.0, 5.0]], "width": 8.0 }),
        )
        .expect("the path is erased");
    assert_eq!(
        clamped_of(&outcome),
        Some(true),
        "the geometry was coarser than it was asked for: {outcome:?}"
    );
}

/// **The shaped tip says it too.**
#[test]
fn the_shaped_tip_says_it_too() {
    let mut session = session();
    let length = a_long_length();
    let id = a_line(&mut session, length);
    let outcome = session
        .execute(
            "vector.path.erase.region",
            &json!({ "path": id, "points": [[length / 2.0, -5.0], [length / 2.0, 5.0]], "size": 8.0 }),
        )
        .expect("the path is erased");
    assert_eq!(
        clamped_of(&outcome),
        Some(true),
        "the geometry was coarser than it was asked for: {outcome:?}"
    );
}

/// **A path that is left alone is not clamped either.**
#[test]
fn a_path_that_is_left_alone_is_not_clamped() {
    let mut session = session();
    let id = a_line(&mut session, 20.0);
    let outcome = session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[10.0, 20.0], [10.0, 30.0]], "width": 4.0 }),
        )
        .expect("the eraser runs");
    assert!(!outcome.changed, "nothing was erased: {outcome:?}");
    assert_eq!(clamped_of(&outcome), Some(false), "nothing was sampled: {outcome:?}");
}
