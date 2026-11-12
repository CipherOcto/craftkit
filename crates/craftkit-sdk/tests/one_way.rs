//! The **one way**: the document is handed out only to a command.
//!
//! A program that could change the document without the registry would
//! have a second path to it — a path with no snapshot, no undo step and
//! nothing in the log. The family tree refuses that, and so does this
//! one: the session knows whether a command is running, and outside one
//! the document is a refusal.

use craftkit_sdk::command::{CommandRun, CommandSpec};
use craftkit_sdk::contribution::Registry;
use craftkit_sdk::{EngineError, Outcome, Session};
use serde_json::json;

/// A command that adds a node.
fn a_command() -> CommandSpec {
    CommandSpec {
        id: "test.add",
        label: "Add",
        area: "test",
        capability: "command",
        layer: "core",
        icon: "plus",
        doc: "Add a node.",
        shortcut: None,
        menu: &["test"],
        params: &[],
        mutates: true,
        example: false,
        undoable: true,
        journal: true,
        enabled: craftkit_sdk::always,
        run: CommandRun::Dynamic(std::sync::Arc::new(|session, _params| {
            let id = session.document_mut()?.add_node("node", "test.kind", None);
            Ok(Outcome::changed(format!("added {id}")))
        })),
    }
}

/// A session with that command in it.
fn a_session() -> Session {
    let mut commands: Registry<CommandSpec> = Registry::new();
    commands.register(a_command()).expect("the command is registered");
    Session::new(commands, Registry::new())
}

/// The document is **not** handed out when no command is running.
///
/// This is the second path closed: there is no way to change the
/// document that does not go through the registry.
#[test]
fn the_document_is_not_handed_out_outside_a_command() {
    let mut session = a_session();
    let error = session.document_mut().expect_err("no command is running");
    assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    assert!(error.to_string().contains("no command is running"), "{error}");
}

/// A command **may** have the document.
///
/// The refusal is about the *caller* and not about the document: inside a
/// command the document is handed over, because that is what a command
/// is for.
#[test]
fn a_command_may_have_the_document() {
    let mut session = a_session();
    session.execute("test.add", &json!({})).expect("the command ran");
    assert_eq!(session.document().nodes.len(), 1, "the node was not added");
}

/// The change a command made is **judged**.
///
/// The command's change goes through the same door as every other one:
/// the session takes a snapshot, compares, and undoes what it must.
#[test]
fn a_command_s_change_is_judged() {
    let mut session = a_session();
    session.execute("test.add", &json!({})).expect("the command ran");
    assert_eq!(session.history().len(), 1, "there is no undo step");
    assert_eq!(session.log().len(), 1, "the command is not in the log");
    assert_eq!(session.journal().len(), 1, "the command is not in the journal");
}
