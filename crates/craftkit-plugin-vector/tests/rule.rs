//! The **rule** that says what is inside a path, and the
//! **exclusion** of two paths.
//!
//! The tree's path says which parts of it are inside, and it can keep
//! what two paths do not share. v2's path had a rule that nothing
//! could change, and a library that could exclude and no command that
//! said so.

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

/// A **rectangle** of ten by ten at a place, and its id.
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

/// The path of a node, as the document holds it.
fn path_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(id).expect("the node is there");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The rule of a path, as the document holds it.
fn rule_of(session: &Session, id: u64) -> String {
    path_of(session, id).get("rule").and_then(|rule| rule.as_str()).unwrap_or("").to_string()
}

/// A path may say which parts of it are **inside**.
#[test]
fn a_path_may_say_what_is_inside_it() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0);
    assert_eq!(rule_of(&session, id), "non-zero", "the rule is the default one");
    session.execute("vector.path.rule", &json!({ "path": id, "rule": "evenodd" })).expect("the rule is set");
    assert_eq!(rule_of(&session, id), "even-odd", "the rule is the one that was asked for");
}

/// A rule that is **not a rule** is refused by name.
#[test]
fn a_rule_that_is_not_a_rule_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0);
    let refused = session.execute("vector.path.rule", &json!({ "path": id, "rule": "winding" }));
    let message = match refused {
        Ok(_) => panic!("a rule that is not a rule is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("nonzero"), "the refusal says why: {message}");
}

/// Two paths may be **excluded**: what they share is taken away, and
/// what they do not share is kept.
#[test]
fn two_paths_may_be_excluded() {
    let mut session = session();
    let first = a_rectangle(&mut session, 0.0, 0.0);
    let second = a_rectangle(&mut session, 5.0, 0.0);
    session
        .execute("vector.path.exclude", &json!({ "first": first, "second": second }))
        .expect("the paths are excluded");
    // The two paths are one now, and the second is gone.
    let paths: Vec<u64> =
        session.document().walk().filter(|node| node.kind == "vector.path").map(|node| node.id).collect();
    assert_eq!(paths.len(), 1, "the paths are one now: {paths:?}");
    let gone = session.document().node(second);
    assert!(gone.is_none(), "the second path is gone");
    // The result is the first path without the overlap: what is left
    // is in two pieces, and the overlap is in neither.
    let path = path_of(&session, first);
    let pieces = path.get("subpaths").and_then(|subpaths| subpaths.as_array()).map(|subpaths| subpaths.len());
    assert_eq!(pieces, Some(2), "what is left is in two pieces: {path}");
}
