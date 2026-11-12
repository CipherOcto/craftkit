//! The **resumable command**: a command that asks while it runs.
//!
//! What is proved here: the command stops and says what it wants, the
//! host's answer lets it go on, the whole of it is **one** undo step,
//! and the rule that keeps it safe — *a command that asks must ask before
//! it changes the document* — is enforced by the session rather than by a
//! convention.

use craftkit_derive::Command;
use craftkit_sdk::params::Doc;
use craftkit_sdk::prompt::Question;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> craftkit_sdk::Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A command that changes the document and **then** asks.
///
/// It is the case the rule is about: a command that cannot be run again
/// safely, because running it again would change the document twice.
#[derive(Command)]
#[command(id = "test.change.ask", area = "test", layer = "core", icon = "plus", mutates, undoable)]
pub struct ChangeThenAsk {
    /// What to call the thing it adds.
    pub name: craftkit_sdk::Text,
}

impl ChangeThenAsk {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        session.document_mut()?.add_node_with_under(None, self.name.0.clone(), "core.group", None, json!({}));
        let wanted = Question {
            name: "answer".to_string(),
            kind: Doc::Text,
            help: "The answer to the question.".to_string(),
        };
        let _answer = session.ask(&wanted)?;
        Ok(Outcome::changed("changed and asked"))
    }
}

/// A command that asks **twice**: the first question, then the second.
///
/// It is the case a host has to answer more than once, and the case the
/// bound on the number of questions is about.
#[derive(Command)]
#[command(id = "test.ask.twice", area = "test", layer = "core", icon = "plus", mutates, undoable)]
pub struct AskTwice {
    /// What to call the thing it adds.
    pub name: craftkit_sdk::Text,
}

impl AskTwice {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let first = session.ask(&Question {
            name: "first".to_string(),
            kind: Doc::Text,
            help: "The first answer.".to_string(),
        })?;
        let second = session.ask(&Question {
            name: "second".to_string(),
            kind: Doc::Text,
            help: "The second answer.".to_string(),
        })?;
        let payload = json!({ "first": first, "second": second });
        session.document_mut()?.add_node_with_under(None, self.name.0.clone(), "core.group", None, payload);
        Ok(Outcome::changed("asked twice"))
    }
}

/// A command that asks **for ever**: the bound has to stop it.
#[derive(Command)]
#[command(id = "test.ask.forever", area = "test", layer = "core", icon = "plus", mutates, undoable)]
pub struct AskForever {
    /// What to call the thing it adds.
    pub name: craftkit_sdk::Text,
}

impl AskForever {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        for index in 0..100 {
            let wanted = Question {
                name: format!("q{index}"),
                kind: Doc::Text,
                help: "Another question.".to_string(),
            };
            let _answer = session.ask(&wanted)?;
        }
        Ok(Outcome::changed("asked for ever"))
    }
}

/// A command that asks, and the document does not change until it has
/// the answer.
#[test]
fn a_command_that_asks_says_what_it_wants() {
    let mut session = ecosystem().session();
    let outcome = session
        .execute("doc.ask", &json!({ "question": "what is the name of the ship?" }))
        .expect("the command is waiting, not failing");
    let question = outcome.needs_input.expect("the command is waiting for something");
    assert_eq!(question.name, "answer");
    assert!(!question.help.is_empty(), "the question says what it is for: {question:?}");
    assert_eq!(outcome.data["status"], json!("needs_input"));
    assert!(!outcome.changed, "nothing changed");
    assert_eq!(session.document().nodes.len(), 0, "the document is as it was");
    assert!(session.pending().is_some(), "the session is waiting");
}

/// The host's answer lets the command go on, and the answer lands in the
/// document.
#[test]
fn the_hosts_answer_lets_the_command_go_on() {
    let mut session = ecosystem().session();
    session
        .execute("doc.ask", &json!({ "question": "what is the name of the ship?" }))
        .expect("the command is waiting");
    let outcome = session.answer(json!("the Starfighter")).expect("the answer is taken");
    assert!(outcome.changed, "the command did its work");
    assert!(outcome.needs_input.is_none(), "and it is not waiting any more");
    assert!(session.pending().is_none(), "the session is not waiting");
    assert_eq!(session.document().nodes.len(), 1, "the note is there");
    let note = &session.document().nodes[0];
    assert_eq!(note.kind, "core.note");
    assert_eq!(note.data["question"], json!("what is the name of the ship?"));
    assert_eq!(note.data["answer"], json!("the Starfighter"));
}

/// Asking and answering are **one** command, so the undo is one step.
#[test]
fn the_undo_is_one_step_for_the_whole_command() {
    let mut session = ecosystem().session();
    session.execute("doc.ask", &json!({ "question": "why?" })).expect("the command is waiting");
    session.answer(json!("because")).expect("the answer is taken");
    assert_eq!(session.document().nodes.len(), 1);
    session.undo().expect("the undo works");
    assert_eq!(session.document().nodes.len(), 0, "one undo took the note away");
}

/// A caller that **knows** the answer never asks: the answer is a
/// parameter.
#[test]
fn a_caller_that_knows_the_answer_never_asks() {
    let mut session = ecosystem().session();
    let outcome = session
        .execute("doc.ask", &json!({ "question": "why?", "answer": "because" }))
        .expect("the command runs");
    assert!(outcome.needs_input.is_none(), "the command did not ask");
    assert!(session.pending().is_none(), "and nothing is waiting");
    assert_eq!(session.document().nodes[0].data["answer"], json!("because"));
}

/// The **rule**: a command that changes the document and then asks is
/// refused, and the change is put back.
#[test]
fn a_command_that_changes_and_then_asks_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("test.change.ask", &json!({ "name": "a sneaky box" }))
        .expect_err("the command is refused");
    assert!(error.to_string().contains("must ask before it changes"), "{error}");
    assert_eq!(session.document().nodes.len(), 0, "the change was put back");
    assert!(session.pending().is_none(), "and nothing is waiting");
}

/// A command that asks **twice** is answered twice, and the answers are
/// both there.
#[test]
fn a_command_that_asks_twice_is_answered_twice() {
    let mut session = ecosystem().session();
    session.execute("test.ask.twice", &json!({ "name": "a box" })).expect("the command is waiting");
    let first = session.pending().expect("it is waiting").question().expect("a question").clone();
    assert_eq!(first.name, "first");
    session.answer(json!("one")).expect("the first answer is taken");
    let second = session.pending().expect("it is waiting").question().expect("a question").clone();
    assert_eq!(second.name, "second", "and it asks the next one");
    session.answer(json!("two")).expect("the second answer is taken");
    assert!(session.pending().is_none());
    assert_eq!(session.document().nodes[0].data["first"], json!("one"));
    assert_eq!(session.document().nodes[0].data["second"], json!("two"));
}

/// A command that asks for ever is **refused**: the bound is here so
/// that the kit notices, rather than the host.
#[test]
fn a_command_that_asks_for_ever_is_refused() {
    let mut session = ecosystem().session();
    let mut error = None;
    for _ in 0..64 {
        match session.execute("test.ask.forever", &json!({ "name": "a box" })) {
            Ok(_) => {}
            Err(failure) => {
                error = Some(failure);
                break;
            }
        }
    }
    let error = error.expect("the bound stopped it");
    assert!(error.to_string().contains("more than"), "{error}");
}

/// A pending command has changed nothing, so **another** command may run
/// and the waiting one is dropped.
#[test]
fn another_command_drops_the_waiting_one() {
    let mut session = ecosystem().session();
    session.execute("doc.ask", &json!({ "question": "why?" })).expect("the command is waiting");
    session
        .execute("node.add", &json!({ "name": "a box", "kind": "geometry.rect" }))
        .expect("the other command runs");
    assert!(session.pending().is_none(), "the waiting command was dropped");
    assert_eq!(session.document().nodes.len(), 1, "and only the other command's node is there");
}

/// Giving up on a waiting command costs nothing: it changed nothing.
#[test]
fn giving_up_costs_nothing() {
    let mut session = ecosystem().session();
    session.execute("doc.ask", &json!({ "question": "why?" })).expect("the command is waiting");
    assert!(session.cancel(), "there was something to give up");
    assert!(session.pending().is_none());
    assert_eq!(session.document().nodes.len(), 0, "and the document is as it was");
    assert!(!session.cancel(), "there is nothing to give up now");
}

/// Answering when nothing is waiting is refused.
#[test]
fn answering_when_nothing_is_waiting_is_refused() {
    let mut session = ecosystem().session();
    let error = session.answer(json!("too late")).expect_err("there is nothing waiting");
    assert!(error.to_string().contains("no command waiting"), "{error}");
}

/// The question the host is given carries what the command said about
/// it, so it can be put to a person as it is.
#[test]
fn the_question_carries_the_commands_own_words() {
    let mut session = ecosystem().session();
    let outcome = session.execute("doc.ask", &json!({ "question": "why?" })).expect("waiting");
    let question = outcome.needs_input.expect("a question");
    assert_eq!(question.to_json()["name"], json!("answer"));
    assert_eq!(question.to_json()["kind"], json!("text"));
    assert!(!question.to_json()["help"].as_str().unwrap_or_default().is_empty());
}

/// A question that is not a question is refused before anything else
/// happens.
#[test]
fn a_question_that_says_nothing_is_refused() {
    let mut session = ecosystem().session();
    let error =
        session.execute("doc.ask", &json!({ "question": "   " })).expect_err("the question says nothing");
    assert!(error.to_string().contains("has to say something"), "{error}");
    assert!(session.pending().is_none(), "and nothing is waiting");
}

/// The answer is a **text**, and an answer that is not one is refused.
#[test]
fn an_answer_that_is_not_a_text_is_refused() {
    let mut session = ecosystem().session();
    session.execute("doc.ask", &json!({ "question": "why?" })).expect("the command is waiting");
    let error = session.answer(json!(42)).expect_err("the answer is not a text");
    assert!(error.to_string().contains("the answer is a text"), "{error}");
}

/// The pending state is the session's own: it is not in the document.
#[test]
fn the_pending_state_is_not_in_the_document() {
    let mut session = ecosystem().session();
    session.execute("doc.ask", &json!({ "question": "why?" })).expect("the command is waiting");
    let pending = session.pending().expect("it is waiting");
    assert_eq!(pending.id, "doc.ask");
    assert_eq!(pending.asked.len(), 1);
    let json: Json = json!({ "question": "why?" });
    assert_eq!(pending.params, json, "the parameters it was given are kept");
}
