//! The **blob brush**: a shape painted by a brush.
//!
//! The tree's rules: the brush's size is the width of its stroke, the
//! stroke is round at its ends, and the shape grows where the brush
//! went.

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

/// The brush paints on the shape, and the shape **grows**.
#[test]
fn the_brush_paints_on_the_shape() {
    let mut session = session();
    let id = a_square(&mut session);
    let before = area_of(&session, id);
    session
        .execute(
            "vector.path.blob",
            &json!({ "path": id, "points": [[0.0, 10.0], [20.0, 10.0]], "size": 10.0 }),
        )
        .expect("the brush paints");
    let after = area_of(&session, id);
    assert!(after > before, "the shape grew: {after} and not {before}");
}

/// The **bigger** brush paints more.
#[test]
fn the_bigger_brush_paints_more() {
    let mut small_session = session();
    let id = a_square(&mut small_session);
    let before = area_of(&small_session, id);
    small_session
        .execute(
            "vector.path.blob",
            &json!({ "path": id, "points": [[0.0, 10.0], [20.0, 10.0]], "size": 4.0 }),
        )
        .expect("the brush paints");
    let small = area_of(&small_session, id) - before;
    let mut other = session();
    let other_id = a_square(&mut other);
    let before = area_of(&other, other_id);
    other
        .execute(
            "vector.path.blob",
            &json!({ "path": other_id, "points": [[0.0, 10.0], [20.0, 10.0]], "size": 16.0 }),
        )
        .expect("the brush paints");
    let big = area_of(&other, other_id) - before;
    assert!(big > small, "the bigger brush painted more: {big} and {small}");
}

/// A brush that **misses** the shape leaves the shape beside it.
#[test]
fn a_brush_that_misses_leaves_the_shape_beside_it() {
    let mut session = session();
    let id = a_square(&mut session);
    session
        .execute("vector.path.blob", &json!({ "path": id, "points": [[100.0, 100.0]], "size": 10.0 }))
        .expect("the brush paints");
    let contours = contours(&session, id);
    assert_eq!(contours.len(), 2, "the shape and the blob are two contours");
}

/// A **size** that is not positive is refused.
#[test]
fn a_size_that_is_not_positive_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome =
        session.execute("vector.path.blob", &json!({ "path": id, "points": [[0.0, 10.0]], "size": 0.0 }));
    assert!(outcome.is_err(), "the size that is not positive is refused");
}

/// A **fidelity** outside the brush's own bounds is refused.
#[test]
fn a_fidelity_outside_the_bounds_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session.execute(
        "vector.path.blob",
        &json!({ "path": id, "points": [[0.0, 10.0]], "size": 10.0, "fidelity": 200.0 }),
    );
    assert!(outcome.is_err(), "the fidelity outside the bounds is refused");
}

/// A sample that carries the pen's **pressure** is refused: this tree
/// has nowhere to keep it.
#[test]
fn a_sample_with_pressure_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.blob", &json!({ "path": id, "points": [[0.0, 10.0, 0.5]], "size": 10.0 }));
    assert!(outcome.is_err(), "the sample that carries the pressure is refused");
}
