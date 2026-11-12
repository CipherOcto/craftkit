//! The **bounds of a session's own memory**.
//!
//! The document has its own bounds (§63) and so does the session: the
//! history, the journal, the tasks and the log are all things a session
//! holds for as long as it lives, and a session that runs for days must
//! not grow for ever.
//!
//! Every one of these bounds is a **count** and a **number of bytes**,
//! and the bytes are the ones that bind: a count of entries says nothing
//! about how big an entry is.

use craftkit_sdk::command::{CommandRun, CommandSpec};
use craftkit_sdk::contribution::Registry;
use craftkit_sdk::document::{Document, MAX_DOCUMENT_BYTES};
use craftkit_sdk::history::History;
use craftkit_sdk::session::MAX_LOG_ENTRIES;
use craftkit_sdk::tasks::TaskRegistry;
use craftkit_sdk::{EngineError, Outcome, Session};
use serde_json::json;

/// The one parameter the test command takes.
const TEXT_PARAM: craftkit_sdk::error::ParamDoc =
    craftkit_sdk::error::ParamDoc::optional("text", craftkit_sdk::Doc::Text, "The text.");

/// A session with one command in it, and nothing else.
fn a_session() -> Session {
    let mut commands: Registry<CommandSpec> = Registry::new();
    commands
        .register(CommandSpec {
            id: "test.note",
            label: "Note",
            area: "test",
            capability: "command",
            layer: "core",
            icon: "note",
            doc: "Say something.",
            shortcut: None,
            menu: &["test"],
            params: &[TEXT_PARAM],
            mutates: false,
            undoable: false,
            journal: true,
            enabled: craftkit_sdk::always,
            run: CommandRun::Dynamic(std::sync::Arc::new(|_session, _params| {
                Ok(Outcome::new("noted".to_string()))
            })),
        })
        .expect("the command is registered");
    Session::new(commands, Registry::new())
}

/// A document whose payload holds `bytes` of text.
fn a_document_with(payload_bytes: usize) -> Document {
    let mut document = Document::new("big");
    let text = "x".repeat(payload_bytes);
    let node = document.add_node("heavy", "shape.rect", None);
    if let Some(node) = document.node_mut(node) {
        node.data = json!({ "payload": text });
    }
    document
}

/// A document over the byte budget is **refused**.
///
/// The counts — depth, nodes, relations — are all fine. What is not fine
/// is the memory, and a document that is thirty-three megabytes is a
/// document that a machine cannot hold.
#[test]
fn a_document_over_the_byte_budget_is_refused() {
    let document = a_document_with(MAX_DOCUMENT_BYTES + 1024);
    let error = document.check_bounds().expect_err("that document is too big");
    assert!(error.contains("bytes"), "{error}");
}

/// A document inside the budget is accepted.
#[test]
fn a_document_inside_the_byte_budget_is_accepted() {
    let document = a_document_with(1024);
    document.check_bounds().expect("that document fits");
}

/// The estimate never under-estimates.
///
/// A budget a document can lie about is not a budget: the text that is
/// really there has to be counted, not the length of the slice that
/// happens to hold it.
#[test]
fn the_estimate_never_under_estimates() {
    let payload = "x".repeat(4096);
    let document = a_document_with(4096);
    assert!(document.estimated_bytes() >= payload.len(), "the payload was not counted");
}

/// The history is pruned by its **bytes**, not only by its count.
///
/// A tree of small documents is bounded by the count; a tree of large
/// ones is bounded by the bytes — and the bytes are what the memory is.
#[test]
fn the_history_is_pruned_by_its_bytes() {
    let mut history = History::with_budget(a_document_with(1024), 4096);
    for _ in 0..8 {
        history.push("test.note", a_document_with(1024), Vec::new(), None);
    }
    assert!(history.bytes() <= history.budget(), "the history held {} bytes", history.bytes());
    assert!(history.len() < 8, "the history kept every state");
}

/// The state that is current is never dropped for the budget.
///
/// The history is the session's memory, and the current state is the
/// document: dropping it would lose the work.
#[test]
fn the_current_state_survives_the_budget() {
    let mut history = History::with_budget(a_document_with(1024), 2048);
    history.push("test.note", a_document_with(1024), Vec::new(), None);
    let current = history.current();
    assert!(history.node(current).is_some(), "the current state is gone");
}

/// The journal is pruned by its **bytes**.
///
/// An entry holds the parameters the caller sent, and the caller decides
/// how big those are. A count of entries says nothing about that.
#[test]
fn the_journal_is_pruned_by_its_bytes() {
    let mut session = a_session();
    session.set_journal_budget(4096);
    let payload = "x".repeat(2048);
    for _ in 0..8 {
        session.execute("test.note", &json!({ "text": payload })).expect("the command ran");
    }
    let bytes: usize = session.journal().iter().map(|entry| entry.bytes()).sum();
    assert!(bytes <= 4096, "the journal held {bytes} bytes");
    assert!(session.journal().len() < 8, "the journal kept every entry");
}

/// The registry of tasks is pruned by its **bytes**.
///
/// A finished task holds the result the work produced, and that result is
/// as big as the work made it.
#[test]
fn the_tasks_are_pruned_by_their_bytes() {
    let mut registry = TaskRegistry::with_budget(4096);
    let payload = "x".repeat(2048);
    for _ in 0..8 {
        let id = registry.register("pdf.render");
        registry.complete(id, json!({ "pages": payload.clone() })).expect("the task is finished");
    }
    assert!(registry.bytes() <= registry.budget(), "the registry held {} bytes", registry.bytes());
}

/// A task that is still running is not dropped for the budget.
///
/// It is work that has not been accounted for, and losing it would lose
/// the handle the other end knows it by.
#[test]
fn a_running_task_is_not_dropped_for_the_budget() {
    let mut registry = TaskRegistry::with_budget(16);
    let id = registry.register("pdf.render");
    registry.start(id).expect("the task starts");
    registry.complete(id, json!({ "pages": "x".repeat(2048) })).expect("the task is finished");
    let id = registry.register("pdf.render");
    registry.start(id).expect("the task starts");
    assert!(registry.task(id).is_some(), "the running task is gone");
}

/// The log is bounded.
///
/// It is written on every command, and a session that runs for days would
/// otherwise hold every line it ever wrote.
#[test]
fn the_log_is_bounded() {
    let mut session = a_session();
    for _ in 0..(MAX_LOG_ENTRIES + 10) {
        session.execute("test.note", &json!({})).expect("the command ran");
    }
    assert_eq!(session.log().len(), MAX_LOG_ENTRIES, "the log grew");
}

/// A command that fails is still a line in the log.
#[test]
fn a_refused_command_is_in_the_log_and_not_in_the_journal() {
    let mut session = a_session();
    let error = session.execute("test.nothing", &json!({})).expect_err("there is no such command");
    assert!(matches!(error, EngineError::Unknown(_)), "{error:?}");
    assert_eq!(session.log().len(), 1, "the refusal was not written down");
    assert!(session.journal().is_empty(), "a command that did not run is in the journal");
}
