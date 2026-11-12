//! The **setting of the anchors** of a path.
//!
//! The tree's rule: the anchors are the whole of a path's shape, and a
//! malformed anchor is refused rather than given a default — and a
//! command that is refused changes nothing.

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

/// A **rectangle** of ten by ten, and its id.
fn a_rectangle(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .expect("the rectangle is there")
}

/// The path of a node, as the document holds it.
fn path_of(session: &Session, id: u64) -> Json {
    let node = session.document().node(id).expect("the node is there");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The one contour of the path.
fn contour_of(session: &Session, id: u64) -> Json {
    path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .cloned()
        .expect("the path has a contour")
}

/// The **segments** of the contour.
fn segments_of(session: &Session, id: u64) -> Vec<Json> {
    contour_of(session, id)
        .get("segments")
        .and_then(|segments| segments.as_array())
        .cloned()
        .expect("the contour has segments")
}

/// Three anchors, with no handles: a triangle that is not closed.
fn three_anchors() -> Json {
    json!([
        { "x": 0.0, "y": 0.0 },
        { "x": 10.0, "y": 0.0 },
        { "x": 10.0, "y": 10.0 }
    ])
}

/// The anchors of a path are **set**, and the path is the shape they
/// describe.
#[test]
fn the_anchors_of_a_path_are_set() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.anchors.set",
            &json!({ "path": id, "subpaths": [{ "anchors": three_anchors() }] }),
        )
        .expect("the anchors are set");
    let segments = segments_of(&session, id);
    assert_eq!(segments.len(), 2, "three anchors are two sides");
    let end = segments[1].get("line").and_then(|line| line.get("to")).expect("the side has an end");
    assert_eq!(end.get("x").and_then(|x| x.as_f64()), Some(10.0), "the side goes to the third anchor");
}

/// A **closed** contour has the side that comes back to the first anchor.
#[test]
fn a_closed_contour_has_the_side_that_comes_back() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.anchors.set",
            &json!({ "path": id, "subpaths": [{ "anchors": three_anchors(), "closed": true }] }),
        )
        .expect("the anchors are set");
    let segments = segments_of(&session, id);
    assert_eq!(segments.len(), 3, "a closed contour of three anchors has three sides");
    let end = segments[2].get("line").and_then(|line| line.get("to")).expect("the side has an end");
    assert_eq!(end.get("x").and_then(|x| x.as_f64()), Some(0.0), "the last side comes back");
}

/// A **handle** on an anchor makes the side a curve.
#[test]
fn a_handle_makes_the_side_a_curve() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.anchors.set",
            &json!({
                "path": id,
                "subpaths": [{
                    "anchors": [
                        { "x": 0.0, "y": 0.0, "out": [5.0, 0.0] },
                        { "x": 10.0, "y": 0.0 }
                    ]
                }]
            }),
        )
        .expect("the anchors are set");
    let segments = segments_of(&session, id);
    let curve = segments[0].get("cubic").expect("the side is a curve");
    let c1 = curve.get("c1").expect("the curve has a first control point");
    assert_eq!(c1.get("x").and_then(|x| x.as_f64()), Some(5.0), "the handle is where it was put");
}

/// A path may have **more than one contour**, and the anchors of each
/// are set.
#[test]
fn a_path_may_have_more_than_one_contour() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.anchors.set",
            &json!({
                "path": id,
                "subpaths": [
                    { "anchors": three_anchors() },
                    { "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 5.0, "y": 5.0 }], "closed": true }
                ]
            }),
        )
        .expect("the anchors are set");
    let path = path_of(&session, id);
    let subpaths = path.get("subpaths").and_then(|subpaths| subpaths.as_array());
    assert_eq!(subpaths.map(|subpaths| subpaths.len()), Some(2), "the path has two contours");
}

/// A malformed anchor is **refused**, and the path is left as it was.
#[test]
fn a_malformed_anchor_is_refused_and_changes_nothing() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let before = path_of(&session, id);
    let outcome = session.execute(
        "vector.path.anchors.set",
        &json!({ "path": id, "subpaths": [{ "anchors": [{ "x": 0.0 }] }] }),
    );
    assert!(outcome.is_err(), "the anchor without a `y` is refused");
    assert_eq!(path_of(&session, id), before, "the path is as it was");
}

/// A contour with **no anchors** is refused.
#[test]
fn a_contour_without_anchors_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome =
        session.execute("vector.path.anchors.set", &json!({ "path": id, "subpaths": [{ "closed": true }] }));
    assert!(outcome.is_err(), "the contour without anchors is refused");
}

/// A contour of **one anchor** has no side, and is refused.
#[test]
fn a_contour_of_one_anchor_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute(
        "vector.path.anchors.set",
        &json!({ "path": id, "subpaths": [{ "anchors": [{ "x": 0.0, "y": 0.0 }] }] }),
    );
    assert!(outcome.is_err(), "the contour of one anchor is refused");
}

/// A path with **no contour** at all is refused.
#[test]
fn a_path_of_no_contour_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.anchors.set", &json!({ "path": id, "subpaths": [] }));
    assert!(outcome.is_err(), "the path of no contour is refused");
}

/// An anchor that is **smooth** is refused: a path here keeps no kind
/// for an anchor.
#[test]
fn a_smooth_anchor_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute(
        "vector.path.anchors.set",
        &json!({
            "path": id,
            "subpaths": [{
                "anchors": [
                    { "x": 0.0, "y": 0.0, "smooth": true },
                    { "x": 10.0, "y": 0.0 }
                ]
            }]
        }),
    );
    assert!(outcome.is_err(), "the smooth anchor is refused");
}

/// The **fill rule** is not part of the anchors, and it does not change.
#[test]
fn the_fill_rule_does_not_change() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session.execute("vector.path.rule", &json!({ "path": id, "rule": "evenodd" })).expect("the rule is set");
    session
        .execute(
            "vector.path.anchors.set",
            &json!({ "path": id, "subpaths": [{ "anchors": three_anchors() }] }),
        )
        .expect("the anchors are set");
    let path = path_of(&session, id);
    let rule = path.get("rule").and_then(|rule| rule.as_str());
    assert_eq!(rule, Some("even-odd"), "the rule is as it was: {rule:?}");
}
