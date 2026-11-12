//! The **stroke drawn by hand**.
//!
//! The tree's rule: the samples are fitted — the jitter is dropped, the
//! sharp turns stay corners, and the rest is a spline through the
//! samples that is then simplified.

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
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

/// The path of the one path in the document.
fn path_of(session: &Session) -> Json {
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("there is a path");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The one contour of the path.
fn contour_of(session: &Session) -> Json {
    path_of(session)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .cloned()
        .expect("the path has a contour")
}

/// The **sides** of the contour.
fn sides_of(session: &Session) -> Vec<Json> {
    contour_of(session)
        .get("segments")
        .and_then(|segments| segments.as_array())
        .cloned()
        .expect("the contour has sides")
}

/// Where a side ends.
fn end_of(side: &Json) -> (f64, f64) {
    let end = side
        .get("line")
        .and_then(|line| line.get("to"))
        .or_else(|| side.get("cubic").and_then(|cubic| cubic.get("to")))
        .expect("the side has an end");
    (
        end.get("x").and_then(|x| x.as_f64()).expect("the end is across"),
        end.get("y").and_then(|y| y.as_f64()).expect("the end is down"),
    )
}

/// The **control point** of a side, if it is a curve.
fn control_of(side: &Json, name: &str) -> Option<(f64, f64)> {
    let curve = side.get("cubic")?;
    let value = curve.get(name)?;
    Some((value.get("x").and_then(|x| x.as_f64())?, value.get("y").and_then(|y| y.as_f64())?))
}

/// The direction a side **leaves** in.
fn leaves_in(side: &Json, from: (f64, f64)) -> (f64, f64) {
    match control_of(side, "c1") {
        Some(c1) => (c1.0 - from.0, c1.1 - from.1),
        None => {
            let end = end_of(side);
            (end.0 - from.0, end.1 - from.1)
        }
    }
}

/// The direction a side **arrives** in.
fn arrives_in(side: &Json, from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    match control_of(side, "c2") {
        Some(c2) => (to.0 - c2.0, to.1 - c2.1),
        None => (to.0 - from.0, to.1 - from.1),
    }
}

/// The samples of an **L**: a turn of ninety degrees.
fn an_l() -> Json {
    json!([[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]])
}

/// A stroke that is **not fitted** is its samples joined by straight
/// sides.
#[test]
fn a_stroke_that_is_not_fitted_is_its_samples() {
    let mut session = session();
    session
        .execute("vector.path.freehand", &json!({ "points": an_l(), "fit": false }))
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    assert_eq!(sides.len(), 2, "three samples are two sides");
    assert_eq!(end_of(&sides[0]), (10.0, 0.0), "the first side goes to the second sample");
    assert_eq!(end_of(&sides[1]), (10.0, 10.0), "the second side goes to the third sample");
}

/// The **jitter** is dropped: a sample closer than a third of the
/// fidelity to the last one kept is not a sample of the stroke.
#[test]
fn the_jitter_is_dropped() {
    let mut session = session();
    session
        .execute(
            "vector.path.freehand",
            &json!({ "points": [[0.0, 0.0], [0.1, 0.0], [10.0, 0.0]], "fidelity": 1.5 }),
        )
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    assert_eq!(sides.len(), 1, "the sample that was jitter is gone: {sides:?}");
    assert_eq!(end_of(&sides[0]), (10.0, 0.0), "the side goes to the sample that was kept");
}

/// The **last sample** always ends the stroke, even when it is close to
/// the one before it.
#[test]
fn the_last_sample_ends_the_stroke() {
    let mut session = session();
    session
        .execute("vector.path.freehand", &json!({ "points": [[0.0, 0.0], [10.0, 0.0], [10.05, 0.0]] }))
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    assert_eq!(end_of(sides.last().expect("there is a last side")), (10.05, 0.0));
}

/// A turn of more than a hundred degrees stays a **corner**: the sides
/// do not meet straight there.
#[test]
fn a_sharp_turn_stays_a_corner() {
    let mut session = session();
    session
        .execute(
            "vector.path.freehand",
            &json!({ "points": [[0.0, 0.0], [10.0, 0.0], [5.0, 1.0]], "fidelity": 0.1 }),
        )
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    assert_eq!(sides.len(), 2, "three samples are two sides");
    let turn = sides[0].get("line").is_some() || sides[1].get("line").is_some();
    assert!(turn, "the sides are straight at a corner");
    let first_end = end_of(&sides[0]);
    let before = arrives_in(&sides[0], (0.0, 0.0), first_end);
    let after = leaves_in(&sides[1], first_end);
    let dot = before.0 * after.0 + before.1 * after.1;
    assert!(dot < 0.9, "the sides do not meet straight at the corner: {dot}");
}

/// A turn of **less** than a hundred degrees is not a corner: the sides
/// meet straight there.
#[test]
fn a_gentle_turn_is_not_a_corner() {
    let mut session = session();
    session
        .execute("vector.path.freehand", &json!({ "points": an_l(), "fidelity": 0.1 }))
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    let first_end = end_of(&sides[0]);
    let before = arrives_in(&sides[0], (0.0, 0.0), first_end);
    let after = leaves_in(&sides[1], first_end);
    let dot = before.0 * after.0 + before.1 * after.1;
    assert!(dot > 0.9, "the sides meet straight at the sample: {dot}");
}

/// The stroke is **fitted**: the sides at a smooth sample are curves.
#[test]
fn the_stroke_is_fitted() {
    let mut session = session();
    session
        .execute("vector.path.freehand", &json!({ "points": an_l(), "fidelity": 0.1 }))
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    assert!(!sides.is_empty(), "the stroke has sides");
    assert!(control_of(&sides[0], "c2").is_some(), "the side is a curve: {}", sides[0]);
    let contour = contour_of(&session);
    let start = contour.get("start").expect("the contour starts somewhere");
    assert_eq!(start.get("x").and_then(|x| x.as_f64()), Some(0.0), "the stroke starts at the first sample");
    assert_eq!(
        end_of(sides.last().expect("there is a last side")),
        (10.0, 10.0),
        "the stroke ends at the last sample"
    );
}

/// A stroke that is **not fitted** is the samples joined by straight
/// sides.
#[test]
fn a_stroke_that_is_not_fitted_is_straight() {
    let mut session = session();
    session
        .execute("vector.path.freehand", &json!({ "points": an_l(), "fit": false }))
        .expect("the stroke is drawn");
    let sides = sides_of(&session);
    for side in &sides {
        assert!(side.get("line").is_some(), "the side is straight: {side}");
    }
}

/// A **closed** stroke is closed.
#[test]
fn a_closed_stroke_is_closed() {
    let mut session = session();
    session
        .execute(
            "vector.path.freehand",
            &json!({ "points": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], "closed": true }),
        )
        .expect("the stroke is drawn");
    let contour = contour_of(&session);
    assert_eq!(contour.get("closed").and_then(|closed| closed.as_bool()), Some(true));
}

/// The tree's Pencil keeps the pen's **pressure** on a stroke, and this
/// tree has nowhere to keep it: a sample that carries one is refused.
#[test]
fn a_sample_that_carries_a_pressure_is_refused() {
    let mut session = session();
    let outcome =
        session.execute("vector.path.freehand", &json!({ "points": [[0.0, 0.0, 0.5], [10.0, 0.0, 0.4]] }));
    assert!(outcome.is_err(), "the sample with a pressure is refused");
}

/// A **fidelity** outside the bounds is refused.
#[test]
fn a_fidelity_outside_the_bounds_is_refused() {
    let mut session = session();
    let outcome = session.execute("vector.path.freehand", &json!({ "points": an_l(), "fidelity": 1000.0 }));
    assert!(outcome.is_err(), "the fidelity that is too large is refused");
}

/// A stroke of **no samples** is refused.
#[test]
fn a_stroke_of_no_samples_is_refused() {
    let mut session = session();
    let outcome = session.execute("vector.path.freehand", &json!({ "points": [] }));
    assert!(outcome.is_err(), "the stroke of no samples is refused");
}

/// The stroke is **named**, as the tree names it.
#[test]
fn the_stroke_is_named() {
    let mut session = session();
    session.execute("vector.path.freehand", &json!({ "points": an_l() })).expect("the stroke is drawn");
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("there is a path");
    assert_eq!(node.name, "Pencil", "the tree calls a stroke drawn by hand a Pencil");
}
