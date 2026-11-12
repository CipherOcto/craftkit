//! The **eraser with a shaped tip**: the area under a tip taken away.
//!
//! The tree's rules: the tip is an ellipse whose long axis is the size
//! and whose short axis is the roundness; a closed shape loses the
//! area under the tip and an open path loses the outline under it.

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, FormatValue, Path, Session};

use craftkit_vector_geom as geom;

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

/// The paths in the document, in the order they are in it.
fn paths(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "vector.path").map(|node| node.id).collect()
}

/// The **shape** of the path, as the geometry library reads it.
fn shape_of(session: &Session, id: u64) -> Path {
    let node = session.document().node(id).expect("the path is there");
    Path::from_json(&node.data["path"]).expect("the path is read")
}

/// The **area** the shape covers.
fn area_of(session: &Session, id: u64) -> f64 {
    geom::area(&shape_of(session, id)).abs()
}

/// The **contours** of the path, as they are in the document.
fn contours(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"].as_array().expect("the path has contours").clone()
}

/// A **rectangle** twenty by twenty, and its id.
fn a_square(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0 }))
        .expect("the rectangle is added");
    paths(session)[0]
}

/// A closed shape loses the **area** under the tip.
#[test]
fn a_closed_shape_loses_the_area_under_the_tip() {
    let mut session = session();
    let id = a_square(&mut session);
    let before = area_of(&session, id);
    session
        .execute("vector.path.erase.region", &json!({ "path": id, "points": [[10.0, 10.0]], "size": 10.0 }))
        .expect("the shape is erased");
    let after = area_of(&session, id);
    assert!(after < before, "the shape is smaller: {after} and not {before}");
    // The tip is a circle of five across: it takes about seventy-eight
    // of the four hundred.
    assert!((before - after - 78.5).abs() < 2.0, "the tip took its own area: {after}");
}

/// A tip that **misses** the shape leaves it as it was.
#[test]
fn a_tip_that_misses_leaves_the_shape_alone() {
    let mut session = session();
    let id = a_square(&mut session);
    let before = contours(&session, id);
    let outcome = session.execute(
        "vector.path.erase.region",
        &json!({ "path": id, "points": [[100.0, 100.0]], "size": 10.0 }),
    );
    assert!(outcome.is_err() || !outcome.unwrap().changed, "nothing was erased");
    assert_eq!(contours(&session, id), before, "the shape is as it was");
}

/// A shape that is **wholly under the tip** is taken away.
#[test]
fn a_shape_wholly_under_the_tip_is_taken_away() {
    let mut session = session();
    let id = a_square(&mut session);
    session
        .execute("vector.path.erase.region", &json!({ "path": id, "points": [[10.0, 10.0]], "size": 40.0 }))
        .expect("the shape is erased");
    assert_eq!(paths(&session).len(), 0, "the shape is gone");
}

/// An **open path** loses the outline under the tip.
#[test]
fn an_open_path_loses_the_outline_under_the_tip() {
    let mut session = session();
    session
        .execute(
            "vector.path.create",
            &json!({ "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the path is added");
    let id = paths(&session)[0];
    session
        .execute("vector.path.erase.region", &json!({ "path": id, "points": [[10.0, 0.0]], "size": 4.0 }))
        .expect("the path is erased");
    assert_eq!(paths(&session).len(), 2, "the path is two paths now");
}

/// The **roundness** makes the tip flat, and a flat tip takes less away.
#[test]
fn the_roundness_makes_the_tip_flat() {
    let mut flat_session = session();
    let id = a_square(&mut flat_session);
    let before = area_of(&flat_session, id);
    flat_session
        .execute(
            "vector.path.erase.region",
            &json!({ "path": id, "points": [[10.0, 10.0]], "size": 10.0, "roundness": 10.0 }),
        )
        .expect("the shape is erased");
    let flat = before - area_of(&flat_session, id);
    let mut other = session();
    let other_id = a_square(&mut other);
    let before = area_of(&other, other_id);
    other
        .execute(
            "vector.path.erase.region",
            &json!({ "path": other_id, "points": [[10.0, 10.0]], "size": 10.0 }),
        )
        .expect("the shape is erased");
    let round = before - area_of(&other, other_id);
    assert!(flat < round, "the flat tip took less away: {flat} and {round}");
}

/// A **size** that is not positive is refused.
#[test]
fn a_size_that_is_not_positive_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.erase.region", &json!({ "path": id, "points": [[10.0, 10.0]], "size": 0.0 }));
    assert!(outcome.is_err(), "the size that is not positive is refused");
}

/// A **roundness** outside the tip's own bounds is refused.
#[test]
fn a_roundness_outside_the_bounds_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session.execute(
        "vector.path.erase.region",
        &json!({ "path": id, "points": [[10.0, 10.0]], "size": 10.0, "roundness": 200.0 }),
    );
    assert!(outcome.is_err(), "the roundness outside the bounds is refused");
}

/// A drag that is **not a list of points** is refused.
#[test]
fn a_drag_that_is_not_a_list_of_points_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session.execute("vector.path.erase.region", &json!({ "path": id, "points": "here" }));
    assert!(outcome.is_err(), "the drag that is not a list is refused");
}
