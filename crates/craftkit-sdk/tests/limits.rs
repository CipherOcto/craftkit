//! The **limits of the kit**: the numbers the kit keeps.
//!
//! A limit is a number the kit will not go past. Some of them are
//! **promises** — the bytes a document may hold — and the rest are the
//! kit's own caps: the depth of a raw parameter, the number of
//! questions a command may ask, the size of the journal. A limit that
//! no test names is a limit nothing proves holds.

use craftkit_sdk::bundle::File;
use craftkit_sdk::command::{CommandRun, CommandSpec};
use craftkit_sdk::contribution::Registry;
use craftkit_sdk::document::Document;
use craftkit_sdk::format::{FormatSpec, MigrationSpec};
use craftkit_sdk::params::{MAX_JSON_DEPTH, MAX_JSON_NODES, ParamKind};
use craftkit_sdk::prompt::Question;
use craftkit_sdk::resume::{MAX_QUESTIONS, MAX_RESUMES};
use craftkit_sdk::tasks::{TASK_LIMIT, TaskRegistry};
use craftkit_sdk::{EngineError, Outcome, Session};
use serde_json::json;

/// The journal's own limit.
const JOURNAL_LIMIT: usize = craftkit_sdk::journal::JOURNAL_LIMIT;

/// The bundle's own limit.
const MAX_FILES: usize = craftkit_sdk::bundle::MAX_FILES;

/// The history's own limit.
const HISTORY_LIMIT: usize = craftkit_sdk::history::HISTORY_LIMIT;

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
            example: false,
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

/// A raw value **nested deeper** than the limit allows is refused.
///
/// The depth is what a caller can hide behind: a value that is sixty-five
/// deep is a value that costs the reader stack.
#[test]
fn a_raw_parameter_that_is_too_deep_is_refused() {
    let mut value = serde_json::Value::Null;
    for _ in 0..=MAX_JSON_DEPTH {
        value = json!([value]);
    }
    let error = <serde_json::Value as ParamKind>::from_json(&value).expect_err("that value is too deep");
    assert!(error.to_string().contains("larger than the limit"), "{error}");
}

/// A raw value with **more values** than the limit allows is refused.
#[test]
fn a_raw_parameter_with_too_many_values_is_refused() {
    let value = serde_json::Value::Array(
        (0..=MAX_JSON_NODES).map(|_| serde_json::Value::Null).collect::<Vec<serde_json::Value>>(),
    );
    let error = <serde_json::Value as ParamKind>::from_json(&value).expect_err("that value is too big");
    assert!(error.to_string().contains("larger than the limit"), "{error}");
}

/// A command that asks for ever is **refused**.
///
/// A command that asks for ever is a command that never finishes, and
/// the host is the one that has to notice.
#[test]
fn a_command_that_asks_for_ever_is_refused() {
    let mut session = a_session_that_asks();
    let mut answered = 0usize;
    let message = loop {
        answered += 1;
        assert!(answered < MAX_QUESTIONS + 10, "the command asked for ever");
        match session.answer(json!("yes")) {
            Ok(_) => {}
            Err(EngineError::NeedsInput { .. }) => {}
            Err(error) => break error.to_string(),
        }
    };
    assert_eq!(answered, MAX_QUESTIONS, "the bound is the tree's number");
    assert!(message.contains(&format!("asked more than {MAX_QUESTIONS} questions")), "{message}");
}

/// The **answer** bound cannot refuse before the question bound does.
///
/// The two bounds are the same number, and every question a command asks
/// is one answer the host gave: the host answers, the command runs again
/// and asks the next question. So the question is checked first, on the
/// run that follows the last answer, and the answer bound never refuses
/// anything. A limit that cannot bite is a limit that cannot be relied
/// on, and this test says so.
#[test]
fn the_answer_bound_cannot_refuse_before_the_question_bound() {
    let mut session = a_session_that_asks();
    let mut answered = 0usize;
    let message = loop {
        answered += 1;
        assert!(answered < MAX_RESUMES + 10, "the command was answered for ever");
        match session.answer(json!("yes")) {
            Ok(_) => {}
            Err(EngineError::NeedsInput { .. }) => {}
            Err(error) => break error.to_string(),
        }
    };
    assert!(!message.contains("answered more than"), "the answer bound refused first: {message}");
    assert_eq!(MAX_QUESTIONS, MAX_RESUMES, "the two bounds are the same number");
}

/// A session with one command that **asks a question**, and nothing else.
fn a_session_that_asks() -> Session {
    let asked = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let counter = asked.clone();
    let mut commands: Registry<CommandSpec> = Registry::new();
    commands
        .register(CommandSpec {
            id: "test.ask",
            label: "Ask",
            area: "test",
            capability: "command",
            layer: "core",
            icon: "ask",
            doc: "Ask a question.",
            shortcut: None,
            menu: &["test"],
            params: &[],
            mutates: false,
            example: false,
            undoable: false,
            journal: false,
            enabled: craftkit_sdk::always,
            run: CommandRun::Dynamic(std::sync::Arc::new(move |session, _params| {
                let mut count = counter.lock().expect("the counter is not held twice");
                *count += 1;
                let name = format!("question {count}");
                drop(count);
                let answer = session.ask(&Question {
                    name,
                    kind: craftkit_sdk::Doc::Text,
                    help: "Say something.".to_string(),
                })?;
                Ok(Outcome::new(answer.to_string()))
            })),
        })
        .expect("the command is registered");
    let mut session = Session::new(commands, Registry::new());
    // The first run asks the first question.
    match session.execute("test.ask", &json!({})) {
        Ok(_) => {}
        Err(EngineError::NeedsInput { .. }) => {}
        Err(error) => panic!("the command could not ask: {error}"),
    }
    session
}

/// The registry of tasks is **bounded**.
///
/// A **finished** task is dropped when the registry is full: it is work
/// that is accounted for, and the registry keeps the handles of the work
/// that is not.
#[test]
fn the_registry_of_tasks_is_bounded() {
    let mut registry = TaskRegistry::with_budget(1024 * 1024);
    for _ in 0..(TASK_LIMIT + 10) {
        let id = registry.register("pdf.render");
        registry.complete(id, json!({ "pages": 0 })).expect("the task is finished");
    }
    assert!(registry.tasks().len() <= TASK_LIMIT, "the registry held {} tasks", registry.tasks().len());
}

/// The journal is **bounded**.
#[test]
fn the_journal_is_bounded() {
    let mut session = a_session();
    for _ in 0..(JOURNAL_LIMIT + 10) {
        session.execute("test.note", &json!({ "text": "a note" })).expect("the command ran");
    }
    assert!(session.journal().len() <= JOURNAL_LIMIT, "the journal held {} entries", session.journal().len());
}

/// A bundle with **too many files** is refused.
///
/// The count comes from the file, and a count that comes from outside is
/// capped: the bundle is refused, and not truncated into one that fits.
#[test]
fn a_bundle_with_too_many_files_is_refused() {
    let formats = Registry::<FormatSpec>::from_inventory().expect("the formats are there");
    let migrations = Registry::<MigrationSpec>::from_inventory().expect("the migrations are there");
    let files: Vec<File> = (0..=MAX_FILES).map(|index| File::new(format!("file {index}"), vec![0])).collect();
    let bytes = craftkit_sdk::bundle::write(&Document::new("bundled"), files, &formats, &migrations)
        .expect("the bundle is written");
    let error = craftkit_sdk::bundle::read(&bytes, &formats, &migrations).expect_err("it is refused");
    assert!(error.contains("files"), "{error}");
}

/// The **history** is bounded.
///
/// The tree holds a whole document per state, so the history is
/// bounded in the number of states it holds: the oldest state that is
/// not the current one is dropped when the tree is full, and a history
/// that grows for ever is a leak.
#[test]
fn the_history_is_bounded() {
    let mut history = craftkit_sdk::history::History::new(Document::new("the document"));
    for index in 0..(HISTORY_LIMIT + 10) {
        history.push("test.note", Document::new("the document"), Vec::new(), None);
        let _ = index;
    }
    assert!(history.len() <= HISTORY_LIMIT, "the history held {} states", history.len());
}

/// The **zoom** of a camera over `MAX_ZOOM` is **clamped**, not refused.
///
/// The camera is a value: a wheel event can send anything, and a camera
/// that refuses to move is a camera that feels broken. This is a
/// deliberate departure from the tree's rule that a number outside its
/// range is refused, and the tree's camera says so in its own words.
#[test]
fn a_zoom_over_the_limit_is_clamped() {
    let mut camera = craftkit_sdk::camera::Camera::new();
    camera.set_zoom(craftkit_sdk::camera::Camera::MAX_ZOOM * 10.0);
    assert_eq!(camera.zoom(), craftkit_sdk::camera::Camera::MAX_ZOOM, "the zoom is as big as it goes");
}
