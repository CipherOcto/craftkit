//! A command that is **made while the program runs**.
//!
//! A command that was compiled has a struct, and the macro writes the
//! declaration, the reader and the registration from it. A command that
//! arrives after the program was built has none of that — so the SDK
//! writes the declaration, and the body reads its parameter with the
//! SDK's reader.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{CommandSpec, Doc, EngineError, Outcome, ParamDoc, Session, Text};

/// A command made at run time, and the session it is in.
fn runtime_echo() -> (CommandSpec, Session) {
    let mut commands = craftkit_sdk::Registry::new();
    let command = CommandSpec::dynamic(
        "runtime.echo",
        "Echo",
        "runtime",
        "Say the text back.",
        vec![ParamDoc::dynamic("text", Doc::Text, true, "What to say.")],
        |_session, params| {
            let text = params.param::<Text>("text", "runtime.echo")?;
            Ok(Outcome::new(text.0))
        },
    );
    commands.register(command.clone()).expect("the command is registered");
    (command, Session::new(commands, craftkit_sdk::Registry::new()))
}

/// The command is **declared**, not written out.
///
/// The fields the caller does not say are the house ones: the command
/// does not change the document, it is not undoable, and it is written
/// in the journal.
#[test]
fn a_run_time_command_is_declared() {
    let (command, _) = runtime_echo();
    assert_eq!(command.id, "runtime.echo");
    assert_eq!(command.label, "Echo");
    assert_eq!(command.area, "runtime");
    assert_eq!(command.capability, "command");
    assert_eq!(command.layer, "core");
    assert_eq!(command.doc, "Say the text back.");
    assert_eq!(command.menu, &["runtime"]);
    assert!(!command.mutates, "it does not change the document");
    assert!(!command.undoable, "and it is not an undo step");
    assert!(command.journal, "and it is in the journal");
    assert_eq!(command.params.len(), 1, "it takes one parameter");
    assert_eq!(command.params[0].name, "text");
    assert_eq!(command.params[0].kind, Doc::Text);
    assert!(command.params[0].required, "and it is required");
}

/// The command runs, and it says what it was asked to say.
#[test]
fn a_run_time_command_runs() {
    let (_, mut session) = runtime_echo();
    let outcome = session.execute("runtime.echo", &json!({ "text": "hello" })).expect("the command runs");
    assert_eq!(outcome.message, "hello");
}

/// The parameter is read by the **SDK's reader**, and the bound is the
/// SDK's.
///
/// A command that was made at run time has no struct, and the bound does
/// not drift with it.
#[test]
fn the_bound_of_a_run_time_parameter_is_the_sdks() {
    let (_, mut session) = runtime_echo();
    let too_long = "x".repeat(craftkit_sdk::params::MAX_TEXT_CHARACTERS + 1);
    let error =
        session.execute("runtime.echo", &json!({ "text": too_long })).expect_err("the text is too long");
    assert!(matches!(error, EngineError::Param { .. }), "{error:?}");
}

/// A parameter that is not the kind the command declared is refused.
#[test]
fn a_run_time_command_refuses_a_value_of_the_wrong_kind() {
    let (_, mut session) = runtime_echo();
    let error = session.execute("runtime.echo", &json!({ "text": 42 })).expect_err("a number is not a text");
    assert!(matches!(error, EngineError::Param { .. }), "{error:?}");
}

/// A parameter that is not there is refused, and it is the **engine**
/// that refuses it.
#[test]
fn a_run_time_command_refuses_a_missing_parameter() {
    let (_, mut session) = runtime_echo();
    let error = session.execute("runtime.echo", &json!({})).expect_err("the text is missing");
    assert!(error.to_string().contains("text"), "{error}");
}
