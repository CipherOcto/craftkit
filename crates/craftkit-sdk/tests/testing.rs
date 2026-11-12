//! The test shape, written by a macro.
//!
//! These tests are about the **macro**, not about a command: they prove
//! that the macro runs the command the tree's way, that the parameters
//! arrive, that a refusal is not swallowed, and that the document is what
//! the command left.
//!
//! The commands are written **by hand** rather than with the derive, and
//! that is deliberate: the derive writes `crate::…` when it is used inside
//! the SDK itself, and a test that is an integration test is not the SDK.
//! So the commands here are submitted to the inventory directly, which is
//! what the link does with them anyway.

use craftkit_sdk::assert_command;
use craftkit_sdk::command::{CommandRun, CommandSpec};
use craftkit_sdk::{EngineError, Outcome, ParamDoc, ParamKind, Params, Session, Text, always};
use serde_json::json;

/// The name of the node the adding command adds.
const NODE_NAME: &str = "thing";

/// Say the name back.
///
/// The command reads its parameter, so the outcome is the parameter's
/// value: this is the command that proves the parameters arrive.
fn echo(_session: &mut Session, params: &Params<'_>) -> Result<Outcome, EngineError> {
    let name = Text::read(params, "name")
        .map_err(|source| EngineError::Param { id: "test.echo".to_string(), source })?;
    Ok(Outcome::new(format!("echo {}", name.0)))
}

/// Add a node to the document.
fn add(session: &mut Session, _params: &Params<'_>) -> Result<Outcome, EngineError> {
    let id = session.document_mut()?.add_node(NODE_NAME, "test.kind", None);
    Ok(Outcome::changed(format!("added {id}")))
}

/// Say how many nodes the document has.
fn count(session: &mut Session, _params: &Params<'_>) -> Result<Outcome, EngineError> {
    Ok(Outcome::new(format!("{}", session.document().nodes.len())))
}

/// Refuse, on purpose.
///
/// The refusal has words of its own, and the test is about those words.
fn refuse(_session: &mut Session, _params: &Params<'_>) -> Result<Outcome, EngineError> {
    Err(EngineError::Failed { id: "test.refuse".to_string(), message: "the grid is closed".to_string() })
}

/// The command that says the name back.
static TEST_ECHO: CommandSpec = CommandSpec {
    id: "test.echo",
    label: "Echo",
    area: "test",
    capability: "command",
    layer: "core",
    icon: "echo",
    doc: "Say the name back.",
    shortcut: None,
    menu: &["test"],
    params: &[ParamDoc::required("name", <Text as ParamKind>::DOC, "The name to say.")],
    mutates: false,
    undoable: false,
    example: false,
    journal: true,
    enabled: always,
    run: CommandRun::Static(echo),
};

/// The command that adds a node.
static TEST_ADD: CommandSpec = CommandSpec {
    id: "test.add",
    label: "Add",
    area: "test",
    capability: "command",
    layer: "core",
    icon: "plus",
    doc: "Add a node to the document.",
    shortcut: None,
    menu: &["test"],
    params: &[],
    mutates: true,
    undoable: true,
    example: false,
    journal: true,
    enabled: always,
    run: CommandRun::Static(add),
};

/// The command that counts the nodes.
static TEST_COUNT: CommandSpec = CommandSpec {
    id: "test.count",
    label: "Count",
    area: "test",
    capability: "command",
    layer: "core",
    icon: "count",
    doc: "Say how many nodes the document has.",
    shortcut: None,
    menu: &["test"],
    params: &[],
    mutates: false,
    undoable: false,
    example: false,
    journal: true,
    enabled: always,
    run: CommandRun::Static(count),
};

/// The command that refuses.
static TEST_REFUSE: CommandSpec = CommandSpec {
    id: "test.refuse",
    label: "Refuse",
    area: "test",
    capability: "command",
    layer: "core",
    icon: "stop",
    doc: "Refuse, on purpose.",
    shortcut: None,
    menu: &["test"],
    params: &[],
    mutates: false,
    undoable: false,
    example: false,
    journal: true,
    enabled: always,
    run: CommandRun::Static(refuse),
};

craftkit_sdk::inventory::submit! {
    craftkit_sdk::Registration { item: &TEST_ECHO }
}

craftkit_sdk::inventory::submit! {
    craftkit_sdk::Registration { item: &TEST_ADD }
}

craftkit_sdk::inventory::submit! {
    craftkit_sdk::Registration { item: &TEST_COUNT }
}

craftkit_sdk::inventory::submit! {
    craftkit_sdk::Registration { item: &TEST_REFUSE }
}

/// The message of the panic a body made, and nothing else.
///
/// A test that expects a failure has to catch it, and a caught panic prints
/// itself on the way out — so the hook is taken for the length of the catch
/// and put back after it.
fn panic_message(body: impl FnOnce()) -> String {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    std::panic::set_hook(previous);
    match caught {
        Ok(()) => String::new(),
        Err(payload) => match payload.downcast_ref::<String>() {
            Some(message) => message.clone(),
            None => String::from("the panic was not a message"),
        },
    }
}

/// The outcome is the **command's**.
///
/// The macro does not make an outcome of its own: the one the test sees is
/// the one the command returned.
#[test]
fn the_outcome_is_the_commands_own() {
    assert_command!(id = "test.count", params = {}, |_session| {}, |outcome| {
        assert_eq!(outcome.message, "0", "the outcome is not the command's");
    },);
}

/// The parameters reach the command.
///
/// The command reads `name`, and the outcome says it: a parameter that did
/// not arrive would be a different word in the outcome.
#[test]
fn the_parameters_reach_the_command() {
    assert_command!(
        id = "test.echo",
        params = { "name": "Book" },
        |_session| {},
        |outcome| {
            assert_eq!(outcome.message, "echo Book", "the parameter did not arrive");
        },
    );
}

/// A refused command fails the test with the **refusal's own words**.
///
/// The macro does not swallow the refusal and does not replace it with a
/// word of its own: the reader of the test needs what the command said.
#[test]
fn a_refused_command_fails_the_test_with_the_refusals_own_words() {
    let message = panic_message(|| {
        assert_command!(id = "test.refuse", params = {}, |_session| {}, |_outcome| {},);
    });
    assert!(message.contains("the grid is closed"), "the refusal's words are lost: {message}");
    assert!(message.contains("test.refuse"), "the refusal does not name the command: {message}");
}

/// A command that mutates changes the document.
///
/// The document is looked at after the command, through the session the
/// macro leaves behind.
#[test]
fn a_command_that_mutates_changes_the_document() {
    let session = assert_command!(id = "test.add", params = {}, |_session| {}, |outcome| {
        assert!(outcome.changed, "the command did not say it changed the document");
    },);
    assert_eq!(session.document().nodes.len(), 1, "the node was not added");
    assert_eq!(session.document().nodes[0].name, NODE_NAME, "the wrong node was added");
}

/// A command that does not mutate does not change the document.
///
/// The setup runs a command that **does** mutate, so the document has
/// something in it: the command under test must leave it exactly as it was.
#[test]
fn a_command_that_does_not_mutate_does_not_change_the_document() {
    let session = assert_command!(
        id = "test.count",
        params = {},
        |session| {
            session.execute("test.add", &json!({})).expect("the setup command ran");
        },
        |outcome| {
            assert_eq!(outcome.message, "1", "the command did not see the setup");
            assert!(!outcome.changed, "the command said it changed the document");
        },
    );
    assert_eq!(
        session.document().nodes.len(),
        1,
        "the document was changed by a command that does not mutate"
    );
}

/// An unknown parameter is **refused**, not ignored.
///
/// A parameter nobody declared is an error: the command says so, and the
/// test fails with those words rather than passing over it.
#[test]
fn an_unknown_parameter_is_refused_rather_than_ignored() {
    let message = panic_message(|| {
        assert_command!(
            id = "test.echo",
            params = { "colour": "red" },
            |_session| {},
            |_outcome| {},
        );
    });
    assert!(message.contains("colour"), "the refusal does not name the parameter: {message}");
    assert!(message.contains("is not a parameter of this command"), "the refusal is lost: {message}");
}

/// The setup runs **before** the command.
///
/// The setup closure holds the session, and the command under test sees
/// what the setup left.
#[test]
fn the_setup_runs_before_the_command() {
    let session = assert_command!(
        id = "test.count",
        params = {},
        |session| {
            session.execute("test.add", &json!({})).expect("the setup command ran");
        },
        |outcome| {
            assert_eq!(outcome.message, "1", "the command ran before the setup");
        },
    );
    assert_eq!(session.document().nodes.len(), 1, "the setup did not run");
}
