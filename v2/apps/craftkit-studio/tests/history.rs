//! The history is a **tree**, not a stack.
//!
//! A line of states can only be walked backwards. A tree can be branched:
//! an edit that is undone and then done differently keeps the work that
//! was undone, one undo away, on the other side of the change.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with three nodes in it.
fn three_nodes() -> Session {
    let mut session = ecosystem().session();
    for name in ["one", "two", "three"] {
        session
            .execute("node.add", &json!({ "name": name, "kind": "geometry.rect" }))
            .expect("the node is added");
    }
    session
}

/// Undo goes back one step, and redo goes forward again.
#[test]
fn undo_goes_back_and_redo_goes_forward() {
    let mut session = three_nodes();
    assert_eq!(session.document().nodes.len(), 3);
    session.undo().expect("the last node is undone");
    assert_eq!(session.document().nodes.len(), 2, "one node went");
    session.redo(None).expect("the node is back");
    assert_eq!(session.document().nodes.len(), 3, "the node came back");
}

/// Undo takes the selection and the tool with it.
#[test]
fn undo_takes_the_selection_and_the_tool_with_it() {
    let mut session = three_nodes();
    session.execute("select.node", &json!({ "node": 2 })).expect("the node is selected");
    session.execute("tool.hold", &json!({ "tool": "tool.rect" })).expect("the tool is held");
    session
        .execute("node.add", &json!({ "name": "four", "kind": "geometry.rect" }))
        .expect("the node is added");
    session.undo().expect("the node is undone");
    assert_eq!(session.selected_ids(), &[2], "the selection came back");
    assert_eq!(session.tool(), Some("tool.rect"), "the tool came back");
}

/// A change made after an undo makes a **branch**.
///
/// The work that was undone is not thrown away: it is still there, and
/// redo goes back into it.
#[test]
fn a_change_after_an_undo_makes_a_branch() {
    let mut session = three_nodes();
    session.undo().expect("the last node is undone");
    assert_eq!(session.document().nodes.len(), 2);
    session
        .execute("node.add", &json!({ "name": "other", "kind": "geometry.rect" }))
        .expect("the other node is added");
    assert_eq!(session.document().nodes.len(), 3, "the other node is there");

    // The state that was undone to has two children now: the work that
    // was undone, and the work that replaced it.
    let branch_point = session.history().parent().expect("the branch point is there");
    let branches = session.history().node(branch_point).expect("the state is there").children.clone();
    assert_eq!(branches.len(), 2, "there are two branches: {branches:?}");

    // Undo again, and redo without a branch takes the newest one: the
    // work that was just done.
    session.undo().expect("the other node is undone");
    session.redo(None).expect("the newest branch is taken");
    assert_eq!(session.document().nodes[2].name, "other");

    // Redo with a branch takes the one that is named, and the work that
    // was undone is still there.
    let old = branches[0];
    session.undo().expect("the other node is undone");
    session.redo(Some(old)).expect("the old branch is taken");
    assert_eq!(session.document().nodes[2].name, "three", "the old work came back");
}

/// The tree is a tree: the states have parents and children.
#[test]
fn the_tree_has_parents_and_children() {
    let session = three_nodes();
    let history = session.history();
    assert_eq!(history.len(), 3, "three steps: {:?}", history.nodes().len());
    let path = history.path();
    assert_eq!(path.len(), 4, "the root and three steps: {path:?}");
    let current = history.current();
    let node = history.node(current).expect("the current state is there");
    assert_eq!(node.command, "node.add", "the state knows its command");
    assert!(node.parent.is_some(), "the state has a parent");
    assert_eq!(node.children.len(), 0, "the last state has no children");
}

/// The tree is a command like any other.
#[test]
fn the_tree_is_a_command() {
    let mut session = three_nodes();
    session.undo().expect("the last node is undone");
    let outcome = session.execute("history.tree", &json!({})).expect("the tree is read");
    let data = outcome.data;
    assert_eq!(data["current"], 2, "the current state is the second one");
    assert_eq!(data["branches"].as_array().map(|branches| branches.len()), Some(1), "{data}");

    session.execute("history.undo", &json!({})).expect("the node is undone");
    assert_eq!(session.document().nodes.len(), 1, "two nodes went");
    session.execute("history.redo", &json!({})).expect("the node is back");
    assert_eq!(session.document().nodes.len(), 2, "the node came back");
}

/// Going to a state takes the session there, and the work after it is
/// kept.
#[test]
fn a_state_can_be_visited() {
    let mut session = three_nodes();
    let first = session.history().path()[1];
    session.execute("history.go", &json!({ "state": first })).expect("the state is visited");
    assert_eq!(session.document().nodes.len(), 1, "the document is the one that was there");
    assert_eq!(session.history().branches().len(), 1, "the work after it is kept");
}

/// The tree is **bounded**: a session that runs for days does not grow
/// without limit.
#[test]
fn the_history_is_bounded() {
    let mut session = ecosystem().session();
    for index in 0..(craftkit_sdk::history::HISTORY_LIMIT + 40) {
        session
            .execute("node.add", &json!({ "name": format!("n{index}"), "kind": "geometry.rect" }))
            .expect("the node is added");
    }
    assert!(
        session.history().nodes().len() <= craftkit_sdk::history::HISTORY_LIMIT,
        "the history grew to {}",
        session.history().nodes().len()
    );
    // The current state is still there, and the document is not smaller
    // than the tree can account for.
    assert!(!session.document().nodes.is_empty(), "the nodes are there");
}

/// A batch is **one** state, not one per command inside it.
#[test]
fn a_batch_is_one_state() {
    let mut session = ecosystem().session();
    session
        .batch(|session| {
            session.execute("node.add", &json!({ "name": "one", "kind": "geometry.rect" }))?;
            session.execute("node.add", &json!({ "name": "two", "kind": "geometry.rect" }))?;
            Ok(())
        })
        .expect("the batch ran");
    assert_eq!(session.history().len(), 1, "one state: {:?}", session.history().len());
    session.undo().expect("the batch is undone");
    assert!(session.document().nodes.is_empty(), "both nodes went at once");
}

/// Undoing with nothing to undo is refused, and says so.
#[test]
fn undoing_with_nothing_to_undo_is_refused() {
    let mut session = ecosystem().session();
    let error = session.undo().expect_err("there is nothing to undo");
    assert!(error.to_string().contains("nothing to undo"), "{error}");
}

/// The undo is a **command**.
///
/// Everything a person or an agent can do is a command, and the history
/// is not an exception: the undo and the redo are in the registry, and
/// a front end that opened the session's own door would be a front end
/// with a second way into the design.
#[test]
fn the_undo_is_a_command() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("doc.undo").is_some(), "the undo is a command");
    assert!(ecosystem.commands.get("doc.redo").is_some(), "and so is the redo");

    let mut session = three_nodes();
    assert_eq!(session.document().nodes.len(), 3);
    session.execute("doc.undo", &json!({})).expect("the undo runs");
    assert_eq!(session.document().nodes.len(), 2, "one node went");
    session.execute("doc.redo", &json!({})).expect("the redo runs");
    assert_eq!(session.document().nodes.len(), 3, "the node came back");
}
