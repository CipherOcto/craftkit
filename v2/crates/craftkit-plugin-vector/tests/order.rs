//! The **order** of the drawing.
//!
//! The order the paths are drawn in is the order they are in, and the
//! kit says so: a node that is drawn later is on top, and the picture
//! and the picking agree because both walk the tree in that order.

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

/// Three **rectangles**, and their ids in the order they were made.
fn three_rectangles(session: &mut Session) -> Vec<u64> {
    let mut ids = Vec::new();
    for step in 0..3 {
        session
            .execute(
                "vector.rectangle",
                &json!({ "x": step as f64 * 10.0, "y": 0.0, "width": 10.0, "height": 10.0 }),
            )
            .expect("the rectangle is added");
        ids.push(
            session
                .document()
                .walk()
                .filter(|node| node.kind == "vector.path")
                .last()
                .map(|node| node.id)
                .expect("the rectangle is there"),
        );
    }
    ids
}

/// The paths of the document, in the order they are **drawn**.
fn drawn_of(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "vector.path").map(|node| node.id).collect()
}

/// A path may be brought **in front** of the others.
#[test]
fn a_path_may_be_brought_in_front() {
    let mut session = session();
    let ids = three_rectangles(&mut session);
    assert_eq!(drawn_of(&session), ids, "the paths are drawn in the order they were made");
    session
        .execute("vector.path.order", &json!({ "path": ids[0], "place": "front" }))
        .expect("the path is brought in front");
    assert_eq!(drawn_of(&session), vec![ids[1], ids[2], ids[0]], "the path is drawn last, and is in front");
}

/// And a path may be sent **behind** the others.
#[test]
fn a_path_may_be_sent_behind() {
    let mut session = session();
    let ids = three_rectangles(&mut session);
    session
        .execute("vector.path.order", &json!({ "path": ids[2], "place": "back" }))
        .expect("the path is sent behind");
    assert_eq!(drawn_of(&session), vec![ids[2], ids[0], ids[1]], "the path is drawn first, and is behind");
}

/// A place that is not one of the two is refused, and so is a node that
/// is not there.
#[test]
fn an_order_that_cannot_be_given_is_refused() {
    let mut session = session();
    let ids = three_rectangles(&mut session);
    let refused = session.execute("vector.path.order", &json!({ "path": ids[0], "place": "middle" }));
    assert!(refused.is_err(), "a path goes to the front or to the back");
    let refused = session.execute("vector.path.order", &json!({ "path": 9999, "place": "front" }));
    assert!(refused.is_err(), "a node that is not there is not moved");
}
