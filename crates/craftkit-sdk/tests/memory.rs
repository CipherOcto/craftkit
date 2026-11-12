//! The **session's own memory**, and the one budget that covers it.
//!
//! §65 bounded each part of a session on its own. This file is about the
//! **whole**: a session that is over its budget has to give something up,
//! and what it gives up is decided by what is cheapest to lose — and a
//! command that leaves the document too big is refused and put back.

use craftkit_sdk::command::{CommandRun, CommandSpec};
use craftkit_sdk::contribution::Registry;
use craftkit_sdk::document::MAX_DOCUMENT_BYTES;
use craftkit_sdk::session::{MAX_SESSION_BYTES, Memory};
use craftkit_sdk::{EngineError, Outcome, Session};
use serde_json::json;

/// The parameter the test command takes.
const BYTES_PARAM: craftkit_sdk::error::ParamDoc =
    craftkit_sdk::error::ParamDoc::optional("bytes", craftkit_sdk::Doc::Number, "How many bytes.");

/// A session with a command that adds a node with a payload of `bytes`.
///
/// It is a **mutating** command, so it makes an undo step, and it is
/// **journaled**, so it writes an entry.
fn a_session() -> Session {
    // The kit's own commands are collected from the inventory, so the
    // session has the ones the kit ships — `session.memory` among them.
    let mut commands: Registry<CommandSpec> =
        Registry::from_inventory().expect("the kit's commands are collected");
    commands
        .register(CommandSpec {
            id: "test.fill",
            label: "Fill",
            area: "test",
            capability: "command",
            layer: "core",
            icon: "note",
            doc: "Add a node with a payload.",
            shortcut: None,
            menu: &["test"],
            params: &[BYTES_PARAM],
            mutates: true,
            example: false,
            undoable: true,
            journal: true,
            enabled: craftkit_sdk::always,
            run: CommandRun::Dynamic(std::sync::Arc::new(|session, params| {
                let bytes = params.get("bytes").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
                let payload = "x".repeat(bytes);
                let id = session.document_mut()?.add_node("heavy", "shape.rect", None);
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = json!({ "payload": payload });
                }
                Ok(Outcome::changed("filled".to_string()))
            })),
        })
        .expect("the command is registered");
    Session::new(commands, Registry::new())
}

/// A command that made the document too big is **refused**, and the
/// document is put back.
///
/// The bound used to be checked only when the document was written, and
/// §63 said so: a document that is *built* in memory was bounded only
/// when it was saved. Now the session judges it after every command.
#[test]
fn a_command_that_makes_the_document_too_big_is_refused() {
    let mut session = a_session();
    let error = session
        .execute("test.fill", &json!({ "bytes": MAX_DOCUMENT_BYTES + 1024 }))
        .expect_err("that document is too big");
    assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    assert_eq!(session.document().nodes.len(), 0, "the document was not put back");
}

/// A command that leaves the document inside its bound is accepted.
#[test]
fn a_command_that_keeps_the_document_inside_its_bound_is_accepted() {
    let mut session = a_session();
    session.execute("test.fill", &json!({ "bytes": 1024 })).expect("the command ran");
    assert_eq!(session.document().nodes.len(), 1, "the node is there");
}

/// The session says what it is holding.
///
/// The parts and not one number: a session that is over its budget has
/// to give something up, and the part that gives way is the part that is
/// cheapest to lose.
#[test]
fn the_session_says_what_it_is_holding() {
    let mut session = a_session();
    session.execute("test.fill", &json!({ "bytes": 4096 })).expect("the command ran");
    let outcome = session.execute("session.memory", &json!({})).expect("the command ran");
    let data = outcome.data;
    assert!(data.get("document").and_then(|bytes| bytes.as_u64()).unwrap_or(0) > 0, "{data}");
    assert!(data.get("history").and_then(|bytes| bytes.as_u64()).unwrap_or(0) > 0, "{data}");
    assert_eq!(data.get("inside").and_then(|inside| inside.as_bool()), Some(true), "{data}");
}

/// The journal gives way first.
///
/// It is a record of what was asked for, and the newest entries are the
/// useful ones: it is the cheapest thing a session holds to lose.
#[test]
fn the_journal_gives_way_first() {
    let mut session = a_session();
    session.set_memory_budget(64 * 1024);
    for _ in 0..8 {
        session.execute("test.fill", &json!({ "bytes": 8192 })).expect("the command ran");
    }
    assert!(session.journal().len() <= 1, "the journal kept {} entries", session.journal().len());
    assert_eq!(session.document().nodes.len(), 8, "the document was given up");
}

/// The history gives way last.
///
/// It is the work: a person undoes with it. The journal and the finished
/// tasks go first, and the tree is what is left when everything else has
/// gone.
#[test]
fn the_history_gives_way_last() {
    let mut session = a_session();
    session.execute("test.fill", &json!({ "bytes": 8192 })).expect("the command ran");
    let before = session.history().len();
    assert!(before > 0, "the command made no state");

    // A budget that the document alone is over: nothing else can be
    // given up, so the history is.
    session.set_memory_budget(1024);
    session.execute("test.fill", &json!({ "bytes": 8192 })).expect("the command ran");
    assert!(session.history().len() < before, "the history kept every state");
    assert_eq!(session.document().nodes.len(), 2, "the document was given up");
}

/// The document is never given up.
///
/// It is the work. A budget may cost the history, the journal and the
/// tasks — and the document is not one of them.
#[test]
fn the_document_is_never_given_up() {
    let mut session = a_session();
    session.set_memory_budget(1024);
    session.execute("test.fill", &json!({ "bytes": 4096 })).expect("the command ran");
    assert_eq!(session.document().nodes.len(), 1, "the document was given up");
}

/// The budget is the family's.
///
/// The name and the number are the ones the family uses, so that a reader
/// of either tree can look them up: a session is a session.
#[test]
fn the_session_budget_is_the_familys() {
    assert_eq!(MAX_SESSION_BYTES, MAX_DOCUMENT_BYTES * 5);
}

/// The parts add up to one number.
#[test]
fn the_parts_add_up_to_one_number() {
    let memory = Memory { document: 10, history: 20, journal: 30, tasks: 40, log: 50 };
    assert_eq!(memory.total(), 150);
}
