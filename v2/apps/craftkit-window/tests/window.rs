//! The **window**: thin, and it says so.
//!
//! A front end reads the engine's state and acts through the registry.
//! These tests are about the window doing exactly that: the commands it
//! shows are the registry's, the document it shows is the document,
//! and the only way it changes anything is the session's one way in.

use craftkit_sdk::serde_json::json;
use craftkit_window::Window;

/// A window over the kit.
fn window() -> Window {
    Window::new().expect("the ecosystem is built")
}

/// The window shows **every command that was linked in**.
///
/// It has no list of its own: the commands it shows are the registry's,
/// and a command that is added to the registry appears without the
/// window being told.
#[test]
fn the_window_shows_every_command() {
    let window = window();
    let commands = window.commands();
    assert!(!commands.is_empty(), "there are commands");
    let ids: Vec<&str> = commands.iter().map(|command| command.id).collect();
    assert!(ids.contains(&"node.add"), "the core's own command is there: {ids:?}");
    assert!(ids.contains(&"hello.greet"), "and a plugin's: {ids:?}");
    // The commands are in order, so the window draws them the same way
    // every time.
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "the commands are in order");
}

/// The window runs a command **through the registry**.
///
/// The document changes, and the only way it changed is the session's
/// one way in.
#[test]
fn the_window_runs_a_command_through_the_registry() {
    let mut window = window();
    assert!(window.open("node.add"), "the command opens");
    assert!(window.set("name", json!("a shape")), "the name is set");
    assert!(window.set("kind", json!("geometry.rect")), "and the kind");
    let outcome = window.run().expect("the command runs");
    assert!(outcome.message.contains("a shape"), "{}", outcome.message);

    let tree = window.tree();
    assert_eq!(tree.len(), 1, "the document has a node: {tree:?}");
    assert_eq!(tree[0].name, "a shape");
    assert_eq!(tree[0].kind, "geometry.rect");
    assert_eq!(window.log().len(), 1, "and the window said so");
}

/// The window shows the **plan**: what the document means.
///
/// It is read from the session, and it is read again every time it is
/// asked for: a window that kept it would show a document that is not
/// there any more.
#[test]
fn the_window_shows_the_plan() {
    let mut window = window();
    window.open("node.add");
    window.set("name", json!("a shape"));
    window.set("kind", json!("geometry.rect"));
    window.run().expect("the command runs");

    let plan = window.plan();
    let about = plan.iter().find(|row| row.node == 1);
    assert!(about.is_some(), "the plan says what the node means: {plan:?}");
    assert_eq!(about.map(|row| row.kind.clone()), Some("geometry.rect".to_string()));

    // The plan is read again, and it is the same: the window keeps
    // nothing.
    assert_eq!(window.plan(), plan, "the plan is read again");
}

/// A command that is **not there** opens nothing.
#[test]
fn a_command_that_is_not_there_opens_nothing() {
    let mut window = window();
    assert!(!window.open("nothing.here"), "there is no such command");
    assert!(window.form().is_none(), "and no form was opened");
    assert!(window.run().is_err(), "and there is nothing to run");
}

/// A command that **refuses** says so, and the window says it too.
#[test]
fn a_command_that_refuses_says_so() {
    let mut window = window();
    // Grouping wants two nodes, and the document has none.
    assert!(window.open("node.group"), "the command opens");
    let error = window.run().expect_err("there is nothing to group");
    assert!(
        window.log().iter().any(|line| line.contains("refused")),
        "the window said so: {:?}",
        window.log()
    );
    assert!(error.to_string().contains("group"), "{error}");
}
