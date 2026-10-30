//! The **prompt seam**: a host that has what a command is missing.
//!
//! The command does not change and the document does not change: the
//! host is given a turn before the command runs. And the seam is
//! reachable from the protocol, where an agent that is told what is
//! missing can call again with it.

use craftkit_sdk::prompt::{MapSupplier, Question, Supplier};
use craftkit_sdk::serde_json::{Value as Json, json};

/// The ecosystem the tests drive.
fn ecosystem() -> craftkit_sdk::Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A supplier that answers from a map, and counts what it was asked.
struct Counting {
    /// The answers.
    answers: Json,
    /// What it was asked.
    asked: Vec<Question>,
}

impl Counting {
    /// A supplier with these answers.
    fn new(answers: Json) -> Counting {
        Counting { answers, asked: Vec::new() }
    }
}

impl Supplier for Counting {
    fn supply(&mut self, question: &Question) -> Option<Json> {
        self.asked.push(question.clone());
        self.answers.get(&question.name).cloned()
    }
}

/// A command that is missing nothing is run as it always was, and the
/// host is never asked.
#[test]
fn a_command_that_is_given_everything_asks_nothing() {
    let mut session = ecosystem().session();
    let mut supplier = Counting::new(json!({}));
    session
        .execute_with("node.add", &json!({ "name": "a box", "kind": "geometry.rect" }), &mut supplier)
        .expect("the command runs");
    assert!(supplier.asked.is_empty(), "the host was asked: {:?}", supplier.asked);
    assert_eq!(session.document().nodes.len(), 1);
}

/// A command that is missing something is **asked for it**, and the
/// answer is used.
#[test]
fn a_command_that_is_missing_something_is_asked_for_it() {
    let mut session = ecosystem().session();
    // `node.add` needs a name and a kind. The kind is given; the name is
    // not.
    let mut supplier = Counting::new(json!({ "name": "a box" }));
    session
        .execute_with("node.add", &json!({ "kind": "geometry.rect" }), &mut supplier)
        .expect("the command runs");
    assert_eq!(supplier.asked.len(), 1, "the host was asked once: {:?}", supplier.asked);
    assert_eq!(supplier.asked[0].name, "name");
    assert_eq!(supplier.asked[0].kind.json_name(), "text");
    assert_eq!(session.document().nodes.len(), 1, "and the node is there");
    assert_eq!(session.document().nodes[0].name, "a box");
}

/// The question carries what the command's own declaration says about
/// the parameter, so a host can put it to a person.
#[test]
fn the_question_carries_the_declaration() {
    let mut session = ecosystem().session();
    let mut supplier = Counting::new(json!({ "name": "a box" }));
    session
        .execute_with("node.add", &json!({ "kind": "geometry.rect" }), &mut supplier)
        .expect("the command runs");
    let question = &supplier.asked[0];
    assert!(!question.help.is_empty(), "the question says what it is for: {question:?}");
    assert_eq!(question.to_json()["name"], json!("name"));
    assert_eq!(question.to_json()["kind"], json!("text"));
}

/// A host that cannot answer leaves the command as it was: the failure
/// is the same one the caller would have got.
#[test]
fn a_host_that_cannot_answer_changes_nothing() {
    let mut session = ecosystem().session();
    let mut supplier = Counting::new(json!({}));
    let error = session
        .execute_with("node.add", &json!({ "kind": "geometry.rect" }), &mut supplier)
        .expect_err("the name is still missing");
    assert!(error.to_string().contains("`name` is required"), "{error}");
    assert_eq!(session.document().nodes.len(), 0, "nothing happened");
}

/// The **optional** parameters are not asked for: a host is asked for
/// what the command needs, not for everything it could take.
#[test]
fn the_optional_parameters_are_not_asked_for() {
    let mut session = ecosystem().session();
    let mut supplier = Counting::new(json!({}));
    session
        .execute_with("node.add", &json!({ "name": "a box", "kind": "geometry.rect" }), &mut supplier)
        .expect("the command runs");
    assert!(supplier.asked.is_empty(), "the host was asked: {:?}", supplier.asked);
}

/// A **map** of answers is a supplier, and it is the one a script or a
/// test uses.
#[test]
fn a_map_of_answers_is_a_supplier() {
    let mut session = ecosystem().session();
    let mut supplier = MapSupplier::with(json!({ "name": "a box" }));
    session
        .execute_with("node.add", &json!({ "kind": "geometry.rect" }), &mut supplier)
        .expect("the command runs");
    assert_eq!(session.document().nodes[0].name, "a box");
}
