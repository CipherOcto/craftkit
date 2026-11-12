//! The **copies** of a shape: a grid, a mirror and a ring.
//!
//! The tree's repeat tool makes copies of a shape — in a grid, over a
//! mirror, round a centre. Here the copies are **paths of their own**,
//! and the transform says where each one is.

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

/// A **rectangle** of ten by ten, and its id.
fn a_rectangle(session: &mut Session, x: f64, y: f64) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": x, "y": y, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .last()
        .map(|node| node.id)
        .expect("the rectangle is there")
}

/// The **paths** of the document, with the move each one is placed with.
fn paths_of(session: &Session) -> Vec<(u64, [f64; 6])> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .map(|node| {
            let transform = node.data["transform"].as_array().expect("a path says where it is");
            let numbers: Vec<f64> =
                transform.iter().map(|number| number.as_f64().unwrap_or_default()).collect();
            (node.id, [numbers[0], numbers[1], numbers[2], numbers[3], numbers[4], numbers[5]])
        })
        .collect()
}

/// The **move** each path is placed with: the across and the down.
fn moves_of(session: &Session) -> Vec<(f64, f64)> {
    paths_of(session).into_iter().map(|(_, matrix)| (matrix[4], matrix[5])).collect()
}

/// The grid makes a copy for every **cell** but the first.
#[test]
fn the_grid_makes_its_copies() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0);
    let made = session
        .execute(
            "vector.path.grid",
            &json!({ "path": id, "columns": 3, "rows": 2, "step_x": 20.0, "step_y": 20.0 }),
        )
        .expect("the grid is made");
    assert_eq!(made.data["copies"].as_array().expect("there are copies").len(), 5, "five copies");
    let mut moves = moves_of(&session);
    moves.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut found: Vec<(f64, f64)> = moves;
    found.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.1.partial_cmp(&right.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    assert_eq!(
        found,
        vec![(0.0, 0.0), (0.0, 20.0), (20.0, 0.0), (20.0, 20.0), (40.0, 0.0), (40.0, 20.0)],
        "the copies are in the cells of the grid"
    );
}

/// The mirror makes a copy **over the line**.
#[test]
fn the_mirror_makes_a_mirrored_copy() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0);
    session
        .execute(
            "vector.path.mirror",
            &json!({ "path": id, "from_x": 0.0, "from_y": 0.0, "to_x": 0.0, "to_y": 10.0 }),
        )
        .expect("the copy is made");
    let copy = session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .nth(1)
        .expect("the copy is there");
    let rect = copy.rect.expect("the copy knows where it is");
    assert_eq!(rect.x, -10.0, "the copy is on the other side of the line");
    assert_eq!(rect.w, 10.0, "and it is as wide as the original");
}

/// The ring makes a copy for every **turn** but the first.
#[test]
fn the_radial_makes_a_ring() {
    let mut session = session();
    let id = a_rectangle(&mut session, 10.0, 0.0);
    session
        .execute("vector.path.radial", &json!({ "path": id, "count": 4, "centre_x": 0.0, "centre_y": 0.0 }))
        .expect("the ring is made");
    // The copy a quarter of the way round is above and to the left.
    let copy = session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .nth(1)
        .expect("the copy is there");
    let rect = copy.rect.expect("the copy knows where it is");
    assert!(rect.x < 0.0 && rect.y > 0.0, "the copy is round the centre: {rect:?}");
}

/// Copies that cannot be made are **refused**, and they say why.
#[test]
fn a_copy_that_cannot_be_made_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0);
    // A grid of more copies than the bound allows.
    let refused = session.execute(
        "vector.path.grid",
        &json!({ "path": id, "columns": 100, "rows": 100, "step_x": 1.0, "step_y": 1.0 }),
    );
    assert!(refused.is_err(), "a grid of ten thousand copies is refused");
    // A line with no length mirrors nothing.
    let refused = session.execute(
        "vector.path.mirror",
        &json!({ "path": id, "from_x": 5.0, "from_y": 5.0, "to_x": 5.0, "to_y": 5.0 }),
    );
    assert!(refused.is_err(), "a line with no length mirrors nothing");
    // A count that is not a whole number.
    let refused = session.execute(
        "vector.path.radial",
        &json!({ "path": id, "count": 2.5, "centre_x": 0.0, "centre_y": 0.0 }),
    );
    assert!(refused.is_err(), "a count of copies is a whole number");
}
