//! A command that is **made while the program runs**.
//!
//! A command that was compiled has a struct, and the macro writes the
//! declaration, the reader and the registration from it. A command that
//! arrives after the program was built has none of that — so the SDK
//! writes the declaration, and the body reads its parameter with the
//! SDK's reader.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{CommandSpec, Doc, Either, EngineError, Number, Outcome, ParamDoc, Session, Text};

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

/// A refusal says **which** parameter was refused.
///
/// A refusal that does not name the parameter is a refusal the caller
/// has to guess about, and the reader knows the name it read from.
#[test]
fn a_refusal_names_the_parameter() {
    let (_, mut session) = runtime_echo();
    let error = session.execute("runtime.echo", &json!({ "text": 42 })).expect_err("a number is not a text");
    let message = error.to_string();
    assert!(message.contains("text"), "the refusal names the parameter: {message}");
    assert!(message.contains("a number"), "and says what was wrong: {message}");
}

/// A command that **lies** about its parameter is refused.
///
/// A command that was compiled cannot lie: the compiler wrote the
/// declaration and the reader from the same struct. A command that was
/// **made while the program runs** has a declaration written by hand, and
/// the hand may say a text while the body reads a number.
///
/// The reader is happy — the caller sent the number the body wanted — and
/// the declaration says otherwise. So the call is refused, and the
/// refusal says what was declared and what arrived.
#[test]
fn a_declaration_that_lies_is_refused() {
    let mut commands = craftkit_sdk::Registry::new();
    let command = CommandSpec::dynamic(
        "runtime.liar",
        "Liar",
        "runtime",
        "Say a number back.",
        vec![ParamDoc::dynamic("value", Doc::Text, true, "What to say.")],
        |_session, params| {
            let number = params.param::<Number>("value", "runtime.liar")?;
            Ok(Outcome::new(format!("{}", number.0)))
        },
    );
    commands.register(command).expect("the command is registered");
    let mut session = Session::new(commands, craftkit_sdk::Registry::new());

    let error = session.execute("runtime.liar", &json!({ "value": 42 })).expect_err("the declaration lies");
    assert!(matches!(error, EngineError::Param { .. }), "{error:?}");
    let message = error.to_string();
    assert!(message.contains("value"), "the refusal names the parameter: {message}");
    assert!(message.contains("declared as text"), "and says what it declared: {message}");
    assert!(message.contains("a number"), "and what arrived: {message}");
}

/// A declaration that tells the truth is **not** refused.
///
/// The check is on the value and the declaration, and it must not refuse a
/// call that is right.
#[test]
fn a_declaration_that_tells_the_truth_is_accepted() {
    let mut commands = craftkit_sdk::Registry::new();
    let command = CommandSpec::dynamic(
        "runtime.truth",
        "Truth",
        "runtime",
        "Say a number back.",
        vec![ParamDoc::dynamic("value", Doc::Number, true, "What to say.")],
        |_session, params| {
            let number = params.param::<Number>("value", "runtime.truth")?;
            Ok(Outcome::new(format!("{}", number.0)))
        },
    );
    commands.register(command).expect("the command is registered");
    let mut session = Session::new(commands, craftkit_sdk::Registry::new());

    let outcome = session.execute("runtime.truth", &json!({ "value": 42 })).expect("the command runs");
    assert_eq!(outcome.message, "42");
}

/// A command that joins a session has to be in a layer that is **linked
/// in**.
///
/// The ecosystem refuses a contribution in a layer that is not there, and
/// the session refuses a command that joins it in one.
#[test]
fn a_command_joins_only_in_a_layer_that_is_linked_in() {
    let mut session = Session::new(craftkit_sdk::Registry::new(), craftkit_sdk::Registry::new());
    let mut command = CommandSpec::dynamic(
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
    command.layer = "not-a-layer";
    let error = session.register(command).expect_err("there is no such layer");
    assert!(error.to_string().contains("not-a-layer"), "{error}");
}

/// The core is linked in, so a command in the core joins.
#[test]
fn a_command_joins_in_the_core_layer() {
    let mut session = Session::new(craftkit_sdk::Registry::new(), craftkit_sdk::Registry::new());
    let (command, _) = runtime_echo();
    session.register(command).expect("the core is linked in");
}

/// A **required** parameter the caller did not give is refused.
///
/// The declaration says the parameter is required, and the call did not
/// give it — so the call is refused at the boundary, with the reader's
/// own words.
#[test]
fn a_required_parameter_that_was_not_given_is_refused() {
    let (_, mut session) = runtime_echo();
    let error = session.execute("runtime.echo", &json!({})).expect_err("the text is required");
    assert!(matches!(error, EngineError::Param { .. }), "{error:?}");
    assert!(error.to_string().contains("is required"), "{error}");
}

/// An **optional** parameter the caller did not give is not a refusal.
///
/// The declaration says the parameter is optional, and the body does not
/// read it, so the call runs.
#[test]
fn an_optional_parameter_that_was_not_given_is_not_a_refusal() {
    let mut commands = craftkit_sdk::Registry::new();
    let command = CommandSpec::dynamic(
        "runtime.quiet",
        "Quiet",
        "runtime",
        "Say nothing much.",
        vec![ParamDoc::dynamic("text", Doc::Text, false, "What to say, if anything.")],
        |_session, _params| Ok(Outcome::new("said nothing")),
    );
    commands.register(command).expect("the command is registered");
    let mut session = Session::new(commands, craftkit_sdk::Registry::new());

    let outcome = session.execute("runtime.quiet", &json!({})).expect("the command runs");
    assert_eq!(outcome.message, "said nothing");
}

/// A parameter with **two spellings** is required as a pair.
///
/// Either spelling is enough, and neither on its own is.
#[test]
fn a_pair_of_spellings_is_required_as_a_pair() {
    let mut commands = craftkit_sdk::Registry::new();
    let command = CommandSpec::dynamic(
        "runtime.pair",
        "Pair",
        "runtime",
        "Take one of two.",
        vec![ParamDoc::either("node", Doc::Text, "The node's id.", "node_name")],
        |_session, params| {
            let either = Either::<Text, Text>::read_either(params, "node", "node_name")
                .map_err(|error| EngineError::Param { id: "runtime.pair".to_string(), source: error })?;
            let text = match either {
                Either::Left(text) => text,
                Either::Right(text) => text,
            };
            Ok(Outcome::new(text.0))
        },
    );
    commands.register(command).expect("the command is registered");
    let mut session = Session::new(commands, craftkit_sdk::Registry::new());

    // Either spelling is enough.
    let outcome = session.execute("runtime.pair", &json!({ "node": "box" })).expect("the command runs");
    assert_eq!(outcome.message, "box");
    let outcome = session.execute("runtime.pair", &json!({ "node_name": "box" })).expect("the command runs");
    assert_eq!(outcome.message, "box");

    // And neither is a refusal.
    let error = session.execute("runtime.pair", &json!({})).expect_err("the pair is required");
    assert!(error.to_string().contains("or `node_name` instead of it"), "{error}");
}
